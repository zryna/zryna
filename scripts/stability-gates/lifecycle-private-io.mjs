// Private fixed-role transport. These descriptors and observations grant no approval.
import assert from 'node:assert/strict';
import { writeFileSync } from 'node:fs';
import { gunzipSync } from 'node:zlib';
import { fetchQualificationResource } from '../distribution-release/acquire-qualification-materials.mjs';
import { exact, parse } from './input.mjs';

export const ORIGINALS = Object.freeze(['archive', 'envelope', 'bundle', 'trusted-root', 'cosign',
  'python', 'libc6', 'libgcc-s1', 'libstdcxx6', 'gcc-14-base']);

export function dispatchData(encoded) {
  assert(typeof encoded === 'string' && encoded.length <= 65535);
  const bytes = Buffer.from(encoded, 'base64');
  assert.equal(bytes.toString('base64'), encoded, 'canonical compressed JSON carrier');
  return new TextDecoder('utf-8', { fatal: true }).decode(gunzipSync(bytes, { maxOutputLength: 262144 }));
}

export function selection(raw, operation) {
  assert(['capture', 'execute'].includes(operation));
  const value = parse(Buffer.from(raw));
  exact(value, operation === 'capture' ? ['downloads'] : ['artifact', 'assetsSha256', 'auth', 'preparation'], 'private role');
  if (operation === 'execute') assert.match(value.assetsSha256, /^[a-f0-9]{64}$/u);
  else {
    assert(Array.isArray(value.downloads) && value.downloads.length <= 32);
    const names = new Set();
    for (const row of value.downloads) {
      exact(row, ['bytes', 'name', 'sha256', 'url'], 'download');
      assert(/^[a-z0-9-]{1,64}$/u.test(row.name) && !names.has(row.name)); names.add(row.name);
      assert(Number.isSafeInteger(row.bytes) && row.bytes > 0 && row.bytes <= 192 * 1024 ** 2);
      assert.match(row.sha256, /^[a-f0-9]{64}$/u);
      const url = new URL(row.url);
      assert(url.protocol === 'https:' && !url.username && !url.password);
    }
    assert(ORIGINALS.every(name => names.has(name)), 'complete original input producer');
  }
  return value;
}

export function save(path, data) {
  writeFileSync(path, Buffer.isBuffer(data) ? data : JSON.stringify(data) + '\n', { flag: 'wx', mode: 0o600 });
}

export async function download(row, path) {
  const response = await fetch(row.url, { signal: AbortSignal.timeout(30000), headers: { 'accept-encoding': 'identity' } });
  assert(response.ok && response.body && new URL(response.url).protocol === 'https:');
  // Public redirects are transport only; the independently selected whole hash binds bytes.
  // The original30s signal still governs this body; the existing checker enforces all bounds.
  const bytes = await fetchQualificationResource({ url: response.url, maximum: row.bytes,
    size: row.bytes, digest: row.sha256 }, { fetchImpl: async () => response });
  save(path, bytes);
}
