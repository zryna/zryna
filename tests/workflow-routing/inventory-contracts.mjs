import assert from 'node:assert/strict';
import test from 'node:test';
import { readdirSync } from 'node:fs';
import { resolve } from 'node:path';

export function assertDocumentationPublicationContract(documentation) {
  assert.deepEqual(Object.keys(documentation.on), ['workflow_dispatch']);
  const dispatch = documentation.on.workflow_dispatch;
  assert.deepEqual(Object.keys(dispatch), ['inputs']);
  assert.deepEqual(Object.keys(dispatch.inputs), [
    'ci_run_id', 'ci_attempt', 'expectations_sha256', 'expectations_json',
  ]);
  for (const input of Object.values(dispatch.inputs)) {
    assert.deepEqual(Object.keys(input).sort(), ['description', 'required', 'type']);
    assert.equal(typeof input.description, 'string');
    assert.ok(input.description.length > 0);
    assert.equal(input.required, true);
    assert.equal(input.type, 'string');
  }
  assert.deepEqual(documentation.concurrency, {
    group: 'documentation-${{ github.ref }}', 'cancel-in-progress': false,
  });
  assert.deepEqual(documentation.permissions, { contents: 'read', actions: 'read', checks: 'read' });
  assert.deepEqual(Object.keys(documentation.jobs), ['docs-publish']);
  const publisher = documentation.jobs['docs-publish'];
  assert.equal(publisher.if, "github.ref == 'refs/heads/main'");
  assert.equal(publisher.needs, undefined);
  assert.equal(publisher['continue-on-error'], undefined);
  assert.deepEqual(publisher.steps[0], {
    uses: 'actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1',
    with: { 'fetch-depth': 0, ref: '${{ github.sha }}', 'persist-credentials': false },
  });
  for (const step of publisher.steps) {
    assert.equal(step.if, undefined, 'publication steps must not bypass a failed prerequisite');
    assert.equal(step['continue-on-error'], undefined);
    if (step.uses) assert.match(step.uses, /@[0-9a-f]{40}$/);
  }
  const uniqueStep = (predicate) => {
    const matches = publisher.steps.filter(predicate);
    assert.equal(matches.length, 1);
    return matches[0];
  };
  const validation = uniqueStep((step) => step.run === 'pnpm docs:check');
  const authentication = uniqueStep((step) => step.run ===
    'node scripts/m4/verify-publication.mjs --expectations-sha256 "$M4_EXPECTATIONS_SHA256" --retained "$M4_RETAINED"');
  const exported = uniqueStep((step) => step.id === 'docs-bundle');
  assert.deepEqual(authentication.env, {
    GITHUB_TOKEN: '${{ github.token }}',
    M4_CI_RUN_ID: '${{ inputs.ci_run_id }}',
    M4_CI_ATTEMPT: '${{ inputs.ci_attempt }}',
    M4_EXPECTATIONS_JSON: '${{ inputs.expectations_json }}',
    M4_EXPECTATIONS_SHA256: '${{ inputs.expectations_sha256 }}',
    M4_RETAINED: '${{ runner.temp }}/m4-publication-${{ github.run_id }}-${{ github.run_attempt }}',
  });
  assert.ok(publisher.steps.indexOf(validation) < publisher.steps.indexOf(authentication));
  assert.ok(publisher.steps.indexOf(authentication) < publisher.steps.indexOf(exported));
  assert.match(exported.run, /node scripts\/docs\/export\.mjs/);
  assert.match(exported.run, /--channel next/);
  assert.match(exported.run, /--source-commit "\$\{\{ github\.sha \}\}"/);
  assert.match(exported.run, /--source-ref "\$\{\{ github\.ref \}\}"/);
  assert.match(exported.run, /--output \.zryna\/out\/docs\/next-\$\{\{ github\.run_id \}\}-\$\{\{ github\.run_attempt \}\}/);
  assert.match(exported.run, /\^\[0-9a-f\]\{64\}\$/);
  assert.match(exported.run, /manifest-sha256=\$digest/);
  assert.match(exported.run, /Compiler commit:[\s\S]*Documentation manifest SHA-256:/);
  const uploads = publisher.steps.filter((step) => step.uses?.startsWith('actions/upload-artifact@'));
  assert.equal(uploads.length, 2);
  for (const upload of uploads) {
    assert.equal(upload.uses, 'actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02');
    assert.ok(publisher.steps.indexOf(exported) < publisher.steps.indexOf(upload));
    assert.equal(upload.with['if-no-files-found'], 'error');
    assert.equal(upload.with['compression-level'], 0);
    assert.equal(upload.with['include-hidden-files'], true);
    assert.equal(upload.with['retention-days'], 90);
  }
  const retained = '${{ runner.temp }}/m4-publication-${{ github.run_id }}-${{ github.run_attempt }}';
  assert.equal(uploads[0].with.name, 'zryna-docs-gate-${{ github.sha }}-${{ github.run_id }}-${{ github.run_attempt }}');
  assert.equal(uploads[0].with.path, `${retained}/expectations.json\n${retained}/publication-gate.json\n`);
  assert.equal(uploads[1].with.name, 'zryna-docs-next-${{ github.sha }}-${{ steps.docs-bundle.outputs.manifest-sha256 }}');
  assert.equal(uploads[1].with.path, '.zryna/out/docs/next-${{ github.run_id }}-${{ github.run_attempt }}');
}

export function registerWorkflowInventoryContracts({ workflow, root, ci, documentation }) {
  test('pull-request workflows stay inventoried and every superseded run cancels', () => {
    const names = readdirSync(resolve(root, '.github/workflows')).sort();
    assert.deepEqual(names, [
      'browser-conformance.yml', 'ci.yml', 'cli-performance-collector.yml', 'cli-performance.yml', 'dependency-security.yml', 'documentation.yml', 'installed-command-h1.yml',
      'm6-installed-editor.yml', 'native-host-facts.yml', 'native-provider-activation.yml', 'native-provider-install.yml',
      'native-provider-public.yml', 'playground-compiler-release.yml', 'portable-setup.yml',
      'release-production-candidate.yml',
      'release-qualification.yml', 'release.yml', 'windows-job-metrics.yml',
    ]);
    for (const name of names) {
      const candidate = workflow(name);
      if (!Object.hasOwn(candidate.on, 'pull_request')) continue;
      assert.equal(candidate.concurrency['cancel-in-progress'], true, name);
      assert.match(candidate.concurrency.group, /pull_request\.number/, name);
    }
    assert.equal(Object.hasOwn(workflow('portable-setup.yml').on, 'pull_request'), false);
    assert.equal(ci.jobs['portable-setup'].uses, './.github/workflows/portable-setup.yml');
    assert.deepEqual(ci.jobs['portable-setup'].needs, ['route-contracts', 'm0']);
  });

  test('exact-main documentation dispatch authenticates full CI and M4 before publication', () => {
    assertDocumentationPublicationContract(documentation);
  });

  test('documentation publication rejects weakened dispatch, provenance and gate contracts', () => {
    const mutations = [
      (value) => { value.on.push = { branches: ['main'] }; },
      (value) => { value.on.workflow_dispatch.inputs.ci_attempt.required = false; },
      (value) => { value.on.workflow_dispatch.inputs.expectations_json.default = '{}'; },
      (value) => { delete value.on.workflow_dispatch.inputs.expectations_sha256; },
      (value) => { value.concurrency['cancel-in-progress'] = true; },
      (value) => { value.jobs['docs-publish'].if = "github.ref == 'refs/heads/review'"; },
      (value) => { value.jobs['docs-publish'].steps[0].with.ref = 'main'; },
      (value) => {
        const steps = value.jobs['docs-publish'].steps;
        const auth = steps.findIndex((step) => step.env?.M4_CI_RUN_ID);
        [steps[auth], steps[auth + 1]] = [steps[auth + 1], steps[auth]];
      },
      (value) => {
        value.jobs['docs-publish'].steps.find((step) => step.env?.M4_CI_RUN_ID)['continue-on-error'] = true;
      },
      (value) => {
        value.jobs['docs-publish'].steps.find((step) => step.env?.M4_CI_RUN_ID).env.M4_CI_ATTEMPT = '1';
      },
      (value) => { value.jobs['docs-publish'].steps.find((step) => step.id === 'docs-bundle').if = 'always()'; },
      (value) => {
        value.jobs['docs-publish'].steps.find((step) => step.with?.name?.startsWith('zryna-docs-gate-')).with['retention-days'] = 7;
      },
      (value) => { value.jobs['docs-publish'].steps.at(-1).with['retention-days'] = 7; },
    ];
    for (const mutate of mutations) {
      const weakened = structuredClone(documentation);
      mutate(weakened);
      assert.throws(() => assertDocumentationPublicationContract(weakened), assert.AssertionError);
    }
  });
}
