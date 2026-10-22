import assert from 'node:assert/strict';
import path from 'node:path';
import { digest, portablePath, readStableFile } from './registry-files.mjs';
import { array, exact, sha, reject } from './evidence-input.mjs';
import { validateInstalledIdentity } from './installed-identities.mjs';
import { installedByteCases } from './installed-byte-boundary.mjs';

const world = 'zryna:capability-profiles/command@0.1.0';
const environment = 'wasi:cli/environment@0.2.12';
const limits = Object.freeze({ fuel: 100000, deadlineMillis: 5000, maxWasmStackBytes: 65536,
  backtraceMaxFrames: 1, instances: 2, memories: 1, tables: 0, memoryPages: 256, memoryBytes: 16777216,
  memoryGrowth: false, sharedMemory: false, memory64: false, staticStart: 0, staticEnd: 65536,
  languageStart: 65536, languageEnd: 15728640, canonicalStart: 15728640, canonicalEnd: 16777216,
  maxLiveTransferEntries: 16, maxTransferAllocations: 4096, maxTransferAllocationBytes: 4096,
  maxTransferBytes: 1048576, maxRequestBytes: 4096, maxKeyBytes: 64, maxValueBytes: 1024,
  maxComponentBytes: 1048576, maxManifestBytes: 16384 });
export function producedJson(bytes, pretty = false) {
  let value;
  try { value = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes)); }
  catch { reject('producer observation is not strict UTF-8 JSON'); }
  const serialized = pretty ? `${JSON.stringify(value, null, 2)}\n` : JSON.stringify(value);
  if (!Buffer.from(serialized).equals(bytes)) reject('producer observation formatting/duplicate field mismatch');
  return value;
}
// These are authenticated producer observations. This does not decode a new runtime
// authority or replace the Rust production decoder/component auditor.
export function validateInstalledObservation(manifest, response, { item, oracle, componentBytes, sourceRoot, registry }) {
  exact(manifest, ['schema', 'source', 'composition', 'component', 'grants', 'input', 'limits', 'execution', 'teardown'], 'produced manifest');
  exact(manifest.source, ['path', 'sha256', 'profile', 'verifierRevision', 'programBinding', 'requirements', 'scalarEntry'], 'manifest source');
  exact(manifest.composition, ['binding', 'root', 'language', 'row', 'world', 'policyVersion', 'approved', 'staticQuota'], 'manifest composition');
  exact(manifest.component, ['path', 'kind', 'sha256', 'languageSha256', 'storageSha256', 'linear32Sha256',
    'linuxX8664Sha256', 'worldSha256', 'witClosureDigest', 'witFileCount', 'world', 'wasiVersion',
    'packages', 'explicitImports', 'resolvedImports', 'exports'], 'manifest component');
  exact(manifest.grants, ['requested', 'effective', 'registryCeilings', 'staticQuota', 'hostPolicy'], 'manifest grants');
  exact(manifest.input, ['kind', 'utf8ByteCount'], 'manifest input');
  exact(manifest.execution, ['kind', 'runReturn'], 'ordinary produced execution');
  assert.equal(manifest.schema, 'zryna.wasi-command-manifest.v1');
  const boundary = installedByteCases[item.name];
  const sourceName = boundary ? boundary.file : item.name === 'declared-error' ? 'error.zry' : oracle[1] === 'none' ? 'pure.zry' : 'lookup.zry';
  portablePath(manifest.source.path);
  assert.equal(manifest.source.path, sourceName);
  const sourceDirectory = boundary ? 'tests/installed-command-qualification/fixtures/byte-boundary' : 'examples/wasi-command';
  assert.equal(manifest.source.sha256, digest(readStableFile(path.join(sourceRoot, sourceDirectory, sourceName), 256 * 1024)));
  if (boundary) {
    assert.equal(manifest.source.sha256, boundary.sha256, 'fixed authored installed exact source');
    assert.equal(manifest.source.sha256, item.source.sha256, 'manifest binds actually invoked retained source');
    assert.equal(componentBytes.length, 1048576, 'actual installed exact component byte ceiling');
  }
  assert.equal(manifest.source.profile, 'command-h1-v1');
  assert.equal(manifest.source.verifierRevision, 1);
  assert.deepEqual(manifest.source.scalarEntry, { name: 'main', abiVersion: 1, abiIndex: 0, parameters: [], result: 'bool' });
  assert.equal(manifest.composition.root, 'command-source');
  assert.equal(manifest.composition.language, 'CommandH1V1');
  assert.equal(manifest.composition.row, 'WitCommand');
  assert.equal(manifest.composition.world, world);
  assert.equal(manifest.composition.policyVersion, 'zryna.wit-capability-profiles.v1');
  const required = oracle[1] === 'none' ? [] : [{ capability: 'environment', interface: environment, key: 'MODE' }];
  for (const grants of [manifest.source.requirements, manifest.composition.approved, manifest.grants.requested, manifest.grants.effective]) {
    assert.deepEqual(grants, required, 'actual bounded environment grant observation');
  }
  const quota = required.length ? [0, 0, 1, 1088, 0, 0, 0, 0, 0, 0] : Array(10).fill(0);
  assert.deepEqual(manifest.composition.staticQuota, quota);
  assert.deepEqual(manifest.grants.staticQuota, quota);
  assert.deepEqual(manifest.grants.registryCeilings, [64, 64, 128, 65536, 16, 256, 64, 128, 65536, 8388608]);
  assert.equal(manifest.grants.hostPolicy, 'zryna.command-h1.host.v1');
  assert.deepEqual(manifest.limits, limits);
  assert.deepEqual(manifest.execution, { kind: 'run-returned', runReturn: oracle[0] });
  assert.equal(manifest.teardown, 'confirmed');
  const value = { found: 'test-only-secret-value', empty: '', utf8: 'हिन्दी🙂' }[item.name];
  assert.deepEqual(manifest.input, { kind: oracle[1], utf8ByteCount: value === undefined ? 0 : Buffer.byteLength(value) });
  assert.equal(manifest.component.path, `wasi-command/${item.name}.wasm`);
  assert.equal(manifest.component.kind, 'wasi-command-component-v1');
  assert.equal(manifest.component.world, world);
  assert.equal(manifest.component.wasiVersion, '0.2.12');
  assert.equal(manifest.component.witFileCount, 34);
  for (const value of [manifest.source.sha256, manifest.source.programBinding, manifest.composition.binding,
    ...['sha256', 'languageSha256', 'storageSha256', 'linear32Sha256', 'linuxX8664Sha256', 'worldSha256', 'witClosureDigest'].map(key => manifest.component[key])]) sha(value);
  assert.equal(manifest.component.sha256, digest(componentBytes));
  validateInstalledIdentity(manifest, registry, sourceRoot);
  exact(response, ['version', 'command', 'profile', 'target', 'ok', 'manifest', 'execution', 'diagnostics'], 'actual positive response');
  assert.equal(response.version, 1); assert.equal(response.command, 'run');
  assert.equal(response.profile, 'command-h1-v1'); assert.equal(response.target, 'wasi-command');
  assert.equal(response.ok, oracle[0] === 'ok');
  assert.equal(response.manifest, `.zryna/out/${item.name}.wasi-command-run/zryna-wasi-command-manifest-v1.json`);
  assert.deepEqual(response.execution, { outcome: { kind: 'run-returned', runReturn: oracle[0] }, teardown: 'confirmed' });
  array(response.diagnostics, 255, 'actual diagnostics');
  return manifest;
}
