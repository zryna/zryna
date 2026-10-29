import assert from 'node:assert/strict';
import { lstatSync, mkdirSync, writeFileSync } from 'node:fs';
import { isAbsolute, relative, resolve, sep } from 'node:path';
import { canonical, exact, MAX_LOG, readSafe, sha256 } from './input.mjs';

export function childEnvironment(environment = process.env, witness) {
  const result = Object.fromEntries(Object.entries(environment).filter(([key]) =>
    !/^(GH_TOKEN|GITHUB_TOKEN|ZRYNA_M7_FUZZ_WITNESS)$/i.test(key)));
  if (witness !== undefined) result.ZRYNA_M7_FUZZ_WITNESS = witness;
  return result;
}

// snapshot uses synchronous Git children; scope filtering to that synchronous call.
export function inspectWithoutTokens(inspect, root, environment = process.env) {
  const saved = Object.entries(environment).filter(([key]) => /^(GH_TOKEN|GITHUB_TOKEN)$/i.test(key));
  for (const [key] of saved) delete environment[key];
  try { return inspect(root); }
  finally { for (const [key, value] of saved) environment[key] = value; }
}

export function prepare(root, output) {
  assert(isAbsolute(output), 'absolute new output required');
  const rel = relative(root, output);
  assert(rel.startsWith(`..${sep}`) || isAbsolute(rel), 'output outside checkout');
  for (let parent = resolve(output, '..'); ; parent = resolve(parent, '..')) {
    const stat = lstatSync(parent);
    assert(stat.isDirectory() && !stat.isSymbolicLink(), 'trusted regular parents required');
    if (resolve(parent, '..') === parent) break;
  }
  mkdirSync(output, { mode: 0o700 });
}

export function retain(output, path, bytes) {
  assert(Buffer.isBuffer(bytes) && bytes.length <= MAX_LOG, 'artifact byte cap');
  writeFileSync(resolve(output, path), bytes, { flag: 'wx', mode: 0o600 });
  return descriptor(path, bytes);
}

export function descriptor(path, bytes) {
  return { path, bytes: bytes.length, sha256: sha256(bytes) };
}

export function document(output, path, value) {
  return retain(output, path, Buffer.from(canonical(value)));
}

export function retained(output, pin, path, limit = MAX_LOG) {
  exact(pin, ['path', 'bytes', 'sha256'], 'retained artifact');
  assert.equal(pin.path, path);
  assert(Number.isSafeInteger(pin.bytes) && pin.bytes >= 0 && pin.bytes <= limit);
  const bytes = readSafe(output, path, limit);
  assert.equal(bytes.length, pin.bytes);
  assert.equal(sha256(bytes), pin.sha256);
  return bytes;
}

export function successful(attempt, timeoutMs) {
  exact(attempt, ['exitCode', 'signal', 'error', 'elapsedMs'], 'attempt');
  assert.equal(attempt.exitCode, 0);
  assert.equal(attempt.signal, null);
  assert.equal(attempt.error, null);
  assert(Number.isFinite(attempt.elapsedMs) && attempt.elapsedMs >= 0 && attempt.elapsedMs <= timeoutMs);
}
