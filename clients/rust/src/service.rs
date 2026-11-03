//! The rdrand.com API: `POST {base}/GenerateKey|GenerateBytes|GenerateUInt`
//! with a form-encoded `length`. No API key is needed.

use std::time::Duration;

use base64::Engine;

use crate::{Error, KEY_ALPHABET};

/// Bodies larger than this are refused (16 MiB of key is ~16 MiB of text;
/// 16 MiB of bytes is ~22 MiB of base64).
const MAX_BODY: u64 = 64 * 1024 * 1024;

pub(crate) struct Service {
    base: String,
    agent: ureq::Agent,
}

impl Service {
    pub fn new(base: &str) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(60)))
            .user_agent(concat!("rdrand-client-rust/", env!("CARGO_PKG_VERSION")))
            .build()
            .into();
        Service {
            base: base.trim_end_matches('/').to_owned(),
            agent,
        }
    }

    pub fn key(&self, length: usize) -> Result<String, Error> {
        let s = self.post("GenerateKey", Some(length))?;
        if s.len() != length || !s.bytes().all(|c| KEY_ALPHABET.contains(&c)) {
            return Err(Error::Service(format!("malformed key ({} chars)", s.len())));
        }
        Ok(s)
    }

    pub fn bytes(&self, length: usize) -> Result<Vec<u8>, Error> {
        let s = self.post("GenerateBytes", Some(length))?;
        let raw = base64::engine::general_purpose::STANDARD
            .decode(s.trim())
            .map_err(|e| Error::Service(format!("invalid base64: {e}")))?;
        if raw.len() != length {
            return Err(Error::Service(format!("expected {length} bytes, got {}", raw.len())));
        }
        Ok(raw)
    }

    pub fn uint(&self) -> Result<u32, Error> {
        let s = self.post("GenerateUInt", None)?;
        s.trim()
            .parse()
            .map_err(|_| Error::Service(format!("not an integer: {:?}", s.chars().take(40).collect::<String>())))
    }

    fn post(&self, action: &str, length: Option<usize>) -> Result<String, Error> {
        let url = format!("{}/{action}", self.base);
        let req = self.agent.post(&url);
        let res = match length {
            Some(n) => req.send_form([("length", n.to_string().as_str())]),
            None => req.send_empty(),
        };
        let mut res = res.map_err(|e| Error::Service(e.to_string()))?;
        res.body_mut()
            .with_config()
            .limit(MAX_BODY)
            .read_to_string()
            .map_err(|e| Error::Service(e.to_string()))
    }
}
