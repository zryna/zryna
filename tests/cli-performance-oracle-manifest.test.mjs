// Pure retained-manifest parser controls; observed examples are unmeasured, never new codegen goldens.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { matrixFor, parseMethodology } from '../scripts/stability-gates/cli-performance-methodology-v4.mjs';
import { validateRetainedManifest } from '../scripts/stability-gates/cli-performance-oracle-manifest.mjs';
import { validateOracleObservation } from '../scripts/stability-gates/cli-performance-oracle-capture.mjs';
const document = parseMethodology(readFileSync(new URL('./cli-performance-methodology-v4.json', import.meta.url)));
const fixture = JSON.parse(readFileSync(new URL('./cli-performance-oracle-manifest-examples.json', import.meta.url)));
function context(selector, host = 'linux-x86_64') {
  const example = fixture.examples.find(e => e.selector === selector);
  const row = matrixFor(document, host).find(r => r.id === selector); assert(row);
  return { document, row, host, manifestBytes: Buffer.from(example.manifest), files: structuredClone(example.files),
    readOriginal: path => Buffer.from(fixture.sources[path]) };
}
function changed(selector, mutate, host = 'linux-x86_64') {
  const value = context(selector, host); validateRetainedManifest(value); // meaningful valid baseline first
  const manifest = JSON.parse(value.manifestBytes); mutate(manifest);
  value.manifestBytes = Buffer.from(`${JSON.stringify(manifest)}\n`);
  value.files = value.files.map(p => p.path === value.row.expectedResponse.manifest ?
    { ...p, bytes: value.manifestBytes.length, sha256: sha256(value.manifestBytes) } : p);
  assert.throws(() => validateRetainedManifest(value)); // file binding updated, so semantics must reject
}
test('retained manifests accept all 24 observed original contracts and 20 host-appropriate Windows build/run contracts', () => {
  for (const row of matrixFor(document, 'linux-x86_64')) validateRetainedManifest(context(row.id));
  for (const row of matrixFor(document, 'windows-x86_64')) validateRetainedManifest(context(row.id, 'windows-x86_64'));
});
test('retained manifest fixed profile version protocol command stem target and entrypoint reject plausible relabeling', () => {
  for (const [id, mutations] of [
    ['m1-wrap/build/javascript', [m => { m.profile = 'other'; }, m => { m.version = 2; }, m => { m.entrypoint = 'other.zry'; }]],
    ['m2-modules/build/javascript', [m => { m.profile = 'control-flow-v1'; }, m => { m.command = 'run'; }, m => { m.stem += '_other'; }]],
    ['m3-ownership/build/javascript', [m => { m.protocol_version = 3; }, m => { m.version = 4; }, m => { m.targets = ['native']; }]],
    ['m7-mixed/build/javascript', [m => { m.format = 'zryna.generic-bundle.v2'; }, m => { m.syntaxProtocol = 4; },
      m => { m.artifactStem += '_other'; }, m => { m.source.entrypoint = 'other.zry'; }, m => { m.extra = true; }]],
  ]) for (const mutate of mutations) changed(id, mutate);
});
test('retained manifest original sources staged paths named edges and recomputed graph digest reject substituted closure', () => {
  changed('m1-wrap/build/javascript', m => { m.source_sha256 = '0'.repeat(64); });
  for (const mutate of [m => { m.sources[0].sha256 = '0'.repeat(64); }, m => { m.sources[0].id = 1; },
    m => { m.sources.reverse(); }, m => { m.sources.pop(); }, m => { m.edges[0].target = '../other.zry'; },
    m => { m.edges[0].specifier = './other.zry'; }, m => { m.edges[0].imported = 'other'; },
    m => { m.edges[0].local = 'other'; }, m => { m.graph_sha256 = '0'.repeat(64); }]) changed('m2-modules/build/javascript', mutate);
  changed('m3-ownership/build/javascript', m => { m.sources[0].path = 'tests/m3-fixtures/conformance/owned-aggregate.zry'; });
  for (const mutate of [m => { m.source.modules[0].bytes += 1; }, m => { m.source.modules[0].path = '../original.zry'; },
    m => { m.source.entryModule = 1; }, m => { m.source.graphSha256 = '0'.repeat(64); }, m => { m.source.modules = []; }])
    changed('m7-mixed/build/javascript', mutate);
});
test('retained artifact declarations require exact target kind safe derived path size digest and complete actual files', () => {
  for (const mutate of [m => { m.artifacts[0].kind = 'windows-x86-64-coff-object'; },
    m => { m.artifacts[0].target = 'native'; }, m => { m.artifacts[0].path = '../outside.mjs'; },
    m => { m.artifacts[0].path = '/absolute.mjs'; }, m => { m.artifacts[0].bytes += 1; },
    m => { m.artifacts[0].sha256 = '0'.repeat(64); }, m => { m.artifacts = []; }]) changed('m1-wrap/build/javascript', mutate);
  for (const mutate of [m => { m.artifacts[0].filename = '../outside.mjs'; },
    m => { m.artifacts[0].metadata.scalar_abi = 'v2'; }, m => { delete m.runtime_abi; }]) changed('m3-ownership/build/javascript', mutate);
  const extra = context('m1-wrap/build/javascript'); extra.files.push({ path: '.zryna/out/unclaimed.bin', bytes: 1, sha256: '0'.repeat(64) });
  assert.throws(() => validateRetainedManifest(extra));
});
test('retained invocation and typed results agree with frozen run oracle and builds cannot carry invented outcomes', () => {
  for (const [id, mutate] of [
    ['m1-wrap/run/javascript', m => { m.invocation.export = 'other'; }],
    ['m2-modules/run/javascript', m => { m.results[0].outcome.value += 1; }],
    ['m3-ownership/run/javascript', m => { m.results[0].outcome.value.type = 'bool'; }],
    ['m7-mixed/run/javascript', m => { m.invocation.outcomes[0].outcome.value.value += 1; }],
    ['m7-mixed/build/javascript', m => { m.invocation = { export: 'entry', arguments: [], outcomes: [] }; }],
    ['m2-modules/build/javascript', m => { m.results = [{ target: 'javascript', outcome: { kind: 'returned', type: 'i32', value: 42 } }]; }],
  ]) changed(id, mutate);
});
test('retained manifest bounded JSON rejects duplicate missing unsafe records and detached raw-byte pins', () => {
  const good = context('m1-wrap/build/javascript'); validateRetainedManifest(good);
  for (const manifestBytes of [Buffer.from('{bad'), Buffer.alloc(262145), Buffer.from('{"version":1,"version":1}\n')]) {
    const value = { ...good, manifestBytes, files: good.files.map(p => p.path === good.row.expectedResponse.manifest ?
      { ...p, bytes: manifestBytes.length, sha256: sha256(manifestBytes) } : p) };
    assert.throws(() => validateRetainedManifest(value));
  }
  for (const files of [good.files.filter(p => p.path !== good.row.expectedResponse.manifest),
    good.files.map(p => p.path === good.row.expectedResponse.manifest ? { ...p, bytes: p.bytes + 1 } : p),
    good.files.map(p => p.path === good.row.expectedResponse.manifest ? { ...p, sha256: '0'.repeat(64) } : p)])
    assert.throws(() => validateRetainedManifest({ ...good, files }));
  const wrongOriginal = { ...good, readOriginal: () => Buffer.from('not the original source') };
  assert.throws(() => validateRetainedManifest(wrongOriginal));
});
test('unchanged native ELF artifact and existing M2 native golden are enforced on Windows without a COFF substitution', () => {
  const value = context('m2-modules/build/native', 'windows-x86_64'); validateRetainedManifest(value);
  const stdout = Buffer.from(JSON.stringify(value.row.expectedResponse));
  validateOracleObservation(document, value.row, value.host, stdout, Buffer.alloc(0), value.files);
  const mutated = value.files.map(p => p.path.endsWith('.o') ? { ...p, sha256: 'f'.repeat(64) } : p);
  assert.throws(() => validateOracleObservation(document, value.row, value.host, stdout, Buffer.alloc(0), mutated), /golden/u);
  changed('m2-modules/build/native', m => { m.artifacts[0].kind = 'windows-x86-64-coff-object'; }, 'windows-x86_64');
});
