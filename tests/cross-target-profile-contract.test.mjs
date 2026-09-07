import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const root = new URL('../', import.meta.url);
const document = await readFile(new URL('spec/language/CROSS_TARGET_PROFILES_V1.md', root), 'utf8');

function table(header) {
  const lines = document.split(/\r?\n/);
  const start = lines.indexOf(header);
  assert(start >= 0, `missing table: ${header}`);
  const rows = [];
  for (let index = start + 2; lines[index]?.startsWith('|'); index += 1) {
    rows.push(lines[index].split('|').slice(1, -1).map(cell => cell.trim()));
  }
  assert(rows.length > 0);
  return rows;
}

const decisions = table('| Row | Output target | Language admission | Deployment boundary | clock | environment | filesystem | network | randomness | Enforcement owner | Availability |');

test('composition table covers each target and host boundary with explicit ceilings and owners', () => {
  assert.match(document, /Contract identity: `zryna\.cross-target-profiles\.v1`\. State: \*\*specified-only\*\*/);
  assert.deepEqual(decisions.map(row => row[0]), [
    'U-JS', 'U-WASM', 'U-NATIVE', 'JS-BROWSER', 'JS-NODE',
    'WIT-BROWSER', 'WIT-COMMAND', 'WIT-SERVER', 'NATIVE-HOST', 'UNSUPPORTED',
  ]);
  for (const row of decisions) {
    assert.equal(row.length, 11, row[0]);
    assert(row[2].length > 0 && row[3].length > 0 && row[9].length > 0, row[0]);
    assert(row.slice(4, 9).every(value => value === 'D' || value === 'E'), row[0]);
    if (row[0].startsWith('U-') || row[0] === 'UNSUPPORTED') {
      assert.deepEqual(row.slice(4, 9), ['D', 'D', 'D', 'D', 'D']);
    } else {
      assert.equal(row[10], 'specified-only', row[0]);
    }
  }
});

test('WIT composition rows and split environment boundaries agree with the owning contract', async () => {
  // Read the owning registry: a missing prerequisite or changed policy must fail this check.
  const wit = JSON.parse(await readFile(new URL('tests/wit-capability-profiles-v1.json', root), 'utf8'));
  assert.deepEqual(wit.capabilityOrder, ['clock', 'environment', 'filesystem', 'network', 'randomness']);
  for (const profile of wit.profiles) {
    const row = decisions.find(candidate => candidate[0] === `WIT-${profile.id.toUpperCase()}`);
    assert(row, profile.id);
    assert.deepEqual(row.slice(4, 9), profile.capabilities.map(capability => {
      assert(['denied', 'granted'].includes(capability.decision));
      return capability.decision === 'granted' ? 'E' : 'D';
    }));
  }
  assert.equal(wit.profiles.length, decisions.filter(row => row[0].startsWith('WIT-')).length);
  assert(document.includes(`\`${wit.wit.package}\``));
  assert(document.includes(`WASI \`${wit.wasi.version}\``));
  const environment = wit.profiles.find(profile => profile.id === 'command')
    .capabilities.find(capability => capability.id === 'environment');
  const boundaries = table('| Metric | Exact-limit fixture | First-extra fixture / outcome |');
  for (const [metric, limit] of [
    ['command environment entries', environment.limits.maxEntries],
    ['command environment bytes', environment.limits.maxTotalBytes],
  ]) {
    const row = boundaries.find(candidate => candidate[0] === metric);
    const decimal = value => String(value).replace(/\B(?=(\d{3})+(?!\d))/g, ',');
    assert(row[1].includes(decimal(limit)), metric);
    assert(row[2].includes(decimal(limit + 1)), metric);
  }
});

test('future examples reference defined diagnostic categories without claiming executed conformance', () => {
  const categories = table('| Category | Expected phase | Required explanation |');
  const cases = table('| Case | Input / change | Fixed expected outcome |');
  assert.equal(new Set(cases.map(row => row[0])).size, cases.length);
  assert(cases.some(row => row[0] === 'pure-chain' && row[2].includes('i32:42')));
  for (const id of ['indirect-forbidden', 'incompatible-language', 'unsupported-native', 'omitted-grant', 'replay']) {
    const row = cases.find(candidate => candidate[0] === id);
    assert(row && categories.some(category => row[2].includes(category[0])), id);
  }
  assert.match(document, /Examples below are future fixtures/);
  assert.match(document, /not allocated public codes/);
});
