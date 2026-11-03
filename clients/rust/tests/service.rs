//! Calls a live API. Runs only when RDRAND_TEST_URL is set, e.g.
//! `RDRAND_TEST_URL=https://rdrand.com/API/ cargo test`.

use rdrand_client::{Client, Mode, KEY_ALPHABET};

fn client() -> Option<Client> {
    let url = std::env::var("RDRAND_TEST_URL").ok()?;
    Some(Client::with_mode(Mode::Service).base_url(url))
}

#[test]
fn service_mode_formats() {
    let Some(rng) = client() else {
        eprintln!("RDRAND_TEST_URL not set; skipped");
        return;
    };
    let k = rng.generate_key(100).unwrap();
    assert_eq!(k.len(), 100);
    assert!(k.bytes().all(|c| KEY_ALPHABET.contains(&c)));
    assert!(rng.generate_key(0).unwrap().is_empty());
    assert_eq!(rng.generate_bytes(1000).unwrap().len(), 1000);
    assert!(rng.generate_bytes(0).unwrap().is_empty());
    assert!(rng.generate_uint().unwrap() < 1 << 31);
}

#[test]
fn service_errors_surface() {
    let Some(_) = client() else { return };
    let rng = Client::with_mode(Mode::Service).base_url("https://rdrand.com/API/Nope/");
    assert!(rng.generate_uint().is_err());
}
