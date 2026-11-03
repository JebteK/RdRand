# JebteK.RdRand

Hardware random numbers for .NET. The client uses your CPU's own hardware
generator when it has one: `RDRAND` on Intel and AMD x86_64, `RNDR` on
ARMv8.5+ such as AWS Graviton3 and Graviton4. Otherwise (and if the hardware
ever fails to deliver) it calls the free [rdrand.com](https://rdrand.com) API.
No API key is needed. Results have the same formats either way.

Targets .NET Standard 2.0: .NET 6/8/9+, .NET Core 2.0+, .NET Framework 4.6.1+.
The hardware path is available on linux-x64, linux-arm64, win-x64, osx-x64 and
osx-arm64 (Apple silicon has no `RNDR`, so it uses the API).

## Install

Download `JebteK.RdRand.2.0.0.nupkg` from <https://rdrand.com/Home/Downloads>
into a folder, then:

```sh
dotnet add package JebteK.RdRand --version 2.0.0 --source /path/to/folder
```

## Use

```csharp
using JebteK.RdRand;

using var rng = new RdRandClient();             // RdRandMode.Auto
string key   = await rng.GenerateKeyAsync(32);  // 32 chars from 0-9 A-Z _ a-z
byte[] bytes = await rng.GenerateBytesAsync(16);
uint n       = await rng.GenerateUIntAsync();   // 0 .. 2^31 - 1

Console.WriteLine(rng.Source);                  // Rdrand, Rndr or Service
Console.WriteLine(RdRandClient.HardwareSource); // the CPU's generator, or null
```

Blocking versions: `GenerateKey`, `GenerateBytes`, `GenerateUInt`.

Modes: `RdRandMode.Auto` (hardware, else API), `RdRandMode.Hardware` (throws
`RdRandException` without a hardware generator), `RdRandMode.Service` (API only).
`new RdRandClient(mode, baseUrl, httpClient)` also takes another API base URL
and your own `HttpClient`.

License: MIT, Copyright © 2014-2026 JebteK, LLC.
