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
  assert.equal(job.needs, 'm3');
  assert.equal(job.if, "github.event_name == 'push' && github.ref == 'refs/heads/main'");
  const exportStep = job.steps.find(step => step.id === 'docs-bundle');
  assert(exportStep.run.includes('--source-commit "${{ github.sha }}"'));
  assert(exportStep.run.includes('--source-ref "${{ github.ref }}"'));
  const upload = job.steps.at(-1);
  assert.equal(upload.with.name, 'zryna-docs-next-${{ github.sha }}-${{ steps.docs-bundle.outputs.manifest-sha256 }}');
  assert.equal(upload.with['if-no-files-found'], 'error');
}

test('published M3 bytes require successful merged main M0-M3 authority', () => {
  const workflow = parseDocument(readFileSync(`${workspaceRoot}/.github/workflows/ci.yml`, 'utf8')).toJS();
  publication(workflow);
  for (const mutate of [
    w => { w.jobs['docs-publish'].needs = 'm2'; },
    w => { w.jobs['docs-publish'].if = 'always()'; },
    w => { w.jobs['docs-publish'].steps.at(-1).with.name = 'latest'; },
  ]) {
    const changed = structuredClone(workflow); mutate(changed);
    assert.throws(() => publication(changed));
  }
});
