// Comparison records are evidence consumers, never an execution or publication authority.
import { createHash } from 'node:crypto';
import { decodeJson } from '../../json.mjs';
import { utf8 } from '../../limits.mjs';

export const fail = () => { throw new Error('PLAYGROUND-CLI-PARITY'); };
export const sha256 = value => createHash('sha256').update(value).digest('hex');
export const hash = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
export function exact(value, keys) {
  if (!value || typeof value !== 'object' || Array.isArray(value) ||
      JSON.stringify(Object.keys(value).sort()) !== JSON.stringify([...keys].sort())) fail();
}
export function canonical(value) {
  if (Array.isArray(value)) return '[' + value.map(canonical).join(',') + ']';
  if (value && typeof value === 'object') return '{' + Object.keys(value).sort()
    .map(key => JSON.stringify(key) + ':' + canonical(value[key])).join(',') + '}';
  if (!(value === null || typeof value === 'string' || typeof value === 'boolean' || Number.isSafeInteger(value)) ||
      Object.is(value, -0)) fail();
  return JSON.stringify(value);
}
export const packageBytes = value => Buffer.from(canonical(value) + '\n');
export const packageDigest = (kind, value) => sha256(Buffer.concat([
  Buffer.from('ZRYNA-PACKAGE-RELEASE-V1\0' + kind + '\0'), packageBytes(value),
]));
export function freeze(value) {
  if (value && typeof value === 'object') {
    for (const child of Object.values(value)) freeze(child);
    Object.freeze(value);
  }
  return value;
}
export function jsonOutput(bytes) {
  if (!(bytes instanceof Uint8Array) || bytes.length < 1 || bytes.length > 65536) fail();
  return decodeJson(bytes, 65536);
}
export function checkedText(value) {
  if (typeof value !== 'string' || utf8(value).length > 4096) fail();
  return value;
}
