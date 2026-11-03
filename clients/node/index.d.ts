/// <reference types="node" />

export type Mode = 'auto' | 'hardware' | 'service';
export type Source = 'rdrand' | 'rndr' | 'service';

export interface RdRandOptions {
  /** 'auto' (default): the CPU's generator, else the API. 'hardware': CPU only. 'service': API only. */
  mode?: Mode;
  /** Default 'https://rdrand.com/API/'. */
  baseUrl?: string;
  /** API request timeout. Default 60000. */
  timeoutMs?: number;
}

export class RdRandError extends Error {}

export class RdRand {
  constructor(options?: RdRandOptions);
  readonly mode: Mode;
  /** Where results come from. */
  readonly source: Source;
  /** The CPU's hardware generator, or null if it has none. */
  static hardwareSource(): 'rdrand' | 'rndr' | null;
  /** `length` characters, each uniform over 0-9 A-Z _ a-z. */
  generateKey(length: number): Promise<string>;
  /** `length` random bytes. */
  generateBytes(length: number): Promise<Buffer>;
  /** A random integer from 0 to 2^31 - 1. */
  generateUInt(): Promise<number>;
}

/** The CPU's hardware generator, or null if it has none. */
export function hardwareSource(): 'rdrand' | 'rndr' | null;
export const KEY_ALPHABET: string;
export const DEFAULT_BASE_URL: string;
