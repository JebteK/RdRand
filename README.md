RdRand
======

Hardware random numbers from your CPU's own generator, with the free
[rdrand.com](https://rdrand.com) API as a fallback.

This repository holds the open source rdrand.com clients for **Node.js**,
**Kotlin/Java**, **Rust** and **.NET**. Each one uses the CPU's hardware random
number generator when it has one:

- `RDRAND` on x86_64 (Intel and AMD; Linux, Windows, macOS)
- `RNDR` on ARMv8.5+ with the RNG extension (Linux arm64, e.g. AWS Graviton3 and Graviton4)

Anywhere else, or if the hardware ever fails to deliver mid-call, the clients call
the rdrand.com API. Results have the API's formats whichever source produced them,
and every client reports which source it used (`rdrand`, `rndr` or `service`).

It also keeps the original 2014 .NET library ([legacy, deprecated](#legacy-net-library-v1x-deprecated)).

Clients (v2.0.0)
------

| Client | Source | Install a release | Hardware path ships for |
|---|---|---|---|
| Node.js 18+ | [`clients/node`](clients/node) | `npm install https://rdrand.com/Downloads/jebtek-rdrand-2.0.0.tgz` | linux-x64, linux-arm64, win32-x64, darwin-x64, darwin-arm64 |
| Kotlin / Java (JVM 11+) | [`clients/kotlin`](clients/kotlin) | [rdrand-kotlin-2.0.0.jar](https://rdrand.com/Downloads/rdrand-kotlin-2.0.0.jar) | same five platforms, inside the jar |
| Rust | [`clients/rust`](clients/rust) | `rdrand-client = { git = "https://github.com/JebteK/RdRand" }` | any x86_64 or aarch64 target (compiled in) |
| .NET Standard 2.0 | [`clients/dotnet`](clients/dotnet) | [JebteK.RdRand.2.0.0.nupkg](https://rdrand.com/Downloads/JebteK.RdRand.2.0.0.nupkg) | linux-x64, linux-arm64, win-x64, osx-x64, osx-arm64 |

All four have the same shape:

```js
// Node.js
const { RdRand } = require("@jebtek/rdrand");
const rng = new RdRand();                  // mode "auto" | "hardware" | "service"
await rng.generateKey(32);                 // 32 chars, each uniform over 0-9 A-Z _ a-z
await rng.generateBytes(16);               // 16 random bytes
await rng.generateUInt();                  // 0 to 2^31 - 1
rng.source;                                // "rdrand", "rndr" or "service"
```

```kotlin
val rng = com.jebtek.rdrand.RdRand()      // Kotlin / Java
rng.generateKey(32); rng.generateBytes(16); rng.generateUInt(); rng.source
```

```rust
let rng = rdrand_client::Client::new();    // Rust
rng.generate_key(32)?; rng.generate_bytes(16)?; rng.generate_uint()?; rng.source();
```

```csharp
using var rng = new JebteK.RdRand.RdRandClient();   // .NET
await rng.GenerateKeyAsync(32); await rng.GenerateBytesAsync(16); await rng.GenerateUIntAsync(); _ = rng.Source;
```

Each client's README has the details: [Node.js](clients/node/README.md),
[Kotlin/Java](clients/kotlin/README.md), [Rust](clients/rust/README.md),
[.NET](clients/dotnet/README.md).

### How the hardware path works

- **Rust** executes the instructions directly: `RDRAND` through the
  `_rdrand64_step` intrinsic, `RNDR` through `mrs` on its system register. Both
  are detected at run time.
- **Node.js, Kotlin and .NET** share one small native library,
  [`clients/native`](clients/native), built from that Rust code for each platform.
  It exports a C ABI (used by .NET P/Invoke), JNI functions (used by Kotlin) and a
  Node-API module (used by Node.js). It looks Node's functions up at load time, so
  the same file loads cleanly into any of the three.
- Every 64-bit draw's success flag (CF for RDRAND, NZCV for RNDR) is checked. A
  failed draw is retried up to 10 times, and two identical consecutive values are
  rejected, which catches generators stuck on one value. The generator must pass a
  self-test of 64 distinct values before it is used.
- Keys are drawn by rejection sampling, so each of the 63 characters is exactly
  equally likely. Integers are masked to 31 bits to match the API.

Release 2.0.0 was tested on real hardware with
[`scripts/test-clients-remote.sh`](scripts/test-clients-remote.sh): an Intel Xeon
(`rdrand`), AWS Graviton3 (`rndr`) and Graviton2 (no RNG, so the API fallback).
The Windows and macOS x64 libraries are built and their exports checked, but they
have not been run.

The rdrand.com API
------

The clients' fallback, also usable directly from any language. It is free, with no
sign-up and no API key.

| Endpoint | Parameter | Response body (plain text) |
|---|---|---|
| `POST https://rdrand.com/API/GenerateKey` | `length` (0 to 16777216) | `length` characters from `0-9 A-Z _ a-z` |
| `POST https://rdrand.com/API/GenerateBytes` | `length` (0 to 16777216) | `length` random bytes, base64 |
| `POST https://rdrand.com/API/GenerateUInt` | none | an integer from 0 to 2147483647 |

```sh
curl -d length=32 https://rdrand.com/API/GenerateKey
curl -d length=16 https://rdrand.com/API/GenerateBytes | base64 --decode > random.bin
curl -d "" https://rdrand.com/API/GenerateUInt
```

Send `length` as a form field, a JSON body (`{"length": 32}`) or a query parameter.
CORS is open to any origin. Full documentation:
[llms-full.txt](https://rdrand.com/llms-full.txt) (Markdown),
[openapi.json](https://rdrand.com/openapi.json) (OpenAPI 3.1) and
[llms.txt](https://rdrand.com/llms.txt).

Building
------

`scripts/build-clients.sh` builds every client into `dist/` (the same artifacts as
the [downloads page](https://rdrand.com/Home/Downloads)). It also assembles
self-tests in `dist/client-tests/` for running on other machines. It expects a
macOS host with:

- `rustup` targets `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`,
  `x86_64-pc-windows-gnu`, `x86_64-apple-darwin` and `aarch64-apple-darwin`
- [`zig`](https://ziglang.org) and [`cargo-zigbuild`](https://github.com/rust-cross/cargo-zigbuild)
- Node.js and npm
- Docker, which runs Gradle (`gradle:8.10.2-jdk17`) and the .NET SDK (`mcr.microsoft.com/dotnet/sdk:8.0`)

`scripts/build-native.sh` alone builds the native library: Linux at a glibc 2.17
baseline, Windows through Zig/MinGW, and macOS natively.

Testing:

```sh
cd clients/rust && RDRAND_TEST_URL=https://rdrand.com/API/ cargo test
cd clients/node && RDRAND_TEST_URL=https://rdrand.com/API/ node --test     # after build-clients.sh (prebuilds/)
java -jar dist/client-tests/rdrand-kotlin-selftest-2.0.0.jar
scripts/test-clients-remote.sh ubuntu@HOST https://rdrand.com/API/         # all clients on a real x64/arm64 box
```

Legacy .NET library (v1.x, deprecated)
------

`Jebtek.RdRandLib` (C++/CLI and MASM) executes `rdrand`, and `Jebtek.RdRand` wraps it
in the `RdRandom` class (`GenerateKey`, `GenerateBytes`, `GenerateUnsignedInt`,
`GenerateAPIKey`). The release builds are in `Versions/`; they need .NET Framework on
Windows and the
[Visual C++ 2012 Redistributable](http://www.microsoft.com/en-us/download/details.aspx?id=30679).

**Use the 2.0 .NET client ([`clients/dotnet`](clients/dotnet)) instead.** The
legacy library has known defects:

1. **Failed reads are returned as random numbers.** `rdrand.asm` branches on ZF
   (`jnz success`), but RDRAND reports success in CF and always clears ZF. When the
   generator is momentarily exhausted, `Random32()` returns `0` as if it were
   random. There is also no retry.
2. **Weak fallback.** On CPUs without RDRAND, `RdRandom.GenerateKey` silently
   falls back to `System.Random` (seeded from the clock, not cryptographic), while
   `GenerateBytes` returns `null`.
3. **Windows and .NET Framework only.** It assumes Intel "Ivy Bridge or newer"; AMD
   CPUs have had RDRAND since 2015 too.

The 2.0 client checks the carry flag, retries, rejects stuck values, never falls
back to a non-cryptographic generator, and runs on Linux, Windows and macOS on x64
and arm64.

License (MIT)
======

Copyright © 2014-2026 JebteK, LLC

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
