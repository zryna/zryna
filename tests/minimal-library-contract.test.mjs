import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const read = file => readFile(new URL(`../${file}`, import.meta.url), 'utf8');
const document = await read('spec/libraries/MINIMAL_CORE_HOST_V0.md');
const registry = JSON.parse(await read('tests/wit-capability-profiles-v1.json'));
const composition = new Map(rows(await read('spec/language/CROSS_TARGET_PROFILES_V1.md'),
  /^(U-JS|U-WASM|U-NATIVE|JS-BROWSER|JS-NODE|WIT-BROWSER|WIT-COMMAND|WIT-SERVER|NATIVE-HOST)$/)
  .map(row => [row[0], row.slice(4, 9)]));
const expectedIds = ['C1', 'C2', 'C3', 'A1', 'A2', 'A3', 'A4', 'A5',
  'H1', 'H2', 'H3', 'H4', 'H5'];

function rows(markdown, firstColumn) {
  return markdown.split(/\r?\n/).filter(line => line.startsWith('| '))
    .map(line => line.slice(1, -1).split('|').map(cell => cell.trim()))
    .filter(cells => firstColumn.test(cells[0]));
}

function validateMatrix(markdown, witRegistry) {
  const apis = rows(markdown, /^[CAH]\d+$/);
  assert.deepEqual(apis.map(row => row[0]), expectedIds, 'closed API inventory');
  const gates = new Set(rows(markdown, /^[LF]\d+$/).map(row => row[0]));
  const profiles = new Map(rows(markdown, /^P\d+$/).map(row => [row[0], row[1].split(', ')]));
  const owners = new Set(rows(markdown, /^O\d+$/).map(row => row[0]));
  const errors = new Set(rows(markdown, /^E\d+$/).map(row => row[0]));
  const oracles = new Set(rows(markdown, /^Q\d+$/).map(row => row[0]));
  assert.deepEqual([...gates], ['L1', 'L2', 'L3', 'F1', 'F2', 'F3', 'F4', 'F5', 'F6']);
  assert.equal(oracles.size, 13, 'one fixed oracle per API');
  assert.deepEqual(profiles.get('P0'), ['U-JS', 'U-WASM', 'U-NATIVE']);
  for (const host of profiles.get('P0')) {
    assert.deepEqual(composition.get(host), ['D', 'D', 'D', 'D', 'D']);
  }
  const requirements = {
    H1: ['environment', ['L2', 'F1', 'F2']],
    H2: ['filesystem', ['L2', 'F1', 'F2']],
    H3: ['network', ['L2', 'F1', 'F2']],
    H4: ['clock', ['F1', 'F2', 'F3']],
    H5: ['randomness', ['L3', 'F1', 'F2']],
  };
  const allowedHostRows = new Set(['JS-BROWSER', 'JS-NODE', 'WIT-BROWSER',
    'WIT-COMMAND', 'WIT-SERVER', 'NATIVE-HOST']);
  for (const [index, row] of apis.entries()) {
    assert.equal(row.length, 8, 'complete API columns');
    const [id, layer, operation, language, profile, owner, error, oracle] = row;
    assert(operation.length > 20, `${id} concrete operation`);
    const prerequisites = language.split(', ');
    assert(prerequisites.every(gate => gates.has(gate)), `${id} known language gates`);
    assert(profiles.has(profile), `${id} known profiles`);
    assert(owners.has(owner), `${id} known ownership`);
    assert(errors.has(error), `${id} known errors`);
    assert.equal(oracle, `Q${index + 1}`, `${id} distinct oracle`);
    assert(oracles.has(oracle), `${id} fixed oracle`);
    if (id.startsWith('H')) {
      assert.equal(layer, 'host');
      assert.equal(owner, 'O4');
      assert.equal(error, 'E3');
      const [capability, requiredGates] = requirements[id];
      assert.deepEqual(prerequisites, requiredGates, `${id} future prerequisites`);
      for (const host of profiles.get(profile)) {
        assert(allowedHostRows.has(host), `${id} known host row`);
        const column = ['clock', 'environment', 'filesystem', 'network', 'randomness'].indexOf(capability);
        assert.equal(composition.get(host)?.[column], 'E', `${id} accepted composition ceiling`);
        if (!host.startsWith('WIT-')) continue;
        const world = witRegistry.profiles.find(item => item.id === host.slice(4).toLowerCase());
        assert(world, `${id} pinned world`);
        const permission = world.capabilities.find(item => item.id === capability);
        assert.equal(permission?.decision, 'granted', `${id} pinned capability ceiling`);
        if (id === 'H3') {
          assert(permission.interfaces.includes('wasi:http/outgoing-handler@0.2.12'),
            'HTTP requires the exact outgoing interface, not just network eligibility');
        }
      }
    } else {
      assert.equal(profile, 'P0', `${id} no host capability`);
      assert.equal(layer, id.startsWith('C') ? 'pure' : 'allocation');
      assert.deepEqual(prerequisites, [id.startsWith('C') ? 'L1' :
        ['A1', 'A2'].includes(id) ? 'L2' : 'L3']);
      assert.equal(owner, id.startsWith('C') ? 'O1' : ['A4', 'A5'].includes(id) ? 'O3' : 'O2');
      assert.equal(error, id.startsWith('C') ? 'E0' : id === 'A4' ? 'E2' : 'E1');
    }
  }
}

test('minimal library matrix closes prerequisites, ownership, errors and pinned WIT eligibility', () => {
  validateMatrix(document, registry);
});

test('minimal library matrix rejects missing gates, ambient widening and mismatched interfaces', () => {
  for (const [from, to] of [
    ['| C1 | pure |', '| C1 | host |'],
    ['| L1 | P0 | O1 | E0 | Q1 |', '| L1 | P1 | O1 | E0 | Q1 |'],
    ['| L2, F1, F2 | P1 |', '| L2 | P1 |'],
    ['| F1, F2, F3 | P4 |', '| F1, F2 | P4 |'],
    ['| L3 | P0 | O3 | E2 | Q7 |', '| L3 | P0 | O3 | E2 | Q99 |'],
    ['| P1 | JS-NODE, WIT-COMMAND, NATIVE-HOST |', '| P1 | JS-NODE, WIT-SERVER, NATIVE-HOST |'],
    ['| P2 | JS-NODE, WIT-COMMAND, NATIVE-HOST |', '| P2 | JS-BROWSER |'],
    ['| P3 | JS-BROWSER, JS-NODE, WIT-SERVER, NATIVE-HOST |',
      '| P3 | JS-BROWSER, JS-NODE, WIT-COMMAND, NATIVE-HOST |'],
    ['| P5 | JS-BROWSER, JS-NODE, WIT-COMMAND, WIT-SERVER, NATIVE-HOST |',
      '| P5 | WIT-BROWSER |'],
  ]) {
    const changed = document.replace(from, to);
    assert.notEqual(changed, document, 'mutation must affect the matrix');
    assert.throws(() => validateMatrix(changed, registry));
  }
  assert.throws(() => validateMatrix(document.replace(/^\| A5 \|.*\r?\n/m, ''), registry));
});

test('minimal library scalar examples retain independently fixed signed and Boolean oracles', () => {
  const oracles = new Map(rows(document, /^Q[123]$/).map(row => [row[0], row[1]]));
  const expected = {
    Q1: ['i32:42', 'i32:-2147483648', 'i32:2147483647'],
    Q2: ['i32:-3', 'i32:-2147483648', 'i32:5'],
    Q3: ['i32:17', 'i32:-9'],
  };
  for (const [id, values] of Object.entries(expected)) {
    assert.deepEqual(oracles.get(id).match(/i32:-?\d+/g), values);
  }
  for (const target of ['javascript / pinned Node', 'webassembly / pinned Node',
    'native / supported Linux', 'native run / Windows']) assert(document.includes(target));
  assert.match(document, /ZRYNA-N4002 rejection and no bundle/);
  assert.match(document, /planned conformance fixtures, not executed results/);
  assert.match(document, /returns `i32:42`/);
});

test('minimal library exclusions and rollout preserve current language and public ABI boundaries', async () => {
  const negatives = rows(document, /^N\d+$/);
  assert.deepEqual(negatives.map(row => row[0]), Array.from({ length: 11 }, (_, i) => `N${i + 1}`));
  for (const phrase of ['console.log', 'process.env', 'Date.now', 'Math.random', 'fetch',
    'node:fs', 'Option/Result', 'JSON.parse', 'async/await', 'borrowed import']) {
    assert(negatives.some(row => row.join(' ').includes(phrase)), phrase);
  }
  for (const phrase of ['P0-P5 mapping aligned with accepted PR #374', 'no library implementation or public activation',
    'Ending a borrow never frees its owner', 'JSON remains outside', 'separate M7',
    'before O4', 'document checks are not execution', 'no effectful JS operation',
    'no automatic retry', 'retains the original outcome', 'do not assume post-return runs',
    'ca6307b122e20b071728914a6fdc2d1e7418f083',
    'e639e3ea82866a53cbe22a958aa9d5d7ff823d17', 'd07c070e595bc8ccf738a43627bbd7031cb98bff']) {
    assert(document.toLowerCase().includes(phrase.toLowerCase()), phrase);
  }
  const publicStatus = await read('docs/M3_PUBLIC_PROFILE.md');
  assert.match(publicStatus, /Entry exports accept and return only exact `i32`\/`bool`/);
  const language = await read('spec/language/DATA_OWNERSHIP_V1.md');
  assert.match(language, /user-defined generic declarations are not enabled/);
  assert.match(language, /There are no user-defined destructors/);
});

test('minimal library contract is reachable from M4 and keeps relative links valid', async () => {
  const roadmap = await read('docs/ROADMAP.md');
  const m4 = roadmap.split('## M4 — WebAssembly Components and WASI')[1].split('## M5')[0];
  assert.match(m4, /Proposed bounded M4 scope extension/);
  assert.match(m4, /spec\/libraries\/MINIMAL_CORE_HOST_V0\.md/);
  const links = [...document.matchAll(/\[[^\]\n]+\]\(([^)\n]+)\)/g)]
    .map(([, link]) => link).filter(link => !link.startsWith('https:'));
  assert.deepEqual(links.map(link => link.split('#')[0]).sort(), [
    '../../docs/M3_PUBLIC_PROFILE.md', '../../docs/ROADMAP.md',
    '../../tests/minimal-library-contract.test.mjs', '../abi/OWNERSHIP_RUNTIME_V1.md',
    '../abi/SCALAR_V1.md', '../language/CONTROL_FLOW_MODULES_V1.md',
    '../language/CROSS_TARGET_PROFILES_V1.md',
    '../language/DATA_OWNERSHIP_V1.md', '../wit/CAPABILITY_PROFILES_V1.md',
  ].sort());
  for (const link of links) await readFile(new URL(link.split('#')[0],
    new URL('../spec/libraries/MINIMAL_CORE_HOST_V0.md', import.meta.url)), 'utf8');
});
