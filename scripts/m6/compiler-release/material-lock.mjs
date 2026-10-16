import { requireValue } from '../../distribution/canonical.mjs';

export const REGISTRY = 'registry+https://github.com/rust-lang/crates.io-index';
export const identity = entry => `${entry.name}\0${entry.version}`;

function string(value) {
  requireValue(/^"[^"\\\x00-\x1f]*"$/.test(value), 'compiler Cargo.lock basic string');
  return JSON.parse(value);
}

// Deliberately accepts Cargo's generated v4 subset, not a permissive TOML approximation.
export function compilerLockPackages(input) {
  requireValue(Buffer.isBuffer(input) && input.length > 0 && input.length <= 2 * 1024 * 1024,
    'compiler Cargo.lock byte budget');
  const text = new TextDecoder('utf-8', { fatal: true }).decode(input);
  const entries = [];
  let current = null;
  let dependencies = false;
  let header = false;
  for (const line of text.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    if (dependencies) {
      if (trimmed === ']') dependencies = false;
      else {
        requireValue(trimmed.endsWith(','), 'compiler Cargo.lock dependency framing');
        string(trimmed.slice(0, -1));
      }
      continue;
    }
    if (trimmed === '[[package]]') {
      requireValue(header && entries.length < 2048, 'compiler Cargo.lock package count');
      current = {};
      entries.push(current);
      continue;
    }
    if (!current) {
      requireValue(!header && trimmed === 'version = 4', 'compiler Cargo.lock format');
      header = true;
      continue;
    }
    const match = /^(name|version|source|checksum|dependencies) = (.+)$/.exec(trimmed);
    requireValue(match && !Object.hasOwn(current, match[1]), 'compiler Cargo.lock closed package');
    if (match[1] === 'dependencies') {
      requireValue(['[', '[]'].includes(match[2]), 'compiler Cargo.lock dependency array');
      current.dependencies = true;
      dependencies = match[2] === '[';
    } else current[match[1]] = string(match[2]);
  }
  requireValue(header && !dependencies && entries.length > 0, 'compiler Cargo.lock closure');
  const result = new Map();
  for (const entry of entries) {
    requireValue(typeof entry.name === 'string' && /^[A-Za-z0-9_-]+$/.test(entry.name)
      && typeof entry.version === 'string' && /^[0-9][A-Za-z0-9+._-]*$/.test(entry.version)
      && (entry.source === undefined || typeof entry.source === 'string')
      && (entry.source === undefined ? entry.checksum === undefined
        : /^[a-f0-9]{64}$/.test(entry.checksum ?? '')), 'compiler Cargo.lock package identity');
    const key = identity(entry);
    requireValue(!result.has(key), 'compiler Cargo.lock ambiguous duplicate package');
    result.set(key, entry);
  }
  return result;
}
