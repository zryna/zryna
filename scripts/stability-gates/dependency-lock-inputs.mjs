// Fixed lock/material reader for the Python owner. No source/host/advisory disposition issuer.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { closeSync, fsyncSync, openSync, readFileSync, realpathSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { isAbsolute, join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { npmQueries } from './dependency-lock-parser.mjs';
assert.equal(process.version, 'v22.22.1', 'pinned Node version');
const [mode, root] = process.argv.slice(2);
assert.equal(process.argv.length, mode === '--msal-control' ? 5 : 4);
assert(isAbsolute(root) && resolve(root) === root && realpathSync(root) === root);
const request = createRequire(join(root, 'package.json'));
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const emit = value => {
  const text = JSON.stringify(value); assert(Buffer.byteLength(text) <= 1024 * 1024);
  process.stdout.write(text);
};
const yamlInfo = request('yaml/package.json');
assert.equal(yamlInfo.version, '2.8.3', 'actual selected YAML parser');
const { parseDocument } = request('yaml');
const parse = path => {
  const raw = readFileSync(path); assert(raw.length <= 4 * 1024 * 1024);
  const doc = parseDocument(new TextDecoder('utf-8', { fatal: true }).decode(raw), { uniqueKeys: true });
  assert.deepEqual(doc.errors, []); assert.deepEqual(doc.warnings, []);
  return doc.toJS({ maxAliasCount: 0 });
};
if (mode === '--locks') {
  const lock = parse(join(root, 'pnpm-lock.yaml'));
  const packages = npmQueries(lock);
  emit({ format: 'zryna.dependency-lock-inputs.v1', node: process.version,
    yamlVersion: yamlInfo.version, yamlEntrySha256: hash(readFileSync(request.resolve('yaml'))), packages });
} else if (mode === '--msal-control') {
  const output = process.argv[4]; assert(isAbsolute(output) && resolve(output) === output);
  const { execute } = await import(pathToFileURL(join(root, 'scripts/stability-gates/process.mjs')));
  const env = Object.fromEntries(['PATH', 'HOME', 'USERPROFILE', 'TMP', 'TEMP', 'TMPDIR',
    'SystemRoot', 'WINDIR'].filter(key => process.env[key] !== undefined).map(key => [key, process.env[key]]));
  const argv = ['--test', '--test-reporter=tap', 'tests/msal-backport.test.mjs'];
  const result = await execute(process.execPath, argv, { root, timeoutMs: 20000, env });
  for (const [name, raw] of [['msal-owner.stdout.log', result.stdout], ['msal-owner.stderr.log', result.stderr]]) {
    const fd = openSync(join(output, name), 'wx', 0o600);
    try { writeFileSync(fd, raw); fsyncSync(fd); } finally { closeSync(fd); }
  }
  emit({ format: 'zryna.dependency-msal-owner-status.v1', argv: [process.execPath, ...argv],
    exitCode: result.exitCode, signal: result.signal, error: result.error,
    stdoutBytes: result.stdout.length, stderrBytes: result.stderr.length,
    stdoutSha256: hash(result.stdout), stderrSha256: hash(result.stderr) });
} else {
  assert.equal(mode, '--msal');
  const { auditMaterials, root: ownerRoot } = await import(pathToFileURL(join(root, 'tests/msal-backport/materials.mjs')));
  assert.equal(resolve(ownerRoot), root);
  const state = auditMaterials();
  const installed = state.provenance.installedAfterimages.map(pin => {
    const raw = readFileSync(join(state.packageRoot, pin.path)); assert(raw.length <= 2 * 1024 * 1024);
    return { path: pin.path, bytes: raw.length, sha256: hash(raw),
      canonicalLfSha256: hash(Buffer.from(new TextDecoder('utf-8', { fatal: true }).decode(raw).replaceAll('\r\n', '\n'))) };
  });
  emit({ format: 'zryna.dependency-msal-materials.v1', patch: state.provenance.patch,
    upstream: state.provenance.upstream, installed, variance: state.variance });
}
