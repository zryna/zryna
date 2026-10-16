import assert from 'node:assert/strict';
import test from 'node:test';

export function registerRequiredPlatformContracts(ci) {
  test('CI retains every protected pull-request context and one manual full entry point', () => {
    assert.deepEqual(ci.on, {
      pull_request: null,
      workflow_dispatch: { inputs: { structure_base: {
        description: 'Full trusted ancestor commit for repository structure validation',
        required: true,
        type: 'string',
      } } },
    });
    assert.equal(ci.env.ZRYNA_STRUCTURE_BASE,
      "${{ github.event_name == 'pull_request' && github.event.pull_request.base.sha || inputs.structure_base }}");
    const contexts = [
      ...ci.jobs.rust.strategy.matrix.os.map((os) => ci.jobs.rust.name.replace('${{ matrix.os }}', os)),
      ci.jobs.adapter.name,
      ci.jobs.m0.name,
      ci.jobs.m2.name,
      ci.jobs.m3.name,
    ];
    assert.deepEqual(contexts, [
      'rust (ubuntu-latest)',
      'rust (windows-latest)',
      'adapter',
      'm0',
      'm2',
      'm3',
    ]);
    assert.equal(ci.concurrency['cancel-in-progress'], true);
    assert.match(ci.concurrency.group, /pull_request\.number/);
    assert.match(ci.jobs['route-contracts'].steps.at(-1).run, /workflow-paths\.mjs --all/);
    assert.deepEqual(ci.jobs.m0.needs,
      ['fast-contracts', 'owned-data-quick', 'preflight', 'rust', 'adapter', 'route-contracts',
        'provider-conformance-v4']);
    assert.deepEqual(ci.jobs.m0.steps[0], {
      uses: 'actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1',
      with: { 'fetch-depth': 0 },
    });
    const m0Gate = ci.jobs.m0.steps.at(-1);
    assert.match(m0Gate.run, /ROUTING_RESULT/);
    assert.match(m0Gate.run, /verify-provider-v4-ci-result\.mjs/);
    assert.equal(m0Gate.env.PROVIDER_V4_REQUIRED,
      '${{ needs.route-contracts.outputs.provider_v4 }}');
    assert.equal(m0Gate.env.PROVIDER_V4_RESULT,
      '${{ needs.provider-conformance-v4.result }}');
    assert.equal(ci.jobs.m0.if, 'always()');
  });

  test('required Rust platform jobs include playground contracts and Linux helper units', () => {
    const rust = ci.jobs.rust;
    assert.deepEqual(rust.strategy.matrix.os, ['ubuntu-latest', 'windows-latest']);
    const contracts = rust.steps.filter(step => step.name === 'Verify restricted playground and M6 contracts');
    assert.deepEqual(contracts, [{ name: 'Verify restricted playground and M6 contracts',
      run: 'pnpm playground:contract\npnpm m6:contract\n' }]);
    const helpers = rust.steps.filter(step => step.name === 'Verify Linux playground helper units');
    assert.deepEqual(helpers, [{ name: 'Verify Linux playground helper units', if: "runner.os == 'Linux'",
      env: { PYTHONDONTWRITEBYTECODE: '1' }, run: 'python3 tests/playground-host-unit.py\n' +
        'python3 tests/playground-cgroup-unit.py\npython3 tests/playground-join-unit.py\n' +
        'python3 tests/playground-helper-policy-unit.py\n' +
        'python3 -B examples/playground/restricted/daemon/preparation/test_preparation.py\n' +
        "python3 -B -m unittest discover -s tests -p 'playground_operator*.py'\n" }]);
    const publication = rust.steps.filter(step => step.name === 'Verify Linux compiler publication boundaries');
    assert.deepEqual(publication, [{ name: 'Verify Linux compiler publication boundaries',
      if: "runner.os == 'Linux'",
      run: 'node --test --test-concurrency=1 scripts/m6/compiler-release/*.test.mjs' }]);
    const operator = rust.steps.filter(step => step.name === 'Verify Linux operator qualification contracts');
    assert.deepEqual(operator, [{ name: 'Verify Linux operator qualification contracts',
      if: "runner.os == 'Linux'", env: { PYTHONDONTWRITEBYTECODE: '1' },
      run: 'node --test --test-concurrency=1 examples/playground/restricted/daemon/launcher*.test.mjs\n' +
        'node --test --test-concurrency=1 examples/playground/restricted/daemon/cli-parity/*.test.mjs\n' +
        'node --test --test-concurrency=1 examples/playground/restricted/daemon/ui-qualification/*.test.mjs\n' +
        "python3 -B -m unittest discover -s examples/playground/restricted/daemon/cli-parity -p 'test_*.py'\n" +
        "python3 -B -m unittest discover -s examples/playground/restricted/daemon/ui-qualification -p 'test_owner_*.py'\n" }]);
    assert(rust.steps.indexOf(contracts[0]) < rust.steps.indexOf(helpers[0]));
    assert(rust.steps.indexOf(helpers[0]) < rust.steps.indexOf(publication[0]));
    assert(rust.steps.indexOf(publication[0]) < rust.steps.indexOf(operator[0]));
    assert.equal(rust.steps.at(-1).run, 'node scripts/run-m0-conformance.mjs');
  });
}
