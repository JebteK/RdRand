# rdrand-kotlin

Hardware random numbers for Kotlin and Java (JVM 11+). The client uses your
CPU's own hardware generator when it has one: `RDRAND` on Intel and AMD
x86_64, `RNDR` on ARMv8.5+ such as AWS Graviton3 and Graviton4. Otherwise (and
if the hardware ever fails to deliver) it calls the free
[rdrand.com](https://rdrand.com) API. No API key is needed. Results have the
same formats either way.

The jar bundles the hardware library for Linux, Windows and macOS on x64 and
arm64 (Apple silicon has no `RNDR`, so it uses the API). Its only dependency is
the Kotlin standard library.

## Install

Download `rdrand-kotlin-2.0.0.jar` from <https://rdrand.com/Home/Downloads> into
`libs/`, then in `build.gradle.kts`:

```kotlin
dependencies {
    implementation(files("libs/rdrand-kotlin-2.0.0.jar"))
}
```

## Use

```kotlin
import com.jebtek.rdrand.RdRand

val rng = RdRand()                    // Mode.AUTO
val key = rng.generateKey(32)         // 32 chars from 0-9 A-Z _ a-z
val bytes = rng.generateBytes(16)     // 16 random bytes
val n = rng.generateUInt()            // 0 .. Int.MAX_VALUE (2^31 - 1)

println(rng.source)                   // RDRAND, RNDR or SERVICE
println(RdRand.hardwareSource())      // the CPU's generator, or null
```

From Java: `new RdRand().generateKey(32)`, `RdRand.hardwareSource()`.

Modes: `Mode.AUTO` (hardware, else API), `Mode.HARDWARE` (throws
`RdRandException` without a hardware generator), `Mode.SERVICE` (API only).
`RdRand(mode, baseUrl, timeout)` also takes another API base URL. Calls block;
hardware calls take microseconds.

License: MIT, Copyright © 2014-2026 JebteK, LLC.
