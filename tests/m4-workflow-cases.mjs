import assert from 'node:assert/strict';

// Authenticate the additive M4 routing/lanes before existing foundation snapshot checks.
export function withoutM4Workflow(candidate) {
  const copy = structuredClone(candidate);
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
    if (step.if !== undefined) assert.equal(step.if, 'always()');
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
