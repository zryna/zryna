import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { reconcileRequiredOwnerLogs, parseRequiredDeclarations, checkRequiredMaterials } from '../scripts/stability-gates/required-owner-logs.mjs';
import { index, fixture, readMaterial, transcript } from './stability-required-owner-support.mjs';
const run = f => reconcileRequiredOwnerLogs(index, f.observation, f.context);
const validFixture = (...args) => { const f = fixture(...args); run(f); return f; };
test('synthetic Linux ABI join preserves raw ignored3 and remains blocked/unauthenticated', () => {
  const f = validFixture(), result = run(f);
  assert.deepEqual(result.rawDefaultTargets[0].counts, { passed: 1, failed: 0, ignored: 3, measured: 0, filtered: 0 });
  assert.equal(result.coverageCardinality, 3);
  assert.deepEqual(result.requiredNames.map(row => row.fullTestName).sort(), index.rows.filter(row => row.package === 'zryna-ownership-runtime-abi').map(row => row.fullTestName).sort()); assert.equal(result.rawIgnoredSubtracted, 0);
  assert.equal(result.ownerTargets[0].counts.filtered, 91);
  assert.equal(result.proofQualified, false); assert.equal(result.sourceAuthenticated, false);
  assert.equal(result.closureStatus, 'blocked'); assert.equal(result.thresholds, null);
});
test('synthetic Windows exact ABI owner is distinct and Linux cannot stand in for Windows', () => {
  assert.equal(run(validFixture('m7-windows-abi', 'windows-x86_64')).coverageCardinality, 3);
  const f = validFixture(); f.context.host = 'windows-x86_64'; assert.throws(() => run(f));
});
test('parser provider requires separate target blocks with all3 plus1 exact names', () => {
  const f = validFixture('native-parser-provider'); assert.equal(run(f).coverageCardinality, 4);
  assert.equal(run(f).ownerTargets.length, 2);
  const all = f.observation.targets.flatMap(section => index.rows.filter(row => row.package === section.package && row.target === section.target).map(row => ({ name: row.fullTestName, outcome: 'ok' })));
  f.rewrite(f.observation.targets[0].stdout, transcript(all)); f.observation.targets.splice(1); assert.throws(() => run(f));
});
test('missing/short required name and unrelated nonzero suite cannot substitute', () => {
  for (const mode of ['missing', 'short', 'unrelated']) {
    const f = validFixture(), rows = index.rows.filter(row => row.package === 'zryna-ownership-runtime-abi').map(row => ({ name: row.fullTestName, outcome: 'ok' }));
    if (mode === 'missing') rows.pop();
    if (mode === 'short') rows[0].name = rows[0].name.split('::').at(-1);
    if (mode === 'unrelated') rows.splice(0, rows.length, { name: 'unrelated', outcome: 'ok' });
    f.rewrite(f.observation.targets[0].stdout, transcript(rows)); assert.throws(() => run(f));
  }
});
test('zero-filter/listed-only, ignored and duplicate proof rows reject', () => {
  const name = index.rows.find(row => row.package === 'zryna-ownership-runtime-abi').fullTestName;
  for (const text of [transcript([], 300), `${name}: test\n0 tests, 0 benchmarks\n`, transcript([{ name, outcome: 'ignored' }]), transcript([{ name, outcome: 'ok' }, { name, outcome: 'ok' }])]) {
    const f = validFixture(); f.rewrite(f.observation.targets[0].stdout, text); assert.throws(() => run(f));
  }
});
test('default unknown ignored/missing declaration and rewriting raw ignores reject', () => {
  for (const mode of ['unknown', 'missing', 'rewritten']) {
    const f = validFixture(); let text = f.logs.get(f.observation.defaults[0].stdout.path).toString();
    if (mode === 'unknown') text = text.replace('tests::record_declaration_budget_is_exact_and_plus_one', 'unknown_ignored_case');
    if (mode === 'missing') text = transcript([{ name: 'ordinary_nonrequired_control', outcome: 'ok' }]);
    if (mode === 'rewritten') text = text.replace('3 ignored', '0 ignored');
    f.rewrite(f.observation.defaults[0].stdout, text); assert.throws(() => run(f));
  }
});
test('source-role/revision/tree, host, owner, argv and process outcomes stay exact', () => {
  const changes = [f => { f.observation.source.role = 'tested-pr-head'; }, f => { f.observation.source.revision = '3'.repeat(40); }, f => { f.observation.source.tree = '3'.repeat(40); }, f => { f.observation.host = 'windows-x86_64'; }, f => { f.observation.owner = 'unknown'; }, f => { f.observation.argv.push('--list'); }, f => { f.observation.argv = f.observation.argv.filter(x => x !== '--locked'); }, f => { f.observation.exitCode = 1; }, f => { f.observation.signal = 'SIGKILL'; }, f => { f.observation.error = 'ETIMEDOUT'; }];
  for (const change of changes) { const f = validFixture(); change(f); assert.throws(() => run(f)); }
});
test('log size/hash/missing stderr, invalid UTF8 and unparsed/footer/nested blocks reject', () => {
  const changes = [f => { f.observation.targets[0].stdout.size++; }, f => { f.observation.targets[0].stdout.sha256 = 'f'.repeat(64); }, f => { delete f.observation.targets[0].stderr; }, f => { f.rewrite(f.observation.targets[0].stdout, 'running 0 tests\ntest result: ok. truncated\n'); }, f => { const d = f.observation.targets[0].stdout; f.rewrite(d, f.logs.get(d.path).toString() + transcript([], 4)); }, f => { const d = f.observation.targets[0].stdout; f.rewrite(d, Buffer.from([0xff])); }];
  for (const change of changes) { const f = validFixture(); change(f); assert.throws(() => run(f)); }
});
test('unknown target, duplicate execution section and other-target required row reject', () => {
  const f = validFixture('native-parser-provider'); f.observation.targets[0].target = 'unknown'; assert.throws(() => run(f));
  const g = validFixture('native-parser-provider'); g.observation.targets[1] = g.observation.targets[0]; assert.throws(() => run(g));
  const z = validFixture('native-parser-provider'), one = z.observation.targets.find(row => row.target === 'native_parser_v4_provider');
  const names = index.rows.filter(row => row.package === one.package && row.target === one.target).map(row => ({ name: row.fullTestName, outcome: 'ok' }));
  names.push({ name: 'pinned_provider_matches_frozen_diagnostic_corpus', outcome: 'ok' }); z.rewrite(one.stdout, transcript(names)); assert.throws(() => run(z));
});
test('Linux native-source and lifecycle remain unresolved, not waived or assigned Windows receipts', () => {
  assert.throws(() => run(fixture('native-source-resource', 'linux-x86_64')));
  const f = validFixture(); f.observation.owner = 'installed-lifecycle'; assert.throws(() => run(f));
});
test('fixed declaration/material identity and blocked/no-auth output cannot be promoted', () => {
  const bytes = readFileSync(new URL('./stability-required-ignored-v2.json', import.meta.url));
  assert.equal(parseRequiredDeclarations(bytes).rows.length, 28);
  const changed = structuredClone(index); changed.thresholds = { maximum: 1 }; assert.throws(() => checkRequiredMaterials(changed, readMaterial));
  assert.throws(() => checkRequiredMaterials(index, () => Buffer.from('changed source')));
  const f = validFixture(); f.observation.proofQualified = true; assert.throws(() => run(f));
});

test('failed/measured/footer discrepancies reject despite matching printed proof names', () => {
  for (const mutate of [text => text.replace('test result: ok.', 'test result: FAILED.'), text => text.replace('0 measured', '1 measured'), text => text.replace('3 passed', '4 passed')]) {
    const f = validFixture(), d = f.observation.targets[0].stdout;
    f.rewrite(d, mutate(f.logs.get(d.path).toString())); assert.throws(() => run(f));
  }
});
