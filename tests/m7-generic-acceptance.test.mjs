import assert from 'node:assert/strict';
import { withoutDeadlineEvidence } from './ci-deadline-evidence-cases.mjs';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { test } from 'node:test';
import { parseDocument } from 'yaml';
import { commandsFor, loadRegistry, platformName, verifyTestOutput } from '../scripts/m7-generic-acceptance/commands.mjs';
import { sourceIdentity } from '../scripts/m7-generic-acceptance/evidence.mjs';
const ROOT = resolve(import.meta.dirname, '..');
const registry = loadRegistry(ROOT);
function output(command, platform, change = {}) {
  const names = change.names ?? command.tests[platform];
  const summary = `test result: ok. ${change.passed ?? names.length} passed; ${change.failed ?? 0} failed; ${change.ignored ?? 0} ignored; 0 measured; 0 filtered out; finished in 0.01s`;
  return `${names.map(name => `test ${name} ... ok`).join('\n')}\n${summary}\n`;
}
test('platform packets are explicit, nonzero and do not replay full packages', () => {
  assert.equal(platformName('linux', 'x64'), 'linux');
  assert.equal(platformName('win32', 'x64'), 'windows');
  assert.throws(() => platformName('darwin', 'x64'));
  assert.throws(() => platformName('linux', 'arm64'));
  assert.deepEqual(commandsFor(registry, 'linux').map(row => row.id), ['public-cli']);
  assert.deepEqual(commandsFor(registry, 'windows').map(row => row.id), ['public-cli', 'portable-deep', 'shared-abi-ignored']);
  assert.deepEqual(commandsFor(registry, 'linux')[0].args, ['test', '--locked', '-p', 'zryna', '--test', 'generic_public', '--', '--test-threads=1']);
  assert(!registry.commands.some(row => row.args.includes('--workspace')));
  assert.equal(registry.commands[0].tests.linux.length, 11);
  assert(registry.commands[0].tests.linux.includes('original_template_only_library_builds_all_with_empty_scalar_abi'));
  assert.equal(registry.commands[0].tests.windows.length, 12);
});
test('each required exact test must execute once with zero failures and ignores', () => {
  for (const platform of ['linux', 'windows']) {
    for (const command of commandsFor(registry, platform)) {
      const good = output(command, platform);
      assert.equal(verifyTestOutput(good, '', command, platform).executed, command.tests[platform].length);
      assert.equal(verifyTestOutput(good.replaceAll('\n', '\r\n'), '', command, platform).ignored, 0);
      for (const wrong of [
        output(command, platform, { names: [], passed: 0 }),
        output(command, platform, { names: command.tests[platform].slice(1) }),
        output(command, platform, { names: [...command.tests[platform], command.tests[platform][0]] }),
        output(command, platform, { names: ['unrelated', ...command.tests[platform].slice(1)] }),
        output(command, platform, { failed: 1 }), output(command, platform, { ignored: 1 }),
        `${good}${good}`, good.replace('test result: ok.', 'test result: FAILED.'),
      ]) assert.throws(() => verifyTestOutput(wrong, '', command, platform));
    }
  }
});
test('inherited shared ABI gate is exact and genuinely reused by generic verification', () => {
  const abi = commandsFor(registry, 'windows')[2];
  assert.deepEqual(abi.tests.windows, [
    'tests::record_declaration_budget_is_exact_and_plus_one',
    'tests::nested_declaration_budget_is_exact_and_plus_one',
    'tests::checked_header_budget_is_exact_and_plus_one',
  ]);
  const source = readFileSync(resolve(ROOT, 'crates/zryna-ownership-runtime-abi/src/generic_v1.rs'), 'utf8');
  assert(source.includes('if !super::input_within_limits(&contract)'));
  const portable = commandsFor(registry, 'windows')[1];
  assert.equal(portable.tests.windows.length, 3);
  assert(!portable.platforms.includes('linux'));
});
test('tested merge revision and clean state are checked, never replaced by PR head', () => {
  const sha = 'a'.repeat(40); const tree = 'b'.repeat(40);
  const spawn = (_program, args) => ({ status: 0, signal: null, stderr: '',
    stdout: args[0] === 'status' ? '' : `${args.includes('HEAD^{tree}') ? tree : sha}\n` });
  assert.equal(sourceIdentity(ROOT, sha, spawn).testedTree, tree);
  assert.throws(() => sourceIdentity(ROOT, 'c'.repeat(40), spawn));
  const dirty = (program, args) => args[0] === 'status'
    ? { ...spawn(program, args), stdout: '?? changed.zry\n' } : spawn(program, args);
  assert.throws(() => sourceIdentity(ROOT, sha, dirty), 'plausible exact commit/tree cannot hide dirty source');
  for (const fault of [{ status: 1 }, { signal: 'SIGKILL' },
    { error: Object.assign(new Error('spawn'), { code: 'EIO' }) }, { stderr: 'unexpected diagnostic' }]) {
    assert.throws(() => sourceIdentity(ROOT, sha,
      (program, args) => ({ ...spawn(program, args), ...fault })),
    'failed observation with plausible source text is never acceptance');
  }
});
test('canonical M0 remains in Rust and the dedicated finite packet binds actual upstream Rust success', () => {
  const workflow = withoutDeadlineEvidence(parseDocument(readFileSync(resolve(ROOT, '.github/workflows/ci.yml'), 'utf8')).toJS(), true);
  const rust = workflow.jobs.rust;
  assert.deepEqual(rust.strategy.matrix.os, ['ubuntu-latest', 'windows-latest']);
  const checkout = rust.steps.find(row => row.uses?.startsWith('actions/checkout@'));
  assert.equal(checkout.with['fetch-depth'], 0);
  assert.equal(checkout.with.ref, undefined, 'preserve existing merge-revision integration policy');
  assert.equal(rust['timeout-minutes'], "${{ matrix.os == 'windows-latest' && 60 || 40 }}");
  const m0 = rust.steps.findIndex(row => row.id === 'm0-complete');
  assert.equal(m0, rust.steps.length - 1);
  assert.equal(rust.steps[m0].run, 'node scripts/run-m0-conformance.mjs');
  const job = workflow.jobs['m7-platform']; assert.deepEqual(job.needs, ['rust']);
  assert.deepEqual(job.strategy.matrix.os, rust.strategy.matrix.os);
  const packet = job.steps.find(row => row.id === 'm7-generic');
  assert.equal(packet.env.ZRYNA_M7_UPSTREAM_M0_RESULT, '${{ needs.rust.result }}');
  assert(packet.run.includes('--expected-source-revision "${{ github.sha }}"'));
  assert.equal(job.steps[0].with.ref, undefined, 'same tested merge revision in the dedicated job');
  const upload = job.steps.find(row => row.name === 'Retain M7 generic acceptance commands and actual counts');
  assert.equal(upload.with['if-no-files-found'], 'error');
  assert(upload.with.name.includes('${{ github.sha }}'));
});
