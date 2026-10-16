// Private operator test protocol, never a guest capability or a general HTTP API.
import { createHash } from 'node:crypto';

export const bounds = Object.freeze({ frame: 2097152, aggregate: 67108864, frames: 1024,
  requests: 256, live: 4, queued: 32, headers: 4096, stderr: 16384, lifetimeMs: 300000 });
export const fail = () => { throw new Error('PLAYGROUND-UI-TRANSPORT'); };
export const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
export const hash = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
export function exact(value, keys) {
  if (!value || typeof value !== 'object' || Array.isArray(value) ||
      JSON.stringify(Object.keys(value).sort()) !== JSON.stringify([...keys].sort())) fail();
}
export function integer(value, maximum = Number.MAX_SAFE_INTEGER) {
  if (!Number.isSafeInteger(value) || value < 1 || value > maximum) fail();
  return value;
}
export function decode(bytes) {
  if (!(bytes instanceof Uint8Array) || !bytes.length || bytes.length > bounds.frame) fail();
  let text; let value;
  try { text = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true }).decode(bytes); value = JSON.parse(text); }
  catch { fail(); }
  const pending = [[value, 0]];
  while (pending.length) {
    const [entry, depth] = pending.pop();
    if (depth > 8) fail();
    if (entry && typeof entry === 'object') {
      const entries = Object.values(entry);
      if (entries.length > 512) fail();
      for (const child of entries) pending.push([child, depth + 1]);
    } else if (typeof entry === 'number' && (!Number.isSafeInteger(entry) || Object.is(entry, -0))) fail();
  }
  // Exact canonical serialization also rejects duplicate keys and alternate token spellings.
  if (JSON.stringify(value) !== text) fail();
  return value;
}
export function binary(value, maximum) {
  if (typeof value !== 'string' || value.length > Math.ceil(maximum / 3) * 4 ||
      !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(value)) fail();
  const bytes = Buffer.from(value, 'base64');
  if (bytes.length > maximum || bytes.toString('base64') !== value) fail();
  return bytes;
}
export function envelope(value, type, keys) {
  exact(value, ['version', 'type', ...keys]);
  if (value.version !== 1 || value.type !== type) fail();
}
export function origin(value) {
  if (typeof value !== 'string' || !/^http:\/\/127\.0\.0\.1:[1-9][0-9]{0,4}$/.test(value)) fail();
  const parsed = new URL(value);
  if (!parsed.port || Number(parsed.port) > 65535 || parsed.origin !== value) fail();
  return value;
}
export function start(value) {
  envelope(value, 'start', ['runId', 'origin', 'launchPath', 'capability', 'planSha256']);
  if (typeof value.runId !== 'string' || !/^[a-f0-9]{32}$/.test(value.runId) ||
      value.launchPath !== '/examples/playground/restricted/' || !hash(value.capability) ||
      !hash(value.planSha256)) fail();
  origin(value.origin);
  if (Buffer.byteLength(JSON.stringify(value)) > 4096) fail();
  return Object.freeze({ ...value });
}
