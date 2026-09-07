import assert from 'node:assert/strict';
import { readFileSync, mkdtempSync, mkdirSync, writeFileSync, rmSync, cpSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { test } from 'node:test';
import { parseDocument } from 'yaml';
import { validateM3Registry, loadAndValidateM3Conformance, workspaceRoot } from '../scripts/check-m3-conformance.mjs';
import { QUICK, FULL, validateCommands, runCommand } from '../scripts/lib/m3-gates.mjs';

const bytes = readFileSync(resolve(workspaceRoot, 'tests/m3-conformance-v1.json'));
const workflow = parseDocument(readFileSync(resolve(workspaceRoot, '.github/workflows/ci.yml'), 'utf8'));
assert.deepEqual(workflow.errors, []);

function validateWorkflow(candidate) {
  const jobs = candidate.jobs;
  const platform = jobs['m3-platform'];
  assert.deepEqual(Object.keys(platform).sort(), ['name', 'runs-on', 'steps', 'strategy', 'timeout-minutes']);
  assert.equal(platform.name, 'm3 (${{ matrix.os }})');
  assert.equal(platform['runs-on'], '${{ matrix.os }}');
  assert.equal(platform['timeout-minutes'], 30);
  assert.deepEqual(platform.strategy, { 'fail-fast': false, matrix: { os: ['ubuntu-latest', 'windows-latest'] } });
  assert.deepEqual(platform.steps, jobs['m2-platform'].steps.map(step =>
    step.run === 'pnpm m2:check' ? { run: 'pnpm m3:check' } : step));
  assert.deepEqual(jobs.m3, {
    name: 'm3', if: 'always()', needs: ['m0', 'm2', 'm3-platform'], 'runs-on': 'ubuntu-latest',
    steps: [{ name: 'Verify complete M3 gate', env: {
      M0_RESULT: '${{ needs.m0.result }}', M2_RESULT: '${{ needs.m2.result }}',
      PLATFORM_RESULT: '${{ needs.m3-platform.result }}',
    }, run: 'test "$M0_RESULT" = success && test "$M2_RESULT" = success && test "$PLATFORM_RESULT" = success' }],
  });
  assert.deepEqual(jobs.m2.needs, ['m0', 'm2-platform']);
  assert.deepEqual(jobs.m0.needs,
    ['owned-data-quick', 'preflight', 'rust', 'adapter', 'route-contracts']);
  const pkg = JSON.parse(readFileSync(resolve(workspaceRoot, 'package.json')));
  for (const [name, file] of [['quick', 'quick'], ['check', 'conformance']]) {
    assert.equal(pkg.scripts[`m3:${name}`], `node scripts/run-m3-${file}.mjs`);
  }
  assert.equal(pkg.scripts['m3:registry'], 'node scripts/check-m3-conformance.mjs && node --test tests/m3-conformance.test.mjs');
}

test('fixed registry binds observations and phase-owned oracles', () => {
  const registry = loadAndValidateM3Conformance();
  assert.equal(registry.valid.length, 15); assert.equal(registry.invalid.length, 4);
  assert.equal(registry.evidence.length, 18);
  assert.equal(registry.faults.length, 26);
  assert.equal(registry.runtimeInvalid.length, 2);
  assert.deepEqual(registry.targetOrder, ['javascript', 'webassembly', 'native']);
});
test('removed, duplicated, substituted and self-derived oracle claims reject', () => {
  for (const mutate of [
    r => r.faults.pop(), r => r.faults.push(r.faults[0]),
    r => { r.faults[0].expected.code = 'ZRYNA-R3006'; },
    r => { r.faults[1].trace.pop(); }, r => { r.faults[1].trace[0].place += 1; },
    r => { r.faults[1].trace.reverse(); }, r => { r.faults[1].fault.ordinal += 1; },
    r => r.valid.pop(), r => r.valid.push(r.valid[0]), r => r.invalid.pop(),
    r => { r.valid[0].expected = 66; }, r => { r.invalid[0].phase = 'execution'; },
    r => { r.invalid[0].code = 'ZRYNA-M9999'; }, r => r.evidence.pop(),
    r => { r.fixtures[0].path = '../outside.zry'; }, r => r.fixtures.reverse(),
    r => { r.valid[0].expected = 'javascript-output'; }, r => { r.unknown = true; },
  ]) {
    const changed = JSON.parse(bytes); mutate(changed);
    assert.throws(() => validateM3Registry(JSON.stringify(changed)), /frozen oracle/);
  }
  assert.throws(() => validateM3Registry(Buffer.alloc(65537)), /bounded registry/);
});
test('changed, missing and unregistered fixture bytes reject', () => {
  const root = mkdtempSync(resolve(tmpdir(), 'zryna-m3-registry-'));
  try {
    const dir = resolve(root, 'tests/m3-fixtures/conformance');
    mkdirSync(resolve(root, 'tests/m3-fixtures'), { recursive: true });
    cpSync(resolve(workspaceRoot, 'tests/m3-fixtures/conformance'), dir, { recursive: true });
    const pair = resolve(dir, 'pair.zry'); writeFileSync(pair, 'changed');
    assert.throws(() => validateM3Registry(bytes, root), /pair/);
    rmSync(pair); assert.throws(() => validateM3Registry(bytes, root), /ENOENT/);
    cpSync(resolve(workspaceRoot, 'tests/m3-fixtures/conformance/pair.zry'), pair);
    writeFileSync(resolve(dir, 'unregistered.zry'), '');
    assert.throws(() => validateM3Registry(bytes, root), /unregistered/);
  } finally { rmSync(root, { recursive: true, force: true }); }
});
test('quick and full commands cannot omit or weaken evidence', () => {
  for (const [commands, full] of [[QUICK, false], [FULL, true]]) {
    validateCommands(commands, full);
    for (const changed of [commands.slice(1), [...commands].reverse(), [...commands, commands[0]]]) {
      assert.throws(() => validateCommands(changed, full), /frozen inventory/);
    }
    const changed = structuredClone(commands); changed[0].args.push('--skip');
    assert.throws(() => validateCommands(changed, full), /frozen inventory/);
  }
  for (const id of ['ownership-ir-boundaries', 'ownership-source-boundaries']) {
    assert(FULL.some(c => c.id === id && c.args.includes('--include-ignored')));
  }
});
test('failed spawn, signal, nonzero status and zero-test filters reject', () => {
  const entry = { id: 'negative', executable: 'cargo', args: ['test'], timeout: 1 };
  for (const result of [
    { error: new Error('timeout') }, { status: null, signal: 'SIGTERM' }, { status: 1 },
    { status: 0, stdout: 'test result: ok. 0 passed; 0 failed;' },
  ]) assert.throws(() => runCommand(entry, () => result));
  runCommand(entry, (_exe, _args, options) => {
    assert.equal(options.shell, false); assert.equal(options.windowsHide, true);
    return { status: 0, stdout: 'test result: ok. 1 passed; 0 failed;' };
  });
});
test('Linux/Windows aggregate retains M0-M2 and rejects bypasses', () => {
  validateWorkflow(workflow.toJS());
  for (const mutate of [
    w => w.jobs.m3.needs.pop(), w => { w.jobs.m3.if = 'success()'; },
    w => { w.jobs.m3.steps[0].run += ' || true'; },
    w => { w.jobs['m3-platform'].if = 'false'; },
    w => { w.jobs['m3-platform']['continue-on-error'] = true; },
    w => w.jobs['m3-platform'].strategy.matrix.os.pop(),
    w => { w.jobs['m3-platform'].steps.at(-1).run = 'pnpm m3:quick'; },
    w => { w.jobs['m3-platform'].steps.at(-1)['continue-on-error'] = true; },
    w => w.jobs.m0.needs.pop(), w => w.jobs.m2.needs.pop(),
  ]) { const changed = workflow.toJS(); mutate(changed); assert.throws(() => validateWorkflow(changed)); }
});
