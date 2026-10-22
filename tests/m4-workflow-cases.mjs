import assert from 'node:assert/strict';

const sqliteSetup = {
  name: 'Prepare pinned SQLite source fixture', if: "runner.os == 'Linux'",
  'timeout-minutes': 3, shell: 'bash',
  run: `python3 -B scripts/m5/prepare_sqlite_fixture.py --output "$RUNNER_TEMP/zryna-sqlite-source"
printf 'ZRYNA_SQLITE_FIXTURE_ROOT=%s\\n' "$RUNNER_TEMP/zryna-sqlite-source" >> "$GITHUB_ENV"
`,
};

// Authenticate this exact prerequisite before projecting the unchanged legacy workflow.
export function withoutSQLiteFixture(candidate) {
  const copy = structuredClone(candidate);
  for (const [id, target] of [
    ['preflight', step => step.run === 'node scripts/run-preflight.mjs --compiler-contracts'],
    ['rust', step => step.name === 'Verify M2 driver security'],
    ['m4-platform', step => step.name === 'Execute complete M0 through M4 core matrix'],
  ]) {
    const steps = copy.jobs[id].steps;
    const setups = steps.filter(step => step.name === sqliteSetup.name);
    const executions = steps.filter(target);
    assert.equal(setups.length, 1, `${id}: one required SQLite setup`);
    assert.equal(executions.length, 1, `${id}: one owning execution`);
    assert.deepEqual(setups[0], sqliteSetup, `${id}: exact required source acquisition`);
    const index = steps.indexOf(executions[0]);
    assert(index > 0);
    assert.equal(steps[index - 1], setups[0], `${id}: setup immediately precedes owner`);
    steps.splice(index - 1, 1);
  }
  assert(!/sqlite/i.test(JSON.stringify(copy)), 'no other SQLite setup, locator or bypass');
  return copy;
}

// Authenticate the additive M4 routing/lanes before existing foundation snapshot checks.
export function withoutM4Workflow(candidate) {
  const copy = withoutSQLiteFixture(candidate);
  const jobs = copy.jobs;
  const core = jobs['m4-platform'];
  assert.equal(core.name, 'm4 (${{ matrix.os }})');
  assert.deepEqual(core.strategy.matrix.os, ['ubuntu-latest', 'windows-latest']);
  assert.equal(core['timeout-minutes'], 90);
  assert.equal(core.needs, 'route-contracts');
  assert.equal(core.if, "needs.route-contracts.outputs.m4 == 'true'");
  assert.equal(core['continue-on-error'], undefined);
  assert.equal(core.steps[0].with.ref, '${{ github.event.pull_request.head.sha || github.sha }}');
  assert.equal(core.steps[0].with['persist-credentials'], false);
  assert(core.steps.some(step => step.run?.includes('--lane core --evidence')));
  assert(core.steps.some(step => step.run === 'pnpm m4:registry && pnpm m4:evidence:contract'));
  for (const step of core.steps) {
    assert.equal(step['continue-on-error'], undefined);
    if (step.if !== undefined) assert(step === core.steps.at(-2) || step === core.steps.at(-1), 'only closed uploads are conditional');
  }
  const uploads = core.steps.filter(step => step.uses?.startsWith('actions/upload-artifact@'));
  assert.deepEqual(uploads, core.steps.slice(-2), 'exact final upload pair');
  for (const [index, upload] of uploads.entries()) {
    assert.equal(upload.if, index === 0 ? 'success()' : 'failure()');
    assert.equal(upload.uses, 'actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a');
    assert.deepEqual(upload.with, {
      name: `m4-core-${index === 0 ? '' : 'diagnostics-'}${'${{ matrix.os }}'}-${'${{ github.event.pull_request.head.sha || github.sha }}'}-${'${{ github.run_id }}'}-${'${{ github.run_attempt }}'}`,
      path: `${'${{ runner.temp }}'}/m4-core-evidence${index === 0 ? '-retained' : ''}/**`,
      'if-no-files-found': 'error', 'retention-days': 90, 'compression-level': 0,
    });
  }
  for (const lane of ['browser', 'installed']) {
    assert.deepEqual(jobs[`m4-${lane}`], {
      name: `m4 ${lane}`, if: "needs.route-contracts.outputs.m4 == 'true'", needs: 'route-contracts',
      uses: lane === 'browser' ? './.github/workflows/browser-conformance.yml' : './.github/workflows/installed-command-h1.yml',
    });
  }
  const aggregate = jobs.m4;
  assert.equal(aggregate.name, 'm4');
  assert.equal(aggregate.if, 'always()');
  assert.deepEqual(aggregate.needs, ['route-contracts', 'm3', 'm4-platform', 'm4-browser', 'm4-installed']);
  assert.equal(aggregate.steps.at(-1).run, 'node scripts/m4/verify-ci-result.mjs');
  assert.deepEqual(aggregate.steps.at(-1).env, {
    M4_REQUIRED: '${{ needs.route-contracts.outputs.m4 }}', ROUTING_RESULT: '${{ needs.route-contracts.result }}',
    M3_RESULT: '${{ needs.m3.result }}', M4_CORE_RESULT: '${{ needs.m4-platform.result }}',
    M4_BROWSER_RESULT: '${{ needs.m4-browser.result }}', M4_INSTALLED_RESULT: '${{ needs.m4-installed.result }}',
  });
  assert.equal(jobs['route-contracts'].outputs.m4, '${{ steps.routes.outputs.m4 }}');
  delete jobs['route-contracts'].outputs.m4;
  for (const id of ['m4-platform', 'm4-browser', 'm4-installed', 'm4']) delete jobs[id];
  return copy;
}
