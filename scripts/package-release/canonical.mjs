import { createHash } from 'node:crypto';

export const MAX_BYTES = 65536;
export const MAX_DEPTH = 6;

export function reject(code, detail) {
  throw new Error(`${code}: ${detail}`);
}

export function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`;
  if (value && typeof value === 'object') {
    return `{${Object.keys(value).sort().map(key =>
      `${JSON.stringify(key)}:${canonical(value[key])}`).join(',')}}`;
  }
  return JSON.stringify(value);
}

export function bytes(value) {
  return Buffer.from(`${canonical(value)}\n`, 'utf8');
}

export function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

export function digest(kind, value) {
  return sha256(Buffer.concat([
    Buffer.from(`ZRYNA-PACKAGE-RELEASE-V1\0${kind}\0`),
    bytes(value),
  ]));
}

// This accepts bytes, never an already parsed producer object.
export function parseCanonical(input) {
  if (!Buffer.isBuffer(input) || input.length > MAX_BYTES) {
    reject('P168-BUDGET', 'wire bytes exceed 65536 or are not a Buffer');
  }
  let text;
  try {
    text = new TextDecoder('utf-8', { fatal: true }).decode(input);
  } catch {
    reject('P168-WIRE', 'invalid UTF-8');
  }
  let depth = 0;
  let quoted = false;
  let escaped = false;
  for (const character of text) {
    if (quoted) {
      if (escaped) escaped = false;
      else if (character === '\\') escaped = true;
      else if (character === '"') quoted = false;
    } else if (character === '"') quoted = true;
    else if (character === '{' || character === '[') {
      if (++depth > MAX_DEPTH) reject('P168-BUDGET', 'nesting exceeds 6');
    } else if (character === '}' || character === ']') depth--;
  }
  let value;
  try {
    value = JSON.parse(text);
  } catch {
    reject('P168-WIRE', 'invalid JSON');
  }
  // Byte equality also rejects duplicate keys, BOM, alternate escapes/numbers,
  // CRLF, whitespace, and differently ordered object keys.
  if (!bytes(value).equals(input)) reject('P168-WIRE', 'noncanonical JSON bytes');
  return value;
}
