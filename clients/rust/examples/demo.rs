//! `cargo run --example demo` prints which source this machine uses and a
//! sample of each call, from the hardware (or the API fallback) and from the API.

use rdrand_client::{Client, Mode};

fn main() -> Result<(), rdrand_client::Error> {
    println!(
        "hardware generator: {}",
        Client::hardware_source().map_or("none", |s| s.as_str())
    );
    for mode in [Mode::Auto, Mode::Service] {
        let rng = Client::with_mode(mode);
        println!("{mode:?} (source: {})", rng.source().as_str());
        println!("  generate_key(32)   = {}", rng.generate_key(32)?);
        println!("  generate_bytes(16) = {:02x?}", rng.generate_bytes(16)?);
        println!("  generate_uint()    = {}", rng.generate_uint()?);
    }
    Ok(())
}
