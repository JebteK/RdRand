//! The CPU's hardware random number generator: `RDRAND` on x86_64 (Intel and
//! AMD), `RNDR` on 64-bit ARM (ARMv8.5 `FEAT_RNG`, e.g. AWS Graviton3 and
//! later; not Graviton2 or Apple silicon).
//!
//! Each 64-bit draw is checked for the instruction's success flag and retried
//! up to [`RETRIES`] times (Intel's guidance for a momentarily exhausted
//! generator). Two identical consecutive values are treated as a failed draw,
//! which catches generators stuck on one value (such as AMD's all-ones
//! erratum). The generator is only reported as available after a startup
//! self-test of 64 distinct values.

use std::cell::Cell;
use std::sync::OnceLock;

use crate::Source;

/// Attempts per 64-bit value before giving up.
pub const RETRIES: usize = 10;

/// The hardware generator failed to deliver after [`RETRIES`] attempts, or
/// this CPU has none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwError {
    Unavailable,
    Failure,
}

/// The hardware generator on this CPU, if present and passing the self-test.
/// Detection runs once per process.
pub fn source() -> Option<Source> {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    // Short-circuit: never execute the instruction unless the CPU reports it.
    let ok = *AVAILABLE.get_or_init(|| imp::detected() && self_test());
    ok.then_some(imp::SOURCE)
}

/// Fill `buf` from the hardware generator.
pub fn fill(buf: &mut [u8]) -> Result<(), HwError> {
    if source().is_none() {
        return Err(HwError::Unavailable);
    }
    for chunk in buf.chunks_mut(8) {
        let v = next_u64().ok_or(HwError::Failure)?;
        chunk.copy_from_slice(&v.to_le_bytes()[..chunk.len()]);
    }
    Ok(())
}

fn self_test() -> bool {
    let mut seen = Vec::with_capacity(64);
    for _ in 0..64 {
        match next_u64() {
            Some(v) if !seen.contains(&v) => seen.push(v),
            _ => return false,
        }
    }
    true
}

thread_local! {
    static LAST: Cell<u64> = const { Cell::new(0) };
}

/// Only call once `imp::detected()` is true.
fn next_u64() -> Option<u64> {
    LAST.with(|last| {
        for _ in 0..RETRIES {
            if let Some(v) = imp::draw() {
                if v != last.get() {
                    last.set(v);
                    return Some(v);
                }
            }
        }
        None
    })
}

#[cfg(target_arch = "x86_64")]
mod imp {
    pub const SOURCE: crate::Source = crate::Source::Rdrand;

    pub fn detected() -> bool {
        std::arch::is_x86_feature_detected!("rdrand")
    }

    #[target_feature(enable = "rdrand")]
    unsafe fn step() -> Option<u64> {
        let mut v = 0u64;
        // RDRAND reports success in CF, which the intrinsic returns as 1.
        (core::arch::x86_64::_rdrand64_step(&mut v) == 1).then_some(v)
    }

    pub fn draw() -> Option<u64> {
        // SAFETY: only reached after `detected()` confirmed RDRAND.
        unsafe { step() }
    }
}

#[cfg(target_arch = "aarch64")]
mod imp {
    pub const SOURCE: crate::Source = crate::Source::Rndr;

    pub fn detected() -> bool {
        std::arch::is_aarch64_feature_detected!("rand")
    }

    pub fn draw() -> Option<u64> {
        let v: u64;
        let ok: u64;
        // SAFETY: only reached after `detected()` confirmed FEAT_RNG. RNDR is
        // read through its system register encoding (s3_3_c2_c4_0) so no
        // assembler support for the name is needed. It sets NZCV to 0b0000 on
        // success and 0b0100 (Z set, value 0) on failure.
        unsafe {
            core::arch::asm!(
                "mrs {v}, s3_3_c2_c4_0",
                "cset {ok}, ne",
                v = out(reg) v,
                ok = out(reg) ok,
                options(nomem, nostack),
            );
        }
        (ok == 1).then_some(v)
    }
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
mod imp {
    pub const SOURCE: crate::Source = crate::Source::Rdrand;

    pub fn detected() -> bool {
        false
    }

    pub fn draw() -> Option<u64> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_when_available() {
        let Some(src) = source() else {
            assert_eq!(fill(&mut [0u8; 8]), Err(HwError::Unavailable));
            eprintln!("no hardware generator on this CPU; hardware tests skipped");
            return;
        };
        eprintln!("hardware generator: {}", src.as_str());
        for len in [0usize, 1, 7, 8, 9, 63, 4096] {
            let mut buf = vec![0u8; len];
            fill(&mut buf).unwrap();
        }
        // 1 MiB: no zero 32-bit words (the legacy defect), bytes cover 0..=255.
        let mut buf = vec![0u8; 1 << 20];
        fill(&mut buf).unwrap();
        assert!(buf.chunks(4).all(|w| w != [0, 0, 0, 0]));
        let mut seen = [false; 256];
        buf.iter().for_each(|&b| seen[b as usize] = true);
        assert!(seen.iter().all(|&s| s));
    }
}
