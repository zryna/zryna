import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { parseDocument } from 'yaml';
import { loadAndValidateM3Conformance, workspaceRoot } from '../scripts/check-m3-conformance.mjs';

test('complete beginner sources and commands bind to the independent executable corpus', () => {
  const registry = loadAndValidateM3Conformance();
  const guide = readFileSync(`${workspaceRoot}/docs/M3_GETTING_STARTED.md`, 'utf8');
  const sections = [...guide.matchAll(/^### ([a-z-]+):[^\n]*\n([\s\S]*?)(?=^### |^## |$(?![\s\S]))/gm)];
  assert.equal(sections.length, 12);
  for (const [, id, body] of sections) {
    const fixture = registry.fixtures.find(f => f.id === id);
    assert(fixture, id);
    const sources = [...body.matchAll(/```zry\n([\s\S]*?)```/g)].map(match => match[1]);
    const expected = [readFileSync(`${workspaceRoot}/${fixture.path}`, 'utf8')];
    if (fixture.dependency) {
      const dependency = registry.fixtures.find(f => f.id === fixture.dependency);
      expected.push(readFileSync(`${workspaceRoot}/${dependency.path}`, 'utf8'));
    }
    assert.deepEqual(sources, expected, `${id}: complete source bytes`);
    const observation = registry.valid.find(entry => entry.fixture === id);
    const args = observation.arguments.map(value => `--arg=i32:${value}`).join(' ');
    const command = `cargo run --locked -p zryna -- run .zryna/cache/m3-guide/main.zry --profile data-ownership-v1 --target javascript --name m3-${id}-run-1 --export ${observation.export} ${args} --node "$NODE"`;
    assert(body.includes(command), `${id}: exact public command`);
    assert(body.includes(`javascript: i32 ${observation.expected}`), `${id}: fixed observation`);
  }
  for (const id of ['moved', 'unknown', 'conflict']) {
    const fixture = registry.fixtures.find(f => f.id === id);
    assert(guide.includes(readFileSync(`${workspaceRoot}/${fixture.path}`, 'utf8')));
    const invalid = registry.invalid.find(entry => entry.fixture === id);
    assert(guide.includes(invalid.code));
  }
});

function publication(workflow) {
  const job = workflow.jobs['docs-publish'];
  assert.equal(job.if, "github.ref == 'refs/heads/main'");
  const gate = job.steps.find(step => step.name === 'Authenticate completed exact-main full CI and all M4 evidence');
  assert(gate?.run.includes('scripts/m4/verify-publication.mjs'));
  assert(job.steps.indexOf(gate) < job.steps.findIndex(step => step.id === 'docs-bundle'));
  assert.equal(gate.env.M4_EXPECTATIONS_SHA256, '${{ inputs.expectations_sha256 }}');
  assert.equal(gate.env.M4_CI_RUN_ID, '${{ inputs.ci_run_id }}');
  assert.equal(gate.env.M4_CI_ATTEMPT, '${{ inputs.ci_attempt }}');
  const exportStep = job.steps.find(step => step.id === 'docs-bundle');
  assert(exportStep.run.includes('--source-commit "${{ github.sha }}"'));
  assert(exportStep.run.includes('--source-ref "${{ github.ref }}"'));
  const upload = job.steps.at(-1);
  assert.equal(upload.with.name, 'zryna-docs-next-${{ github.sha }}-${{ steps.docs-bundle.outputs.manifest-sha256 }}');
  assert.equal(upload.with['if-no-files-found'], 'error');
  assert.equal(upload.with['retention-days'], 90);
}

test('published M3 bytes use the protected main publication workflow', () => {
  const workflow = parseDocument(
    readFileSync(`${workspaceRoot}/.github/workflows/documentation.yml`, 'utf8'),
  ).toJS();
  assert.deepEqual(Object.keys(workflow.on), ['workflow_dispatch']);
  for (const name of ['ci_run_id', 'ci_attempt', 'expectations_sha256', 'expectations_json']) {
    assert.equal(workflow.on.workflow_dispatch.inputs[name].required, true);
  }
  assert.equal(workflow.permissions.actions, 'read');
  assert.equal(workflow.permissions.checks, 'read');
  publication(workflow);
  for (const mutate of [
    w => { w.jobs['docs-publish'].if = 'true'; },
    w => { w.jobs['docs-publish'].steps = w.jobs['docs-publish'].steps.filter(step => !step.run?.includes('scripts/m4/verify-publication.mjs')); },
    w => { w.jobs['docs-publish'].steps.at(-1).with.name = 'latest'; },
    w => { w.jobs['docs-publish'].steps.at(-1).with['retention-days'] = 7; },
  ]) {
    const changed = structuredClone(workflow); mutate(changed);
    assert.throws(() => publication(changed));
  }
});
