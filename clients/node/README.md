# @jebtek/rdrand

Hardware random numbers for Node.js. The client uses your CPU's own hardware
generator when it has one: `RDRAND` on Intel and AMD x86_64, `RNDR` on
ARMv8.5+ such as AWS Graviton3 and Graviton4. Otherwise (and if the hardware
ever fails to deliver) it calls the free [rdrand.com](https://rdrand.com) API.
No API key is needed. Results have the same formats either way.

Node.js 18 or later, no dependencies. The hardware path ships prebuilt for
linux-x64, linux-arm64, win32-x64, darwin-x64 and darwin-arm64 (Apple silicon
has no `RNDR`, so it uses the API).

## Install

```sh
npm install https://rdrand.com/Downloads/jebtek-rdrand-2.0.0.tgz
```

## Use

```js
const { RdRand } = require('@jebtek/rdrand');   // or: import { RdRand } from '@jebtek/rdrand'

const rng = new RdRand();                  // mode 'auto'
const key = await rng.generateKey(32);     // 32 chars from 0-9 A-Z _ a-z
const bytes = await rng.generateBytes(16); // Buffer of 16 random bytes
const n = await rng.generateUInt();        // 0 .. 2^31 - 1

console.log(rng.source);                   // 'rdrand', 'rndr' or 'service'
console.log(RdRand.hardwareSource());      // the CPU's generator, or null
```

Options: `new RdRand({ mode, baseUrl, timeoutMs })` with `mode` one of
`'auto'` (hardware, else API), `'hardware'` (rejects with `RdRandError`
without a hardware generator) or `'service'` (API only).

License: MIT, Copyright © 2014-2026 JebteK, LLC.
