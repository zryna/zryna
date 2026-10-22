import assert from 'node:assert/strict';
import path from 'node:path';
import { digest, readStableFile } from './registry-files.mjs';
import { exact, integer, reject } from './evidence-input.mjs';
import { rustAccounting } from './evidence-counts.mjs';
import { loadReviewedLimitContracts, validateLimitMappings } from './limit-contracts.mjs';

const marker = '[m4-public-server-size] ';
const caseId = 'server-public-shape-size-exclusion';
const producer = 'crates/zryna-backend-webassembly/src/component_server/tests/public_size_bound.rs';
const selector = 'component_server::tests::public_size_bound::public_single_scalar_size_upper_bound_is_strictly_below_component_ceiling';
const keys = ['schema', 'operation', 'baselineBytes', 'maximumShapeBytes', 'expressionNodeLimit',
  'maximumTreeNodes', 'leafCount', 'addCount', 'exportNameBytes', 'exportNameOccurrences',
  'opcodeByteBound', 'framingByteBound', 'upperBoundBytes', 'componentByteLimit'];
// Exact emitted sizes are independently reviewed observations of the pinned canonical
// emitter, rather than caller-controlled claims or substitutes for the symbolic bound.
const shapes = Object.freeze([
  Object.freeze({ operation: 'reply', family: 'server-status', maximum: 65805 }),
  Object.freeze({ operation: 'clock_read', family: 'server-clock', maximum: 65865 }),
]);

function sourceDescriptor(registry, sourceRoot, relative) {
  const descriptor = registry.sources.find(item => item.path === relative);
  if (!descriptor) reject('quantitative owning source descriptor missing');
  const bytes = readStableFile(path.join(sourceRoot, relative), 2 * 1024 * 1024, relative);
  assert.equal(bytes.length, descriptor.bytes, 'quantitative owning source bytes');
  assert.equal(digest(bytes), descriptor.sha256, 'quantitative owning source digest');
}
function records(output) {
  const lines = output.split('\n');
  const selected = lines.map((line, index) => ({ line, index }))
    .filter(item => item.line.includes('[m4-public-server-size'));
  assert.equal(selected.length, 2, 'exact two quantitative operation records');
  const start = lines.indexOf('running 1 test');
  const finish = lines.indexOf(`test ${selector} ... ok`);
  return selected.map(({ line, index }) => {
    if (!line.startsWith(marker) || line.length > 2048 || index <= start || index >= finish) {
      reject('deformed or misplaced quantitative record');
    }
    const payload = line.slice(marker.length);
    let record;
    try { record = JSON.parse(payload); } catch { reject('quantitative record JSON'); }
    exact(record, keys, 'quantitative record');
    // The Rust producer emits compact JSON. Re-serialization rejects duplicate keys,
    // ambiguous number spellings, trailing records and alternate/truncated framing.
    assert.equal(JSON.stringify(record), payload, 'quantitative compact unambiguous JSON');
    for (const key of keys.filter(key => !['schema', 'operation'].includes(key))) {
      integer(record[key], 1, 1048576, `quantitative ${key}`);
    }
    assert.equal(record.schema, 'zryna.m4-public-server-size.v1');
    return record;
  });
}
function bound(record, exclusion, contract, shape) {
  const q = exclusion.quantitative;
  assert.equal(q.kind, 'canonical-single-scalar-component');
  assert.equal(q.baselineCaseId, caseId);
  const leaves = (q.expressionNodes + 1) / 2;
  const adds = leaves - 1;
  const opcodes = leaves * q.maximumLiteralBytes + adds;
  const upper = q.baselineMaximumBytes + opcodes +
    q.exportOccurrences * q.maximumExportBytes + q.framingAllowanceBytes;
  assert.equal(record.operation, shape.operation, 'ordered quantitative operation identity');
  assert.equal(record.baselineBytes, q.baselineMaximumBytes, 'actual canonical baseline');
  assert.equal(record.expressionNodeLimit, q.expressionNodes + 1, 'owning IR node limit');
  assert.equal(record.maximumTreeNodes, q.expressionNodes, 'maximum admitted full tree');
  assert.equal(record.leafCount, leaves, 'full binary tree leaves');
  assert.equal(record.addCount, adds, 'full binary tree adds');
  assert.equal(record.exportNameBytes, q.maximumExportBytes, 'owning export name limit');
  assert.equal(record.exportNameOccurrences, q.exportOccurrences, 'scalar and bridge names');
  assert.equal(record.opcodeByteBound, opcodes, 'literal and add encoding formula');
  assert.equal(record.framingByteBound, q.framingAllowanceBytes, 'reviewed framing allowance');
  assert.equal(record.upperBoundBytes, upper, 'complete conservative size formula');
  assert.equal(record.upperBoundBytes, q.maximumBytes, 'independently pinned size bound');
  assert.equal(record.componentByteLimit, contract.maximum, 'owning component envelope');
  assert.equal(record.maximumShapeBytes, shape.maximum, 'actual emitted maximal public shape');
  assert(record.baselineBytes < record.maximumShapeBytes && record.maximumShapeBytes <= upper &&
    upper < record.componentByteLimit, 'maximal public shape strictly below production ceiling');
}

// This checks retained stdout against reviewed source contracts. It does not authenticate
// execution: validateM4Evidence still requires its full source, process and hosted bindings.
export function quantitativeAccounting(output, { registry, sourceRoot, gateId, host }) {
  const entry = registry.cases.find(item => item.id === caseId);
  if (!entry) reject('quantitative source proof case missing');
  if (gateId !== entry.gateId) {
    if (output.includes('[m4-public-server-size')) reject('quantitative record in unrelated gate');
    return null;
  }
  assert(['linux', 'windows'].includes(host), 'quantitative supported host');
  assert.equal(entry.proofScope, 'conditional-public-shape-reachability-exclusion');
  assert.deepEqual(entry.test, { runner: 'rust', package: 'zryna-backend-webassembly', target: 'lib',
    selector, source: producer, ignoredByDefault: false }, 'quantitative producer identity');
  const gate = registry.gates.find(item => item.id === gateId);
  assert.equal(gate?.accounting, 'one-exact-passing-nonignored-rust-test');
  assert.equal(gate.source, producer, 'quantitative owning gate source');
  rustAccounting(output, selector);
  const plan = loadReviewedLimitContracts(registry, sourceRoot);
  validateLimitMappings(registry, plan);
  sourceDescriptor(registry, sourceRoot, producer);
  const actual = records(output);
  const exclusions = plan.exclusions.filter(item => item.quantitative !== null);
  assert.equal(exclusions.length, shapes.length, 'complete quantitative exclusion plan');
  for (const [index, shape] of shapes.entries()) {
    const rowId = `${shape.family}-${host}`;
    const row = registry.rows.find(item => item.id === rowId);
    const limit = row?.limits.find(item => item.id === 'component-bytes');
    const exclusion = exclusions.find(item => item.rowIds.includes(rowId));
    if (!limit || !exclusion || !entry.rowIds.includes(rowId)) reject('quantitative row binding absent');
    assert.equal(limit.mapping.kind, 'excluded', 'source shape exclusion mapping');
    assert.equal(limit.mapping.exclusionId, exclusion.id, 'reviewed row exclusion identity');
    const contract = plan.contracts.find(item => item.id === limit.contractId);
    if (!contract?.rowIds.includes(rowId)) reject('quantitative component contract identity');
    for (const binding of [...exclusion.sourceBindings, ...contract.sourceBindings]) {
      sourceDescriptor(registry, sourceRoot, binding.path);
    }
    bound(actual[index], exclusion, contract, shape);
  }
  return { caseId, operations: actual.map(item => item.operation) };
}
