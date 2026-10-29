// Static/synthetic only: no command, tool, compiler, guest or benchmark execution.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { canonical, readSafe, sha256 } from '../scripts/stability-gates/input.mjs';
import { checkKnownGraphs, checkSelectedBytes, matrixFor, parseMethodology,
  validateMethodology } from '../scripts/stability-gates/cli-performance-methodology-v4.mjs';
const raw = readFileSync(new URL('./cli-performance-methodology-v4.json', import.meta.url));
const fresh = () => parseMethodology(raw);
const matrixHashes = {
  'linux-x86_64': '1aa5376a752473e8840df428178d2662cfdd63dfe77247f59e0d24c3572adb55',
  'windows-x86_64': 'cbb18c05c7860bdfc8b6b461a37c850c157c28c6edbc85a799f4813cab7faa06',
};
test('current whole-document snapshot is fixed without any historical projection reader', () => {
  const document = fresh();
  assert.equal(sha256(raw), '5aa0ea425c752976a6eb1a4baa7ae7ca703d655270ef5272b78ec28564dffa1c');
  assert.equal(canonical(document), raw.toString('utf8'));
  assert.equal(document.format, 'zryna.cli-performance-methodology.draft.v4');
  assert(document.sourceRole.includes('99fc32b7ad24cfdbb9b3932a79b6a1fcade2d9e2'));
  assert.deepEqual(document.materials.filter(row => ['Cargo.toml', 'Cargo.lock'].includes(row.path)),
    [
      { bytes: 3384, path: 'Cargo.toml',
        sha256: '765ba86bff634ffa0dfebf662c9fc3be00585b0b053c6b6a3e9ec48368e2e568' },
      { bytes: 52083, path: 'Cargo.lock',
        sha256: 'a128e241fa1968308f2d5e2e828c94c29c413e718a7b96368e7a4032ac019751' },
    ]);
});
test('fixed exact argv outputs and all 24 Linux / 20 Windows cells preserve the original matrix', () => {
  for (const host of ['linux-x86_64', 'windows-x86_64']) {
    const rows = matrixFor(fresh(), host);
    assert.equal(rows.length, host === 'linux-x86_64' ? 24 : 20);
    assert.equal(sha256(canonical(JSON.parse(JSON.stringify(rows)))), matrixHashes[host]);
    assert.equal(rows.filter(row => row.phase === 'run' && row.target === 'native').length,
      host === 'linux-x86_64' ? 4 : 0);
  }
  assert.throws(() => matrixFor(fresh(), 'macos-aarch64'));
});
test('actual current selected sources oracles and all four known module graphs match fixed bytes', () => {
  const reader = (path, limit) => readSafe(process.env.ZRYNA_PERFORMANCE_UNIT_SOURCE_ROOT, path, limit);
  assert.equal(checkSelectedBytes(fresh(), reader).files, 27);
  assert.deepEqual(checkKnownGraphs(fresh(), reader).map(row => row.modules), [1, 2, 2, 1]);
});
test('missing changed or truncated selected bytes fail instead of learning current expected hashes', () => {
  for (const reader of [() => Buffer.alloc(0), () => Buffer.from('changed'), () => { throw Error('missing'); }]) {
    assert.throws(() => checkSelectedBytes(fresh(), reader));
  }
});
test('one warmup and five samples original source values and run compile semantics stay frozen', () => {
  const d = fresh(); assert.equal(d.samples.warmupsPerCell, 1); assert.equal(d.samples.measuredPerCell, 5);
  assert.equal(d.phases.run, 'run-cli-compile-plus-execution');
  assert.deepEqual(d.families.map(row => row.expectedValue.value), [-2147483648, 42, 29, 97]);
  assert.deepEqual(d.families.map(row => row.profile), [null, 'control-flow-v1', 'data-ownership-v1', 'bounded-generics-v1']);
});
test('caller rewrites of pins commands graph profiles values samples caps or authority are rejected', () => {
  for (const mutate of [d => { d.materials[3].sha256 = '0'.repeat(64); },
    d => { d.sourceRole = 'caller authenticated'; }, d => { d.families[2].moduleGraph[0].imports = []; },
    d => { d.families[0].entry = 'other.zry'; }, d => { d.families[3].expectedValue.value = 1; },
    d => { d.families[3].profile = null; }, d => { d.samples.measuredPerCell = 1; },
    d => { d.collection.hostWallGuardMs = 999999; }, d => { d.thresholds = { wall: 1 }; },
    d => { d.baselineAuthority = {}; }, d => { d.closureStatus = 'closed'; }]) {
    const document = fresh(); mutate(document); assert.throws(() => validateMethodology(document));
  }
  assert.throws(() => parseMethodology(Buffer.from('{"a":1,"a":2}\n')));
  assert.throws(() => parseMethodology(Buffer.from(JSON.stringify(JSON.parse(raw), null, 2))));
});
test('current declaration and collected status never issue source baseline threshold or closure authority', () => {
  const status = validateMethodology(fresh());
  assert.equal(status.sourceAuthenticated, false); assert.equal(status.performanceQualified, false);
  assert.equal(status.baselineAuthority, null); assert.equal(status.thresholds, null);
  assert.equal(status.collectorImplemented, false); assert.equal(status.hostCells, 44); assert(Object.isFrozen(status));
});
