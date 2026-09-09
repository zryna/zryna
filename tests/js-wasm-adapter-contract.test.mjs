import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const root = new URL('../', import.meta.url);
const read = relative => readFile(new URL(relative, root), 'utf8');
const contract = await read('spec/interop/JS_WASM_ADAPTERS_V1.md');
const proof = await read('spec/interop/JS_WASM_ADAPTER_CONFORMANCE_V1.md');
const vectors = JSON.parse(await read('tests/js-wasm-adapter-v1-vectors.json'));

function table(document, header, columns) {
  const lines = document.split(/\r?\n/);
  const start = lines.indexOf(header);
  assert(start >= 0, `missing table: ${header}`);
  const rows = [];
  for (let index = start + 2; lines[index]?.startsWith('|'); index += 1) {
    const cells = lines[index].split('|').slice(1, -1).map(cell => cell.trim());
    assert.equal(cells.length, columns, cells[0]);
    assert(cells.every(cell => cell.length > 0), cells[0]);
    rows.push(cells);
  }
  assert(rows.length > 0, header);
  assert.equal(new Set(rows.map(row => row[0])).size, rows.length, header);
  return rows;
}

test('adapter design has a closed conversion set with per-row language and library gates', async () => {
  assert.match(contract, /Contract identity: `zryna\.js-wasm-adapters\.v1`\. State: \*\*specified-only\*\*/);
  assert.equal(vectors.contract, 'zryna.js-wasm-adapters.v1');
  assert.equal(vectors.status, 'specified-only');
  const rows = table(contract,
    '| Value | ESM input / output | WIT input / output | Copy or lifetime rule | Required language/interface gate |', 5);
  assert.deepEqual(rows.map(row => row[0]),
    ['i32', 'bool', 'string', 'list-i32', 'list-bool', 'list-string', 'resource']);
  const scalar = JSON.parse(await read('spec/abi/scalar-v1-fixtures.json'));
  assert.equal(scalar.abi, 'zryna-scalar-v1');
  assert.deepEqual(scalar.validExports.find(item => item.logical === 'add').parameters, ['i32', 'i32']);
  assert.match(rows[0][1], /excluding negative zero/);
  assert.match(rows[1][4], /ControlFlowV1 or DataOwnershipV1/);
  assert(rows.slice(2).every(row => row[4].startsWith('accepted ')));
  assert.match(contract, /Their per-API alignment is complete at the design boundary/);
  assert.match(contract, /\[minimal core and host libraries v0\]\(\.\.\/libraries\/MINIMAL_CORE_HOST_V0\.md\)/);
  assert.match(contract, /Exact `zryna\.cross-target-profiles\.v1` composition authority/);
  assert.match(contract, /\[cross-target profiles v1\]\(\.\.\/language\/CROSS_TARGET_PROFILES_V1\.md\)/);
  assert.match(contract, /No nullability is implicit/);
  assert.match(contract, /Callbacks are excluded in v1/);
});

test('adapter world identities and capability ceilings agree with the owning WIT registry', async () => {
  const wit = JSON.parse(await read('tests/wit-capability-profiles-v1.json'));
  const boundaries = table(contract,
    '| Boundary | Interface and compatibility identity | Consumer assumption | Availability |', 4);
  const capabilities = table(contract,
    '| Boundary | clock | environment | filesystem | network | randomness | Exact operation status |', 7);
  const composition = table(await read('spec/language/CROSS_TARGET_PROFILES_V1.md'),
    '| Row | Output target | Language admission | Deployment boundary | clock | environment | filesystem | network | randomness | Enforcement owner | Availability |', 11);
  for (const row of capabilities) {
    const accepted = composition.find(item => item[0] === row[0]);
    assert(accepted, row[0]);
    assert.deepEqual(row.slice(1, 6), accepted.slice(4, 9), row[0]);
  }
  assert.deepEqual(boundaries.map(row => row[0]), capabilities.map(row => row[0]));
  assert(boundaries.every(row => row[3] === 'specified-only'));
  assert.deepEqual(wit.capabilityOrder, ['clock', 'environment', 'filesystem', 'network', 'randomness']);
  assert(contract.includes(`\`${wit.wit.package}\``));
  assert(contract.includes(`WASI \`${wit.wasi.version}\``));
  assert(contract.includes(wit.wit.grammarCommit));
  assert(proof.includes(wit.wit.grammarSource));
  for (const profile of wit.profiles) {
    const id = `WIT-${profile.id.toUpperCase()}`;
    assert(boundaries.find(row => row[0] === id)[1].includes(profile.world));
    assert.deepEqual(capabilities.find(row => row[0] === id).slice(1, 6),
      profile.capabilities.map(item => item.decision === 'granted' ? 'E' : 'D'));
  }
  assert.equal(capabilities.filter(row => row[0].startsWith('WIT-')).length, wit.profiles.length);
  for (const [id, ceiling] of [['JS-BROWSER', ['E', 'D', 'D', 'E', 'E']],
    ['JS-NODE', ['E', 'E', 'E', 'E', 'E']]]) {
    const row = capabilities.find(item => item[0] === id);
    assert.deepEqual(row.slice(1, 6), ceiling);
    assert.match(row[6], /No effectful JS operation admitted in v1; nonempty requests reject/);
  }
});

test('operations identify ownership and lifecycle outcomes distinguish JS disposal from canonical drop', () => {
  const operations = table(contract,
    '| Operation | Input -> output representation | Allocator / owner after success | Borrow interval | Failure and cleanup responsibility |', 5);
  assert.deepEqual(operations.map(row => row[0]), ['scalar call', 'copy-in', 'copy-out',
    'create-resource', 'use-resource', 'release-resource', 'teardown-instance']);
  const states = table(contract,
    '| State / operation | ESM outcome | Typed WIT / canonical outcome | Owner / cleanup |', 4);
  const release = states.find(row => row[0] === 'released / release');
  assert.match(release[1], /idempotent no-op/);
  assert.match(release[2], /second owner drop is invalid/);
  assert.match(release[3], /no second payload free/);
  assert.match(contract, /recycled integer alone cannot prove freshness/);
  assert.match(contract, /Do not assume post-return runs after failed lifting/);
  assert.match(proof, /fatal failure, the oracle is instance invalidation plus embedding-owned reclamation/);
});

test('fixed Unicode vectors independently preserve bytes, NUL, BOM and non-normalized sequences', () => {
  assert.deepEqual(vectors.unicode.map(item => item.id),
    ['empty', 'nul', 'bom', 'accent', 'supplementary', 'decomposed']);
  for (const { id, text, utf8 } of vectors.unicode) {
    assert(text.isWellFormed(), id);
    assert(utf8.every(byte => Number.isInteger(byte) && byte >= 0 && byte <= 255), id);
    assert.deepEqual([...new TextEncoder().encode(text)], utf8, id);
    const decoded = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true })
      .decode(Uint8Array.from(utf8));
    assert.equal(decoded, text, id);
  }
  const accent = vectors.unicode.find(item => item.id === 'accent');
  const decomposed = vectors.unicode.find(item => item.id === 'decomposed');
  assert.notEqual(accent.text, decomposed.text);
  assert.equal(decomposed.text.normalize('NFC'), accent.text);
});

test('independent malformed byte and surrogate vectors require strict encoding rejection', () => {
  assert.deepEqual(vectors.invalidUtf8.map(item => item.id),
    ['overlong', 'truncated', 'surrogate', 'above-unicode-limit', 'isolated-continuation']);
  for (const { id, bytes } of vectors.invalidUtf8) {
    assert(bytes.every(byte => Number.isInteger(byte) && byte >= 0 && byte <= 255), id);
    assert.throws(() => new TextDecoder('utf-8', { fatal: true })
      .decode(Uint8Array.from(bytes)), TypeError, id);
  }
  assert.deepEqual(vectors.invalidUtf16.map(item => item.id), ['lone-high', 'lone-low']);
  for (const { id, text } of vectors.invalidUtf16) {
    assert.equal(text.isWellFormed(), false, id);
    assert.notEqual(new TextDecoder().decode(new TextEncoder().encode(text)), text, id);
  }
});

test('future negative cases name fixed outcomes and retain independent consumer and activation gates', () => {
  const outcomes = table(proof, '| Outcome | Phase / meaning | Recovery and observables |', 3);
  const cases = table(proof, '| Case | Input / action | Fixed expected outcome |', 3);
  for (const [id, outcome] of [
    ['scalar-invalid', 'invalid-value'], ['surrogate-input', 'encoding'],
    ['input-prefix-failure', 'allocation'], ['output-copy-failure', 'allocation'],
    ['create-publish-failure', 'allocation'], ['malformed-result', 'fatal-boundary'],
    ['canonical-allocation-trap', 'fatal-boundary'], ['post-return-trap', 'fatal-boundary'],
  ]) {
    assert(outcomes.some(row => row[0] === outcome));
    assert(cases.find(row => row[0] === id)[2].includes(outcome), id);
  }
  const lifecycle = table(proof, '| Case | Trace / adversarial action | Fixed expected outcome |', 3);
  for (const id of ['stale-use', 'stale-after-reuse', 'double-dispose-js', 'double-drop-wit',
    'wrong-instance', 'forged-resource', 'borrow-escape', 'reentry', 'destructor-failure',
    'callback-input', 'async-output', 'worker-transfer', 'teardown']) {
    assert(lifecycle.some(row => row[0] === id), id);
  }
  const slices = table(proof, '| Slice | Exact prerequisites | Independent completion evidence |', 3);
  assert.deepEqual(slices.map(row => row[0].split(' ')[0]), ['I1', 'I2', 'I3', 'I4', 'C1', 'A1']);
  assert.match(proof, /empty pinned browser world cannot export `add`/);
  assert.match(proof, /real browser; Node's WebAssembly API is not that/);
  assert.match(proof, /not generated bindings or runtime execution evidence/);
});

test('private scalar implementation ledger preserves later consumer and target gates', () => {
  const ledger = table(proof, '| Boundary | State | Exact implemented evidence | Still separate |', 4);
  assert.deepEqual(ledger.map(row => row[0]),
    ['Private scalar ESM interface', 'Private scalar host consumer', 'Non-scalar ESM conversions', 'WIT/Component adapters']);
  assert.equal(ledger[0][1], 'implemented-private by #381');
  assert.match(ledger[0][2], /byte-compared deterministic ESM/);
  assert.match(ledger[0][3], /real browser\/Node consumer conformance/);
  assert.equal(ledger[1][1], 'local candidate by #387; conformance unrun');
  assert.match(ledger[1][3], /reviewed browser archive\/inventory pins/);
  assert(ledger.slice(2).every(row => row[1] === 'specified-only'));
  assert.match(proof, /unsupported core-WebAssembly, native, WIT and\s+Component target claims are rejected/);
  assert.match(proof, /do not imply Windows or macOS native ABI decisions/);
});

test('proposed bounds and accepted library alignment preserve remaining host admission gates', async () => {
  const bounds = table(proof, '| Metric | Maximum | Exact-limit / first-extra oracle |', 3);
  assert.deepEqual(bounds.map(row => Number(row[1].replaceAll(',', ''))),
    [1_048_576, 65_536, 4_194_304, 8_388_608, 1_024, 65_536, 1]);
  assert(bounds.every(row => /extra|1,025|65,537|second entry/.test(row[2])));
  const alignment = table(proof, '| Library APIs | Adapter alignment decision | Remaining implementation gate |', 3);
  assert.deepEqual(alignment.map(row => row[0]),
    ['C1-C3', 'A1-A5', 'H1', 'H2', 'H3', 'H4', 'H5', 'other conversion rows']);
  assert.match(proof, /library\/adapter design alignment is complete/);
  assert.match(alignment.find(row => row[0] === 'H3')[2], /F4 and an adapter\/API revision/);
  assert.match(alignment.find(row => row[0] === 'H4')[2], /F1\/F2\/F3/);
  assert.match(alignment.find(row => row[0] === 'H5')[2], /every element 0\.\.255/);
  const library = await read('spec/libraries/MINIMAL_CORE_HOST_V0.md');
  assert.match(library, /State: \*\*specified-only;/);
  const apis = table(library,
    '| API | Layer | Operation and exact v0 behavior | Language gates | Profiles | Ownership | Errors | Oracle |', 8);
  const oracles = table(proof, '| Library oracle | Exact later consumer proof |', 2);
  const libraryOracles = table(library, '| Oracle | Fixed input and exact required observation |', 2);
  for (const [index, id] of ['H1', 'H2', 'H3', 'H4', 'H5'].entries()) {
    const api = apis.find(row => row[0] === id);
    assert.equal(api[5], 'O4', id);
    assert.equal(api[6], 'E3', id);
    assert.equal(api[7], `Q${index + 9}`, id);
    assert(oracles.some(row => row[0] === `${id} / ${api[7]}`), id);
    assert(libraryOracles.some(row => row[0] === api[7]), id);
  }
  for (const [id, expected] of [['H1', '0-1024 bytes'], ['H2', 'at most 4096 bytes'],
    ['H3', '100..599'], ['H4', 'unsigned 64-bit'], ['H5', 'count i32 in 0..256']]) {
    assert(apis.find(row => row[0] === id)[2].includes(expected), id);
  }
  for (const status of ['invalid-input', 'not-found', 'permission-denied', 'quota-exceeded',
    'invalid-encoding', 'io-failure']) {
    assert(library.includes(status) && proof.includes(`\`${status}\``), status);
  }
  assert.match(proof, /failure after an external effect\s+does not undo it and causes no automatic retry/);
  assert.match(proof, /malformed data at an interface\s+declared as text instead triggers the adapter's fatal invariant-failure rule/);
  assert.match(proof, /not changes to M3 limits or #167 quotas/);
  assert.match(contract, /Source packages that still compile are not\s+necessarily compatible binaries/);
});
