//! Client for [rdrand.com](https://rdrand.com): hardware random numbers.
//!
//! Randomness comes from the local CPU's hardware generator when it has one
//! (`RDRAND` on Intel/AMD x86_64, `RNDR` on ARMv8.5+ such as AWS Graviton3),
//! and from the rdrand.com API otherwise. Results have the same formats as the
//! API whichever source produced them.
//!
//! ```no_run
//! let rng = rdrand_client::Client::new();
//! let key = rng.generate_key(32)?;      // 32 chars from 0-9 A-Z _ a-z
//! let bytes = rng.generate_bytes(16)?;  // 16 random bytes
//! let n = rng.generate_uint()?;         // 0 ..= 2^31 - 1
//! println!("{key} {bytes:?} {n} (from {})", rng.source().as_str());
//! # Ok::<(), rdrand_client::Error>(())
//! ```
//!
//! The client is blocking. From async code, call it inside
//! `tokio::task::spawn_blocking` (hardware calls take microseconds, so a
//! [`Mode::Hardware`] client can also be called directly).

pub mod hw;
#[cfg(feature = "service")]
mod service;
#[cfg(not(feature = "service"))]
mod service {
    //! Built without the `service` feature: hardware only.
    pub(crate) struct Service;
    impl Service {
        pub fn new(_base: &str) -> Self {
            Service
        }
        pub fn key(&self, _: usize) -> Result<String, crate::Error> {
            Err(crate::Error::HardwareUnavailable)
        }
        pub fn bytes(&self, _: usize) -> Result<Vec<u8>, crate::Error> {
            Err(crate::Error::HardwareUnavailable)
        }
        pub fn uint(&self) -> Result<u32, crate::Error> {
            Err(crate::Error::HardwareUnavailable)
        }
    }
}

use std::fmt;

/// The API's base URL; the action name (`GenerateKey`, ...) is appended.
pub const DEFAULT_BASE_URL: &str = "https://rdrand.com/API/";

/// The 63 characters `GenerateKey` draws from, each equally likely.
pub const KEY_ALPHABET: &[u8; 63] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxyz";

/// Where randomness may come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// The CPU's generator if it has one, else the rdrand.com API. A hardware
    /// failure mid-call also falls back to the API.
    #[default]
    Auto,
    /// Only the CPU's generator; fail if there is none.
    Hardware,
    /// Only the rdrand.com API.
    Service,
}

/// Where a result came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// x86_64 `RDRAND` (Intel, AMD).
    Rdrand,
    /// ARMv8.5 `RNDR` (e.g. AWS Graviton3/4).
    Rndr,
    /// The rdrand.com API.
    Service,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Rdrand => "rdrand",
            Source::Rndr => "rndr",
            Source::Service => "service",
        }
    }
}

#[derive(Debug)]
pub enum Error {
    /// [`Mode::Hardware`] on a CPU without a working hardware generator.
    HardwareUnavailable,
    /// The hardware generator stopped delivering (retries exhausted), and no
    /// fallback was allowed.
    HardwareFailure,
    /// The API call failed or returned something malformed.
    Service(String),
    /// A length the API cannot represent (over `i32::MAX`).
    InvalidLength(usize),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::HardwareUnavailable => f.write_str("this CPU has no working hardware random number generator"),
            Error::HardwareFailure => f.write_str("the hardware random number generator failed to deliver"),
            Error::Service(e) => write!(f, "rdrand.com API: {e}"),
            Error::InvalidLength(n) => write!(f, "length {n} is too large"),
        }
    }
}

impl std::error::Error for Error {}

/// A random number client. Cheap to create; reuse it to keep the API's HTTP
/// connection alive.
pub struct Client {
    mode: Mode,
    service: service::Service,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    /// [`Mode::Auto`] against [`DEFAULT_BASE_URL`].
    pub fn new() -> Self {
        Self::with_mode(Mode::Auto)
    }

    pub fn with_mode(mode: Mode) -> Self {
        Client {
            mode,
            service: service::Service::new(DEFAULT_BASE_URL),
        }
    }

    /// Use another deployment of the API (e.g. a private instance).
    #[cfg(feature = "service")]
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.service = service::Service::new(&url.into());
        self
    }

    /// The CPU's hardware generator, if it has a working one.
    pub fn hardware_source() -> Option<Source> {
        hw::source()
    }

    /// Where this client's results come from (barring a hardware failure in
    /// [`Mode::Auto`], which falls back to the API).
    pub fn source(&self) -> Source {
        match (self.mode, hw::source()) {
            (Mode::Service, _) => Source::Service,
            (_, Some(s)) => s,
            (_, None) => Source::Service,
        }
    }

    /// `length` characters, each uniform over [`KEY_ALPHABET`].
    pub fn generate_key(&self, length: usize) -> Result<String, Error> {
        check_length(length)?;
        self.with_fallback(|| key_from(length, hw::fill), |s| s.key(length))
    }

    /// `length` random bytes.
    pub fn generate_bytes(&self, length: usize) -> Result<Vec<u8>, Error> {
        check_length(length)?;
        self.with_fallback(
            || {
                let mut buf = vec![0u8; length];
                hw::fill(&mut buf).map(|()| buf)
            },
            |s| s.bytes(length),
        )
    }

    /// A random integer in `0 ..= 2^31 - 1` (the API's range, so it fits a
    /// signed 32-bit integer too).
    pub fn generate_uint(&self) -> Result<u32, Error> {
        self.with_fallback(
            || {
                let mut b = [0u8; 4];
                hw::fill(&mut b).map(|()| uint31(b))
            },
            |s| s.uint(),
        )
    }

    fn with_fallback<T>(
        &self,
        hardware: impl FnOnce() -> Result<T, hw::HwError>,
        remote: impl FnOnce(&service::Service) -> Result<T, Error>,
    ) -> Result<T, Error> {
        if self.mode != Mode::Service {
            match hardware() {
                Ok(v) => return Ok(v),
                Err(e) if self.mode == Mode::Hardware => {
                    return Err(match e {
                        hw::HwError::Unavailable => Error::HardwareUnavailable,
                        hw::HwError::Failure => Error::HardwareFailure,
                    });
                }
                Err(_) => {}
            }
        }
        remote(&self.service)
    }
}

fn check_length(length: usize) -> Result<(), Error> {
    if length > i32::MAX as usize {
        return Err(Error::InvalidLength(length));
    }
    Ok(())
}

/// Uniform over the 63-character alphabet: bytes 252..=255 are rejected so
/// every character gets exactly 4 of the 252 accepted byte values.
fn key_from(length: usize, mut fill: impl FnMut(&mut [u8]) -> Result<(), hw::HwError>) -> Result<String, hw::HwError> {
    let mut out = Vec::with_capacity(length);
    let mut scratch = [0u8; 512];
    while out.len() < length {
        let want = (length - out.len()).min(scratch.len() - 8);
        // ~1.6% of bytes are rejected; a few spare bytes usually cover them.
        let n = (want + want / 32 + 8).min(scratch.len());
        fill(&mut scratch[..n])?;
        for &b in &scratch[..n] {
            if b < 252 && out.len() < length {
                out.push(KEY_ALPHABET[(b % 63) as usize]);
            }
        }
    }
    Ok(String::from_utf8(out).expect("alphabet is ASCII"))
}

fn uint31(b: [u8; 4]) -> u32 {
    u32::from_le_bytes(b) & 0x7fff_ffff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_mapping_is_exactly_uniform() {
        let mut counts = [0usize; 63];
        for b in 0u8..=251 {
            counts[(b % 63) as usize] += 1;
        }
        assert!(counts.iter().all(|&c| c == 4));
    }

    #[test]
    fn key_from_counter_bytes() {
        // A deterministic "generator" exercises rejection and chunking.
        let mut next = 0u8;
        let fill = |buf: &mut [u8]| {
            for b in buf {
                *b = next;
                next = next.wrapping_add(1);
            }
            Ok(())
        };
        let k = key_from(2000, fill).unwrap();
        assert_eq!(k.len(), 2000);
        assert!(k.bytes().all(|c| KEY_ALPHABET.contains(&c)));
        assert!(key_from(0, |_| Ok(())).unwrap().is_empty());
    }

    #[test]
    fn uint31_masks_the_sign_bit() {
        assert_eq!(uint31([0xff; 4]), 0x7fff_ffff);
        assert_eq!(uint31([1, 0, 0, 0x80]), 1);
    }

    #[test]
    fn hardware_mode_matches_detection() {
        let rng = Client::with_mode(Mode::Hardware);
        match Client::hardware_source() {
            Some(src) => {
                assert_eq!(rng.source(), src);
                assert_eq!(rng.generate_key(40).unwrap().len(), 40);
                assert_eq!(rng.generate_bytes(33).unwrap().len(), 33);
                assert!(rng.generate_uint().unwrap() < 1 << 31);
            }
            None => {
                assert!(matches!(rng.generate_key(4), Err(Error::HardwareUnavailable)));
                assert!(matches!(rng.generate_uint(), Err(Error::HardwareUnavailable)));
            }
        }
    }

    #[test]
    fn rejects_lengths_the_api_cannot_take() {
        let rng = Client::new();
        assert!(matches!(
            rng.generate_bytes(i32::MAX as usize + 1),
            Err(Error::InvalidLength(_))
        ));
    }
}
