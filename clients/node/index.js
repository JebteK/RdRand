'use strict';
// rdrand.com client: hardware random numbers from the local CPU (RDRAND on
// Intel/AMD, RNDR on ARMv8.5+ such as AWS Graviton3), falling back to the
// rdrand.com API. Results have the API's formats whichever source made them.

const path = require('path');

const DEFAULT_BASE_URL = 'https://rdrand.com/API/';
const KEY_ALPHABET = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxyz';
const MAX_LENGTH = 2147483647;
const VERSION = '2.0.0';

const SOURCES = { 1: 'rdrand', 2: 'rndr' };
const NATIVE_FILES = { win32: 'rdrand_native.dll', darwin: 'librdrand_native.dylib' };

class RdRandError extends Error {
  constructor(message, options) {
    super(message, options);
    this.name = 'RdRandError';
  }
}

let native; // undefined: not loaded yet; null: unavailable
function loadNative() {
  if (native !== undefined) return native;
  native = null;
  const file = path.join(
    __dirname,
    'prebuilds',
    `${process.platform}-${process.arch}`,
    NATIVE_FILES[process.platform] || 'librdrand_native.so',
  );
  try {
    const mod = { exports: {} };
    process.dlopen(mod, file);
    if (typeof mod.exports.fill === 'function') native = mod.exports;
  } catch {
    // No prebuilt library for this platform: the API is used instead.
  }
  return native;
}

/** The CPU's hardware generator ('rdrand' or 'rndr'), or null if it has none. */
function hardwareSource() {
  const n = loadNative();
  return (n && SOURCES[n.source()]) || null;
}

/** Fill a Buffer from the hardware generator. Returns false if it cannot. */
function hardwareFill(buf) {
  const n = loadNative();
  return !!n && n.fill(buf) === 0;
}

function checkLength(length) {
  if (!Number.isInteger(length) || length < 0 || length > MAX_LENGTH) {
    throw new RangeError(`length must be an integer from 0 to ${MAX_LENGTH}`);
  }
}

/** Uniform over the 63-character alphabet: bytes 252-255 are rejected. */
function keyFromHardware(length) {
  const out = [];
  while (out.length < length) {
    const want = length - out.length;
    const buf = Buffer.allocUnsafe(Math.min(want + (want >> 5) + 8, 65536));
    if (!hardwareFill(buf)) return null;
    for (let i = 0; i < buf.length && out.length < length; i++) {
      if (buf[i] < 252) out.push(KEY_ALPHABET[buf[i] % 63]);
    }
  }
  return out.join('');
}

class RdRand {
  /**
   * @param {object} [options]
   * @param {'auto'|'hardware'|'service'} [options.mode='auto']
   * @param {string} [options.baseUrl='https://rdrand.com/API/']
   * @param {number} [options.timeoutMs=60000]
   */
  constructor({ mode = 'auto', baseUrl = DEFAULT_BASE_URL, timeoutMs = 60000 } = {}) {
    if (!['auto', 'hardware', 'service'].includes(mode)) {
      throw new RangeError(`mode must be 'auto', 'hardware' or 'service'`);
    }
    this.mode = mode;
    this.baseUrl = baseUrl.replace(/\/+$/, '');
    this.timeoutMs = timeoutMs;
  }

  static hardwareSource() {
    return hardwareSource();
  }

  /** Where results come from: 'rdrand', 'rndr' or 'service'. */
  get source() {
    return (this.mode !== 'service' && hardwareSource()) || 'service';
  }

  /** `length` characters, each uniform over 0-9 A-Z _ a-z. */
  async generateKey(length) {
    checkLength(length);
    return this.#withFallback(
      () => keyFromHardware(length),
      async () => {
        const s = await this.#post('GenerateKey', length);
        if (s.length !== length || ![...s].every((c) => KEY_ALPHABET.includes(c))) {
          throw new RdRandError(`malformed key from the API (${s.length} chars)`);
        }
        return s;
      },
    );
  }

  /** `length` random bytes, as a Buffer. */
  async generateBytes(length) {
    checkLength(length);
    return this.#withFallback(
      () => {
        const buf = Buffer.allocUnsafe(length);
        return hardwareFill(buf) ? buf : null;
      },
      async () => {
        const buf = Buffer.from(await this.#post('GenerateBytes', length), 'base64');
        if (buf.length !== length) {
          throw new RdRandError(`expected ${length} bytes from the API, got ${buf.length}`);
        }
        return buf;
      },
    );
  }

  /** A random integer from 0 to 2^31 - 1. */
  async generateUInt() {
    return this.#withFallback(
      () => {
        const buf = Buffer.allocUnsafe(4);
        return hardwareFill(buf) ? buf.readUInt32LE(0) & 0x7fffffff : null;
      },
      async () => {
        const s = (await this.#post('GenerateUInt')).trim();
        const n = Number(s);
        if (!/^\d+$/.test(s) || n > 0xffffffff) {
          throw new RdRandError(`not an integer from the API: ${JSON.stringify(s.slice(0, 40))}`);
        }
        return n;
      },
    );
  }

  async #withFallback(hardware, remote) {
    if (this.mode !== 'service') {
      const v = hardware();
      if (v !== null) return v;
      if (this.mode === 'hardware') {
        throw new RdRandError(
          hardwareSource()
            ? 'the hardware random number generator failed to deliver'
            : 'this CPU has no working hardware random number generator',
        );
      }
    }
    return remote();
  }

  async #post(action, length) {
    const init = {
      method: 'POST',
      headers: { 'User-Agent': `rdrand-client-node/${VERSION}` },
      signal: AbortSignal.timeout(this.timeoutMs),
    };
    if (length !== undefined) init.body = new URLSearchParams({ length: String(length) });
    let res;
    try {
      res = await fetch(`${this.baseUrl}/${action}`, init);
    } catch (e) {
      throw new RdRandError(`rdrand.com API: ${e.message}`, { cause: e });
    }
    const text = await res.text();
    if (!res.ok) throw new RdRandError(`rdrand.com API: HTTP ${res.status}`);
    return text;
  }
}

module.exports = { RdRand, RdRandError, hardwareSource, KEY_ALPHABET, DEFAULT_BASE_URL };
