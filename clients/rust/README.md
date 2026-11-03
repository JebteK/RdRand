# rdrand-client

Hardware random numbers for Rust. The client uses your CPU's own hardware
generator when it has one: `RDRAND` on Intel and AMD x86_64, `RNDR` on
ARMv8.5+ such as AWS Graviton3 and Graviton4 (Apple silicon has no `RNDR`).
Otherwise (and if the hardware ever fails to deliver) it calls the free
[rdrand.com](https://rdrand.com) API. No API key is needed. Results have the
same formats either way.

## Install

From GitHub (Cargo finds the crate inside the repository):

```toml
[dependencies]
rdrand-client = { git = "https://github.com/JebteK/RdRand" }
```

Or download `rdrand-rust-2.0.0.zip` from <https://rdrand.com/Home/Downloads>,
unzip it next to your project, then in `Cargo.toml`:

```toml
[dependencies]
rdrand-client = { path = "../rdrand-client-2.0.0" }
# Hardware only, no HTTP dependencies:
# rdrand-client = { path = "../rdrand-client-2.0.0", default-features = false }
```

## Use

```rust
use rdrand_client::{Client, Mode};

let rng = Client::new();                 // Mode::Auto
let key = rng.generate_key(32)?;         // 32 chars from 0-9 A-Z _ a-z
let bytes = rng.generate_bytes(16)?;     // Vec<u8>
let n = rng.generate_uint()?;            // 0 ..= 2^31 - 1

println!("{}", rng.source().as_str());   // "rdrand", "rndr" or "service"
println!("{:?}", Client::hardware_source());
```

Modes: `Mode::Auto` (hardware, else API), `Mode::Hardware` (errors without a
hardware generator), `Mode::Service` (API only). `.base_url(..)` points the
client at another API deployment. Calls block; from async code use
`spawn_blocking` (or call a `Mode::Hardware` client directly, it takes
microseconds). `cargo run --example demo` shows what this machine uses.

The `service` feature (default) adds the API fallback via `ureq`; without it
the crate has no dependencies.

License: MIT, Copyright © 2014-2026 JebteK, LLC.
