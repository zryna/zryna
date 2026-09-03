import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';

import {
  loadAndValidateManifest,
  validateManifestDocument,
  validatePackageDocument,
} from '../scripts/run-m0-conformance.mjs';

const fixturePaths = [
  'tests/fixtures/syntax-v2-valid.json',
  'tests/fixtures/syntax-v2-missing-field.json',
  'tests/fixtures/syntax-v2-unknown-field.json',
  'tests/fixtures/typescript-adapter-v2-request.json',
  'tests/fixtures/typescript-adapter-v2-result.json',
  'tests/fixtures/typescript-adapter-v2-error-result.json',
  'tests/fixtures/typescript-adapter-v2-warning-result.json',
  'tests/fixtures/syntax-v3-valid.json',
  'tests/fixtures/typescript-adapter-v3-request.json',
  'tests/fixtures/typescript-adapter-v3-result.json',
  'tests/fixtures/m2-straight-line-request.json',
  'tests/fixtures/m2-straight-line-result.json',
  'tests/fixtures/m2-semantic-negative-request.json',
  'tests/fixtures/m2-semantic-negative-result.json',
  'tests/fixtures/m2-control-flow-positive-request.json',
  'tests/fixtures/m2-control-flow-positive-result.json',
  'tests/fixtures/m2-control-flow-negative-request.json',
  'tests/fixtures/m2-control-flow-negative-result.json',
];

function clonedManifest() {
  return structuredClone(loadAndValidateManifest());
}

function workflowJob(workflow, jobId) {
  const lines = workflow.split(/\r?\n/);
  const start = lines.findIndex((line) => line === `  ${jobId}:`);
  assert.notEqual(start, -1, `missing ${jobId} job`);
  const next = lines.findIndex((line, index) => index > start && /^  [a-zA-Z0-9_-]+:$/.test(line));
  return lines.slice(start, next === -1 ? undefined : next).join('\n');
}

test('M0 registry names every fail-closed foundation boundary', () => {
  const manifest = loadAndValidateManifest();
  assert.equal(manifest.schemaVersion, 1);
  assert.equal(manifest.milestone, 'M0');
  assert.deepEqual(
    manifest.coverage.map(({ id }) => id),
    [
      'architecture-scanner-manifest-and-graph',
      'source-authority',
      'diagnostic-authority',
      'frontend-handshake-and-worker',
      'syntax-protocol-verifier',
      'semantic-stop-gate',
      'universal-ir-verifier',
      'javascript-backend-boundary',
      'native-mir-verifier',
      'native-backend-boundary',
      'driver-boundary',
      'typescript-adapter',
      'protocol-wire-schema',
    ],
  );
});

test('M0 fixture inventory rejects unlisted additions and listed deletions', () => {
  const manifest = clonedManifest();
  assert.throws(
    () => validateManifestDocument(manifest, [...fixturePaths, 'tests/fixtures/unlisted.json']),
    /registered fixture inventory differs/,
  );
  assert.throws(
    () => validateManifestDocument(manifest, fixturePaths.slice(1)),
    /registered fixture inventory differs/,
  );
});

test('M0 fixture declarations reject duplicates and changed expectations', () => {
  const duplicated = clonedManifest();
  duplicated.fixtures.push(structuredClone(duplicated.fixtures[0]));
  assert.throws(() => validateManifestDocument(duplicated, fixturePaths), /fixture ids or order differ/);

  for (const mutation of [
    (fixture) => { fixture.mode = fixture.mode === 'pass' ? 'fail' : 'pass'; },
    (fixture) => { fixture.expected = 'different-outcome'; },
    (fixture) => { fixture.phase = 'unknown-phase'; },
  ]) {
    const changed = clonedManifest();
    mutation(changed.fixtures[0]);
    assert.throws(
      () => validateManifestDocument(changed, fixturePaths),
      /declaration differs from its frozen phase, mode, or expectation/,
    );
  }
});

test('M0 command declarations reject structural and field mutations', () => {
  for (const mutate of [
    (manifest) => manifest.commands.push(structuredClone(manifest.commands[0])),
    (manifest) => manifest.commands.pop(),
    (manifest) => manifest.commands.reverse(),
    (manifest) => { manifest.commands[1].id = manifest.commands[0].id; },
    (manifest) => { manifest.commands[0].executable = 'node'; },
    (manifest) => { manifest.commands[1].args = ['--version']; },
    (manifest) => { manifest.commands[0].platforms = ['windows', 'linux']; },
    (manifest) => { delete manifest.commands[5].environment; },
    (manifest) => { manifest.commands[5].environment.RUSTDOCFLAGS = '-A warnings'; },
    (manifest) => { manifest.commands[0].environment = { RUSTDOCFLAGS: '-D warnings' }; },
  ]) {
    const manifest = clonedManifest();
    mutate(manifest);
    assert.throws(() => validateManifestDocument(manifest, fixturePaths), /invalid M0 conformance registry/);
  }
});

test('M0 coverage declarations reject structural and field mutations', () => {
  for (const mutate of [
    (manifest) => manifest.coverage.push(structuredClone(manifest.coverage[0])),
    (manifest) => manifest.coverage.pop(),
    (manifest) => manifest.coverage.reverse(),
    (manifest) => { manifest.coverage[1].id = manifest.coverage[0].id; },
    (manifest) => { manifest.coverage[0].owner = 'docs'; },
    (manifest) => { manifest.coverage[0].commandId = 'protocol-tests'; },
    (manifest) => { manifest.coverage[0].proofs = ['unrelated proof']; },
  ]) {
    const manifest = clonedManifest();
    mutate(manifest);
    assert.throws(() => validateManifestDocument(manifest, fixturePaths), /invalid M0 conformance registry/);
  }
});

test('M0 registry rejects unknown root fields', () => {
  const manifest = clonedManifest();
  manifest.untrusted = true;
  assert.throws(() => validateManifestDocument(manifest, fixturePaths), /must contain exactly/);
});

test('documented package alias is bound to the canonical runner', async () => {
  const packageDocument = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'));
  assert.doesNotThrow(() => validatePackageDocument(packageDocument));
  const changedPreflight = structuredClone(packageDocument);
  changedPreflight.scripts.preflight = 'node --version';
  assert.throws(() => validatePackageDocument(changedPreflight), /preflight must be exactly/);
  packageDocument.scripts['m0:check'] = 'node --version';
  assert.throws(() => validatePackageDocument(packageDocument), /m0:check must be exactly/);
});

test('package protocol gates retain exact v2 and v3 coverage', async () => {
  const packageDocument = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'));
  assert.doesNotThrow(() => validatePackageDocument(packageDocument));
  for (const script of ['protocol:check', 'protocol:test']) {
    const changed = structuredClone(packageDocument);
    changed.scripts[script] = changed.scripts[script].replace('v2', 'v4');
    assert.throws(() => validatePackageDocument(changed), /must cover exact protocol v2 and v3/);
  }
});

test('required CI consumes the same canonical gate on Linux and Windows', async () => {
  const workflow = await readFile(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8');
  const rust = workflowJob(workflow, 'rust');
  const adapterPlatform = workflowJob(workflow, 'adapter-platform');
  assert.doesNotMatch(rust, /needs:/);
  assert.match(rust, /name: rust \(\$\{\{ matrix\.os \}\}\)/);
  assert.match(rust, /os: \[ubuntu-latest, windows-latest\]/);
  assert.match(rust, /run: node scripts\/run-m0-conformance\.mjs/);
  assert.match(adapterPlatform, /os: \[ubuntu-latest, windows-latest\]/);
  assert.doesNotMatch(adapterPlatform, /needs:/);
  assert.match(adapterPlatform, /run: pnpm protocol:check/);
  assert.match(adapterPlatform, /run: pnpm protocol:test/);
});

test('CI avoids duplicate feature runs and cancels superseded revisions', async () => {
  const workflow = await readFile(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8');
  assert.match(workflow, /push:\n    branches: \[main\]\n  pull_request:/);
  assert.match(
    workflow,
    /group: ci-\$\{\{ github\.workflow \}\}-\$\{\{ github\.event\.pull_request\.number \|\| github\.ref \}\}/,
  );
  assert.match(workflow, /cancel-in-progress: true/);
});

test('Windows CLI smoke precedes the complete M0 gate', async () => {
  const workflow = await readFile(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8');
  const rust = workflowJob(workflow, 'rust');
  const smoke = 'cargo test --locked -p zryna --test cli javascript_build_and_run_publish_exact_bundles -- --exact';
  const completeGate = 'node scripts/run-m0-conformance.mjs';
  assert.match(rust, /if: runner\.os == 'Windows'/);
  assert.ok(rust.indexOf(smoke) > -1, 'missing Windows CLI smoke command');
  assert.ok(rust.indexOf(smoke) < rust.indexOf(completeGate), 'Windows CLI smoke must run before M0');
});

test('documentation publication waits for the complete M2 gate and binds exact provenance', async () => {
  const workflow = await readFile(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8');
  const rust = workflowJob(workflow, 'rust');
  const publisher = workflowJob(workflow, 'docs-publish');
  const docsCheck = 'run: pnpm docs:check';
  const completeGate = 'run: node scripts/run-m0-conformance.mjs';
  assert.ok(rust.indexOf(docsCheck) > -1, 'missing documentation bundle validation');
  assert.ok(rust.indexOf(docsCheck) < rust.indexOf(completeGate), 'documentation validation must precede M0');
  assert.doesNotMatch(rust, /Export authenticated next documentation bundle/);
  assert.match(publisher, /if: github\.event_name == 'push' && github\.ref == 'refs\/heads\/main'/);
  assert.match(publisher, /needs: m2/);
  assert.ok(publisher.indexOf(docsCheck) > -1, 'publisher must revalidate documentation');
  assert.ok(
    publisher.indexOf(docsCheck) < publisher.indexOf('node scripts/docs/export.mjs'),
    'publisher must validate before export',
  );
  assert.match(publisher, /--source-commit "\$\{\{ github\.sha \}\}"/);
  assert.match(publisher, /--source-ref "\$\{\{ github\.ref \}\}"/);
  assert.match(publisher, /actions\/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02/);
  assert.match(
    publisher,
    /name: zryna-docs-next-\$\{\{ github\.sha \}\}-\$\{\{ steps\.docs-bundle\.outputs\.manifest-sha256 \}\}/,
  );
  assert.match(publisher, /Compiler commit:[\s\S]*Documentation manifest SHA-256:/);
  assert.match(publisher, /compression-level: 0/);
  assert.match(publisher, /include-hidden-files: true/);
});

test('required CI exposes a stable aggregate over Rust and adapter gates', async () => {
  const workflow = await readFile(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8');
  const adapter = workflowJob(workflow, 'adapter');
  const aggregate = workflowJob(workflow, 'm0');
  assert.match(adapter, /name: adapter/);
  assert.match(aggregate, /name: m0/);
  assert.match(aggregate, /if: always\(\)/);
  assert.match(aggregate, /needs: \[owned-data-quick, preflight, rust, adapter\]/);
  assert.match(
    aggregate,
    /OWNED_DATA_QUICK_RESULT: \$\{\{ needs\.owned-data-quick\.result \}\}/,
  );
  assert.match(aggregate, /test "\$OWNED_DATA_QUICK_RESULT" = success/);
  assert.match(aggregate, /PREFLIGHT_RESULT: \$\{\{ needs\.preflight\.result \}\}/);
  assert.match(aggregate, /RUST_RESULT: \$\{\{ needs\.rust\.result \}\}/);
  assert.match(aggregate, /ADAPTER_RESULT: \$\{\{ needs\.adapter\.result \}\}/);
});

test('public contributor docs name the canonical closure command', async () => {
  for (const relativePath of ['../README.md', '../CONTRIBUTING.md', '../docs/M0_CONFORMANCE.md']) {
    const contents = await readFile(new URL(relativePath, import.meta.url), 'utf8');
    assert.match(contents, /pnpm m0:check/);
  }
});
