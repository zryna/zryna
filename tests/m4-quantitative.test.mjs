import assert from 'node:assert/strict';
import test from 'node:test';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { digest } from '../scripts/m4/registry-files.mjs';
import { quantitativeAccounting } from '../scripts/m4/evidence-quantitative.mjs';
import { processResult } from '../scripts/m4/evidence-input.mjs';
import { supplementalGates, requireM4ResultInventory, selectM4Gates } from '../scripts/m4/evidence.mjs';

const sourceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const registry = JSON.parse(readFileSync(path.join(sourceRoot, 'tests/m4-conformance-v1.json')));
const producer = 'crates/zryna-backend-webassembly/src/component_server/tests/public_size_bound.rs';
const gateId = 'case-server-public-shape-size-exclusion';
const marker = '[m4-public-server-size] ';
// Recorded Linux dirty #402 focused test, 1 passed / 0 failed / 0 ignored. The
// original combined stream includes Cargo stderr before actual test stdout. Its
// producer hash and full transcript digest are checked below; this is a parser
// fixture, not hosted execution authentication or a complete M4 qualification.
const actual = "   Compiling zryna-backend-webassembly v0.2.3 (/workspace/zryna/crates/zryna-backend-webassembly)\n    Finished `test` profile [unoptimized] target(s) in 3.70s\n     Running unittests src/lib.rs (target/debug/deps/zryna_backend_webassembly-6fbe330aa32e064b)\n\nrunning 1 test\n[m4-public-server-size] {\"addCount\":8191,\"baselineBytes\":8212,\"componentByteLimit\":1048576,\"exportNameBytes\":128,\"exportNameOccurrences\":2,\"expressionNodeLimit\":16384,\"framingByteBound\":256,\"leafCount\":8192,\"maximumShapeBytes\":65805,\"maximumTreeNodes\":16383,\"opcodeByteBound\":57343,\"operation\":\"reply\",\"schema\":\"zryna.m4-public-server-size.v1\",\"upperBoundBytes\":66067}\n[m4-public-server-size] {\"addCount\":8191,\"baselineBytes\":8272,\"componentByteLimit\":1048576,\"exportNameBytes\":128,\"exportNameOccurrences\":2,\"expressionNodeLimit\":16384,\"framingByteBound\":256,\"leafCount\":8192,\"maximumShapeBytes\":65865,\"maximumTreeNodes\":16383,\"opcodeByteBound\":57343,\"operation\":\"clock_read\",\"schema\":\"zryna.m4-public-server-size.v1\",\"upperBoundBytes\":66127}\ntest component_server::tests::public_size_bound::public_single_scalar_size_upper_bound_is_strictly_below_component_ceiling ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 127 filtered out; finished in 0.33s\n\n";
const options = { registry, sourceRoot, gateId, host: 'linux' };
const check = (output = actual, changes = {}) => quantitativeAccounting(output, { ...options, ...changes });
function mutateRecord(output, change, operation = 'reply') {
  return output.split('\n').map(line => {
    if (!line.startsWith(marker)) return line;
    const record = JSON.parse(line.slice(marker.length));
    if (record.operation === operation) change(record);
    return marker + JSON.stringify(record);
  }).join('\n');
}
function temporary(callback) {
  const root = mkdtempSync(path.join(os.tmpdir(), 'm4-quantitative-'));
  try {
    const plan = JSON.parse(readFileSync(path.join(sourceRoot, registry.contractPlan.path)));
    const sources = new Set([producer, registry.contractPlan.path,
      'schemas/zryna-m4-limit-contracts-v1.schema.json', 'package.json',
      ...plan.exclusions.filter(item => item.quantitative).flatMap(item => item.sourceBindings.map(binding => binding.path)),
      ...plan.contracts.filter(item => item.id.startsWith('server-') && item.limitId === 'component-bytes')
        .flatMap(item => item.sourceBindings.map(binding => binding.path))]);
    for (const relative of sources) {
      mkdirSync(path.dirname(path.join(root, relative)), { recursive: true });
      copyFileSync(path.join(sourceRoot, relative), path.join(root, relative));
    }
    symlinkSync(path.join(sourceRoot, 'node_modules'), path.join(root, 'node_modules'),
      process.platform === 'win32' ? 'junction' : 'dir');
    return callback(root);
  } finally { rmSync(root, { recursive: true, force: true }); }
}

test('recorded quantitative Linux transcript is bound to the actual producer and original log', () => {
  assert.equal(digest(actual), 'f31801e323b37ac54841043d51a7783b6fcc8e1bf0a3a11caa720f46530e6ceb');
  assert.equal(digest(readFileSync(path.join(sourceRoot, producer))),
    '203a77f9114eeb834171fc913b39f7aa9bf9761d59de21af5dea6cfc70dde0e2');
  assert.deepEqual(check(), { caseId: 'server-public-shape-size-exclusion', operations: ['reply', 'clock_read'] });
});
test('quantitative metrics bind both platform row contracts without supplying host authentication', () => {
  assert.deepEqual(check(actual, { host: 'windows' }), check());
  assert.throws(() => check(actual, { host: 'darwin' }));
});
test('retained Windows CRLF output reaches quantitative accounting through the actual process reader', () => {
  const root = mkdtempSync(path.join(os.tmpdir(), 'm4-quantitative-crlf-'));
  try {
    mkdirSync(path.join(root, 'logs'));
    const descriptor = (suffix, value) => {
      const relative = `logs/${gateId}.${suffix}.log`;
      const bytes = Buffer.from(value);
      writeFileSync(path.join(root, relative), bytes);
      return { path: relative, bytes: bytes.length, sha256: digest(bytes) };
    };
    const result = { id: gateId, argv: ['cargo', 'test'], launchArgv: ['/observed/cargo', 'test'],
      exitCode: 0, signal: null, error: null, elapsedMs: 1, attachments: [],
      stdout: descriptor('stdout', actual.replace(/\n/g, '\r\n')), stderr: descriptor('stderr', '') };
    const logs = processResult(result, root, `logs/${gateId}`);
    assert.deepEqual(check(logs.stdout, { host: 'windows' }), check());
  } finally { rmSync(root, { recursive: true, force: true }); }
});
test('new quantitative and closed limit self tests stay in the existing bounded Node gate', () => {
  const hostile = supplementalGates.find(item => item.id === 'm4-hostile-self-tests');
  for (const name of ['tests/m4-limit-contracts.test.mjs', 'tests/m4-quantitative.test.mjs']) {
    assert.equal(hostile.argv.filter(item => item === name).length, 1);
  }
  assert.equal(supplementalGates.length, 3);
  const gates = selectM4Gates(registry, 'core', 'linux');
  assert(gates.length <= 80);
  requireM4ResultInventory(gates.map(item => ({ id: item.id })), gates);
});
test('green selected Rust test without quantitative output cannot qualify the exclusion', () => {
  assert.throws(() => check(actual.split('\n').filter(line => !line.startsWith(marker)).join('\n')));
});
test('quantitative records cannot be borrowed by an unrelated gate', () => {
  assert.throws(() => check(actual, { gateId: 'case-server-status-public' }));
  assert.equal(check('ordinary output\n', { gateId: 'case-server-status-public' }), null);
});

for (const [name, change] of [
  ['missing reply record', output => output.split('\n').filter(line => !line.includes('"operation":"reply"')).join('\n')],
  ['missing clock record', output => output.split('\n').filter(line => !line.includes('"operation":"clock_read"')).join('\n')],
  ['duplicate reply record', output => output.replace('"operation":"clock_read"', '"operation":"reply"')],
  ['extra record', output => output + output.split('\n').find(line => line.startsWith(marker)) + '\n'],
  ['records reversed', output => output.replace(/(\[m4-public-server-size\].*)\n(\[m4-public-server-size\].*)/, '$2\n$1')],
  ['truncated JSON', output => output.replace('"upperBoundBytes":66067}', '"upperBoundBytes":66067')],
  ['duplicate JSON key', output => output.replace('"baselineBytes":8212', '"baselineBytes":8000,"baselineBytes":8212')],
  ['numeric alternate spelling', output => output.replace('"baselineBytes":8212', '"baselineBytes":8.212e3')],
  ['trailing JSON', output => output.replace('"upperBoundBytes":66067}', '"upperBoundBytes":66067}{}')],
  ['extra marker prefix', output => output.replace(marker, `prefix ${marker}`)],
  ['malformed marker', output => output.replace(marker, '[m4-public-server-size]')],
  ['record after summary', output => {
    const lines = output.split('\n'); const record = lines.splice(lines.findIndex(line => line.startsWith(marker)), 1);
    return lines.join('\n') + record[0] + '\n';
  }],
  ['missing selected test row', output => output.replace(/^test component_server.*\n/m, '')],
  ['wrong selected test row', output => output.replace('test component_server::tests::public_size_bound::', 'test unrelated::')],
  ['ignored selected test', output => output.replace(' ... ok', ' ... ignored')],
  ['failed test summary', output => output.replace('1 passed; 0 failed', '0 passed; 1 failed')],
]) test(`quantitative receipt rejects ${name}`, () => assert.throws(() => check(change(actual))));

for (const [name, change] of [
  ['wrong schema', record => { record.schema = 'zryna.m4-public-server-size.v2'; }],
  ['unknown operation', record => { record.operation = 'fabricated'; }],
  ['unknown field', record => { record.passed = true; }],
  ['missing field', record => { delete record.maximumShapeBytes; }],
  ['string byte count', record => { record.baselineBytes = '8212'; }],
  ['fractional byte count', record => { record.baselineBytes = 8212.5; }],
  ['zero byte count', record => { record.baselineBytes = 0; }],
  ['unsafe byte count', record => { record.baselineBytes = Number.MAX_SAFE_INTEGER + 1; }],
  ['lower baseline', record => { record.baselineBytes--; }],
  ['higher baseline', record => { record.baselineBytes++; }],
  ['fabricated maximal shape', record => { record.maximumShapeBytes--; }],
  ['maximal shape exceeds symbolic bound', record => { record.maximumShapeBytes = record.upperBoundBytes + 1; }],
  ['wrong IR node limit', record => { record.expressionNodeLimit--; }],
  ['wrong tree nodes', record => { record.maximumTreeNodes++; }],
  ['wrong leaf count', record => { record.leafCount--; }],
  ['wrong add count', record => { record.addCount++; }],
  ['wrong export name limit', record => { record.exportNameBytes--; }],
  ['wrong export occurrence count', record => { record.exportNameOccurrences--; }],
  ['wrong literal encoding bound', record => { record.opcodeByteBound--; }],
  ['wrong framing bound', record => { record.framingByteBound--; }],
  ['lower upper bound', record => { record.upperBoundBytes--; }],
  ['higher upper bound', record => { record.upperBoundBytes++; }],
  ['upper bound equals production ceiling', record => { record.upperBoundBytes = record.componentByteLimit; }],
  ['lower production envelope', record => { record.componentByteLimit--; }],
  ['raised production envelope', record => { record.componentByteLimit++; }],
  ['self consistent fabricated baseline and bound', record => { record.baselineBytes++; record.upperBoundBytes++; }],
  ['self consistent fabricated encoding and bound', record => { record.opcodeByteBound++; record.upperBoundBytes++; }],
]) test(`quantitative receipt rejects ${name}`, () => assert.throws(() => check(mutateRecord(actual, change))));

test('clock metrics cannot reuse reply baseline maximal shape or upper bound', () => {
  for (const [key, value] of [['baselineBytes', 8212], ['maximumShapeBytes', 65805], ['upperBoundBytes', 66067]]) {
    assert.throws(() => check(mutateRecord(actual, record => { record[key] = value; }, 'clock_read')));
  }
});
test('caller cannot substitute the quantitative producer or excluded row identity', () => {
  for (const change of [
    document => { document.cases.find(item => item.id === 'server-public-shape-size-exclusion').test.source = 'fabricated.rs'; },
    document => { document.cases.find(item => item.id === 'server-public-shape-size-exclusion').test.selector = 'fabricated'; },
    document => { document.cases.find(item => item.id === 'server-public-shape-size-exclusion').rowIds = ['server-status-linux']; },
    document => { document.rows.find(item => item.id === 'server-status-linux').limits
      .find(item => item.id === 'component-bytes').mapping.exclusionId = 'server-clock-component-bytes-public-input-exclusion'; },
  ]) {
    const document = structuredClone(registry); change(document);
    assert.throws(() => check(actual, { registry: document }));
  }
});
test('changed metric producer source rejects the preserved green transcript', () => temporary(root => {
  check(actual, { sourceRoot: root });
  writeFileSync(path.join(root, producer), readFileSync(path.join(root, producer)) + '\n// altered\n');
  assert.throws(() => check(actual, { sourceRoot: root }), /quantitative owning source bytes/);
}));
test('changed owning guard source rejects the preserved green transcript', () => temporary(root => {
  const guard = 'crates/zryna-backend-webassembly/src/component_server/shell.rs';
  const bytes = readFileSync(path.join(root, guard)); bytes[0] ^= 1;
  writeFileSync(path.join(root, guard), bytes);
  assert.throws(() => check(actual, { sourceRoot: root }), /quantitative owning source digest/);
}));
test('quantitative evidence independently rejects an unpinned self consistent plan', () => temporary(root => {
  const document = structuredClone(registry);
  const planPath = path.join(root, document.contractPlan.path);
  const plan = JSON.parse(readFileSync(planPath));
  const q = plan.exclusions.find(item => item.quantitative).quantitative;
  q.baselineMaximumBytes++; q.maximumBytes++;
  const bytes = Buffer.from(JSON.stringify(plan, null, 2) + '\n');
  writeFileSync(planPath, bytes);
  Object.assign(document.contractPlan, { bytes: bytes.length, sha256: digest(bytes) });
  assert.throws(() => check(mutateRecord(actual, record => {
    record.baselineBytes++; record.upperBoundBytes++;
  }), { sourceRoot: root, registry: document }), /limit contract authority drifted/);
}));
test('quantitative evidence independently rejects changed closed plan schema', () => temporary(root => {
  const schema = path.join(root, 'schemas/zryna-m4-limit-contracts-v1.schema.json');
  writeFileSync(schema, readFileSync(schema) + '\n');
  assert.throws(() => check(actual, { sourceRoot: root }), /limit schema authority drifted/);
}));
