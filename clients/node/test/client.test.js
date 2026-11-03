'use strict';
// node --test   (set RDRAND_TEST_URL to also exercise the API, e.g. https://rdrand.com/API/)

const test = require('node:test');
const assert = require('node:assert');
const { RdRand, RdRandError, hardwareSource, KEY_ALPHABET } = require('..');

const isKey = (s, n) => s.length === n && [...s].every((c) => KEY_ALPHABET.includes(c));

test('reports the hardware generator', () => {
  const hw = hardwareSource();
  console.log(`hardware generator: ${hw ?? 'none'} (${process.platform}-${process.arch})`);
  assert.ok(hw === null || hw === 'rdrand' || hw === 'rndr');
  assert.strictEqual(new RdRand().source, hw ?? 'service');
  assert.strictEqual(new RdRand({ mode: 'service' }).source, 'service');
});

test('hardware mode', async () => {
  const rng = new RdRand({ mode: 'hardware' });
  if (!hardwareSource()) {
    await assert.rejects(rng.generateKey(4), RdRandError);
    return;
  }
  assert.ok(isKey(await rng.generateKey(5000), 5000));
  assert.strictEqual(await rng.generateKey(0), '');
  const bytes = await rng.generateBytes(1 << 20);
  assert.strictEqual(bytes.length, 1 << 20);
  for (let i = 0; i < bytes.length; i += 4) assert.notStrictEqual(bytes.readUInt32LE(i), 0);
  const n = await rng.generateUInt();
  assert.ok(Number.isInteger(n) && n >= 0 && n < 2 ** 31);
});

test('key characters are uniform (hardware)', async (t) => {
  if (!hardwareSource()) return t.skip('no hardware generator');
  const s = await new RdRand({ mode: 'hardware' }).generateKey(252000);
  const counts = new Map();
  for (const c of s) counts.set(c, (counts.get(c) || 0) + 1);
  assert.strictEqual(counts.size, 63);
  const expected = s.length / 63;
  const chi2 = [...counts.values()].reduce((a, c) => a + (c - expected) ** 2 / expected, 0);
  assert.ok(chi2 < 120, `chi-square ${chi2}`);
});

test('service mode', async (t) => {
  const url = process.env.RDRAND_TEST_URL;
  if (!url) return t.skip('RDRAND_TEST_URL not set');
  const rng = new RdRand({ mode: 'service', baseUrl: url });
  assert.ok(isKey(await rng.generateKey(100), 100));
  assert.strictEqual(await rng.generateKey(0), '');
  assert.strictEqual((await rng.generateBytes(1000)).length, 1000);
  const n = await rng.generateUInt();
  assert.ok(n >= 0 && n < 2 ** 31);
  await assert.rejects(new RdRand({ mode: 'service', baseUrl: url + 'Nope/' }).generateUInt(), RdRandError);
});

test('auto mode always answers', async () => {
  const rng = new RdRand({ baseUrl: process.env.RDRAND_TEST_URL || undefined });
  assert.ok(isKey(await rng.generateKey(32), 32));
  assert.strictEqual((await rng.generateBytes(16)).length, 16);
});

test('rejects bad lengths', async () => {
  const rng = new RdRand();
  await assert.rejects(rng.generateKey(-1), RangeError);
  await assert.rejects(rng.generateBytes(1.5), RangeError);
  assert.throws(() => new RdRand({ mode: 'nope' }), RangeError);
});
