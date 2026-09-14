import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import test from 'node:test';
import { parseDocument } from 'yaml';
import {
  canonicalBounded, parseCanonical, sha256,
} from '../scripts/distribution-release/canonical.mjs';
import {
  ACCEPTED_RECIPE_SHA256,
  checkReleaseReadiness,
  REQUIRED_RELEASE_PATHS,
} from '../scripts/distribution-release/check-release-readiness.mjs';

const root = resolve(import.meta.dirname, '..');
const parsed = parseDocument(readFileSync(resolve(root, '.github/workflows/release.yml'), 'utf8'));
assert.deepEqual(parsed.errors, []);
const workflow = parsed.toJS();
const shaUse = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+@[0-9a-f]{40}$/;

function wire(value) { return Buffer.from(`${canonicalBounded(value)}\n`); }

function readinessFixture(recipe) {
  const source = resolve('release-readiness-source');
  const sourceCommit = 'a'.repeat(40);
  const spawn = (executable, args, options) => {
    assert.equal(executable, 'git');
    let output;
    if (args[0] === 'ls-tree') {
      const path = args.at(-1);
      output = Buffer.from(`100644 blob ${'b'.repeat(40)}\t${path}\0`);
    } else if (args[0] === 'cat-file') output = recipe;
    else throw new Error(`unexpected git command ${args.join(' ')}`);
    return { status: 0, stdout: options.encoding === null ? output : output.toString('utf8') };
  };
  return {
    cwd: source,
    environment: {
      GITHUB_EVENT_NAME: 'push', GITHUB_REPOSITORY: 'zryna/zryna',
      GITHUB_REF: 'refs/tags/v0.2.3', GITHUB_REF_PROTECTED: 'true',
      GITHUB_SHA: sourceCommit, GITHUB_WORKFLOW_SHA: sourceCommit, ZRYNA_SOURCE_ROOT: source,
    },
    sourceCommit,
    spawn,
  };
}

function steps(job, name) {
  return job.steps.filter((step) => step.name === name);
}

test('release workflow has one exact protected-tag entry and non-cancellable run identity', () => {
  assert.deepEqual(workflow.on, { push: { tags: ['v0.2.3'] } });
  assert.deepEqual(workflow.permissions, { contents: 'read' });
  assert.deepEqual(workflow.concurrency, {
    group: 'release-${{ github.ref }}',
    'cancel-in-progress': false,
  });
  assert.deepEqual(workflow.env, {
    NODE_VERSION: '22.22.1', PNPM_VERSION: '11.18.0', RUST_VERSION: '1.97.1',
  });
  assert.deepEqual(Object.keys(workflow.jobs), [
    'admit', 'build', 'reproduce', 'installed-acceptance', 'evidence', 'publish',
    'published-upgrade',
  ]);
});

test('jobs keep read, signing, and publication authorities disjoint and ordered', () => {
  const { admit, build, reproduce, evidence, publish, 'published-upgrade': publishedUpgrade } = workflow.jobs;
  assert.deepEqual(admit.permissions, { actions: 'read', contents: 'read' });
  assert.deepEqual(build.permissions, { contents: 'read' });
  assert.deepEqual(reproduce.permissions, { contents: 'read' });
  assert.deepEqual(evidence.permissions, {
    actions: 'read', attestations: 'write', contents: 'read', 'id-token': 'write',
  });
  assert.deepEqual(publish.permissions, { actions: 'read', contents: 'write' });
  assert.deepEqual(publishedUpgrade.permissions, { contents: 'read' });
  assert.equal(admit.needs, undefined);
  assert.equal(build.needs, 'admit');
  assert.equal(reproduce.needs, 'build');
  assert.deepEqual(evidence.needs, ['reproduce', 'installed-acceptance']);
  assert.equal(publish.needs, 'evidence');
  assert.equal(publishedUpgrade.needs, 'publish');
  assert.equal(publish.environment, 'binary-release');
  for (const [id, job] of Object.entries(workflow.jobs)) {
    if (id !== 'publish') assert.equal(job.environment, undefined, id);
    if (id !== 'evidence') assert.equal(job.permissions['id-token'], undefined, id);
    if (id !== 'evidence') assert.equal(job.permissions.attestations, undefined, id);
    if (id !== 'publish') assert.notEqual(job.permissions.contents, 'write', id);
  }
});

test('public upgrade proof acquires both immutable versions on exact supported hosts', () => {
  const job = workflow.jobs['published-upgrade'];
  assert.equal(job['timeout-minutes'], 30);
  assert.equal(job.strategy['fail-fast'], false);
  assert.deepEqual(job.strategy.matrix.include, [
    { os: 'ubuntu-24.04', target: 'x86_64-unknown-linux-gnu',
      previous: '0.2.1', previous_tag: 'v0.2.1' },
    { os: 'windows-2022', target: 'x86_64-pc-windows-msvc',
      previous: '0.2.1', previous_tag: 'v0.2.1' },
    { os: 'ubuntu-24.04', target: 'x86_64-unknown-linux-gnu',
      previous: '0.2.2', previous_tag: 'v0.2.2' },
    { os: 'windows-2022', target: 'x86_64-pc-windows-msvc',
      previous: '0.2.2', previous_tag: 'v0.2.2' },
  ]);
  const acquisition = steps(job, 'Acquire both exact immutable public releases')[0];
  assert.match(acquisition.run, /releases\/tags\/\$env:ZRYNA_PREVIOUS_TAG/);
  assert.match(acquisition.run, /gh release download \$env:ZRYNA_PREVIOUS_TAG/);
  assert.match(acquisition.run, /releases\/tags\/v0\.2\.3/);
  assert.match(acquisition.run, /gh release download v0\.2\.3/);
  const linux = steps(job, 'Exercise the Linux upgrade as an unprivileged user')[0];
  const windows = steps(job, 'Exercise the Windows upgrade as a standard user')[0];
  assert.match(linux.run, /run-published-upgrade-acceptance\.mjs/);
  assert.match(windows.run, /run-published-upgrade-acceptance\.mjs/);
  assert.match(linux.run, /--previous-version "\$ZRYNA_PREVIOUS_VERSION"/);
  assert.match(windows.run, /'--previous-version', \$env:ZRYNA_PREVIOUS_VERSION/);
  assert.match(windows.run, /Start-Process -FilePath \$node/);
  assert.match(windows.run, /-Credential \$credential -LoadUserProfile/);
  assert.match(windows.run, /net user \$user \/delete/);
  assert.doesNotMatch(`${linux.run}\n${windows.run}`, /cargo|target[\\/]release|target[\\/]debug/);
});

test('all external actions are immutable pins and checkouts cannot retain credentials', () => {
  const uses = Object.values(workflow.jobs)
    .flatMap((job) => job.steps.flatMap((step) => step.uses ? [step.uses] : []));
  assert(uses.length > 0);
  for (const value of uses) assert.match(value, shaUse, value);
  for (const job of Object.values(workflow.jobs)) {
    for (const step of steps(job, 'Checkout tagged workflow tooling')) {
      assert.equal(step.with['persist-credentials'], false);
      assert.equal(step.with.ref, '${{ github.workflow_sha }}');
      assert.equal(step.with.path, 'tooling');
    }
  }
  for (const step of steps(workflow.jobs.admit, 'Checkout exact tagged source')) {
    assert.equal(step.with['persist-credentials'], false);
    assert.equal(step.with.ref, '${{ github.sha }}');
    assert.equal(step.with.path, 'source');
  }
  for (const step of steps(workflow.jobs.build, 'Checkout exact tagged source')) {
    assert.equal(step.with['persist-credentials'], false);
    assert.equal(step.with.ref, '${{ github.sha }}');
    assert.equal(step.with.path, 'source');
  }
});

test('admission requires the reviewed recipe and complete integrations', () => {
  const admit = workflow.jobs.admit;
  const names = admit.steps.map(({ name }) => name).filter(Boolean);
  assert(names.indexOf('Capture protected tag identity')
    < names.indexOf('Capture exact successful protected gates'));
  assert(names.indexOf('Capture exact successful protected gates')
    < names.indexOf('Reject release until every reviewed production prerequisite is present'));
  assert(names.indexOf('Reject release until every reviewed production prerequisite is present')
    < names.indexOf('Require the exact successful production candidate'));
  assert.match(steps(admit, 'Capture protected tag identity')[0].run,
    /create-release-tag-receipt\.mjs/);
  assert.match(steps(admit, 'Capture exact successful protected gates')[0].run,
    /create-preassembly-gates\.mjs/);
  assert.match(steps(admit,
    'Reject release until every reviewed production prerequisite is present')[0].run,
  /check-release-readiness\.mjs/);
  assert.match(steps(admit, 'Require the exact successful production candidate')[0].run,
    /create-production-candidate-receipt\.mjs/);
  assert.equal(ACCEPTED_RECIPE_SHA256,
    '0f522abd343ba4e3c07a38d93c778fa7a0e30aac2295abe1f075091f6b6d7db1');

  const source = resolve('release-readiness-source');
  const sha = 'a'.repeat(40);
  const spawn = (executable, args, options) => {
    assert.equal(executable, 'git');
    const path = args.at(-1);
    const output = path === '.github/workflows/release.yml'
      ? Buffer.from(`100644 blob ${'b'.repeat(40)}\t${path}\0`)
      : Buffer.alloc(0);
    return { status: 0, stdout: options.encoding === null ? output : output.toString('utf8') };
  };
  assert.throws(() => checkReleaseReadiness({
    environment: {
      GITHUB_EVENT_NAME: 'push',
      GITHUB_REPOSITORY: 'zryna/zryna',
      GITHUB_REF: 'refs/tags/v0.2.3',
      GITHUB_REF_PROTECTED: 'true',
      GITHUB_SHA: sha,
      GITHUB_WORKFLOW_SHA: sha,
      ZRYNA_SOURCE_ROOT: source,
    },
    spawn,
  }), (error) => {
    assert.match(error.message, /R406-RELEASE-NOT-READY: missing reviewed prerequisites:/);
    for (const path of REQUIRED_RELEASE_PATHS.slice(1)) assert.match(error.message, new RegExp(path));
    return true;
  });
});

test('readiness requires accepted production semantics after exact recipe digest binding', () => {
  const accepted = readFileSync(resolve(root, 'scripts/distribution/release-recipe-v1.json'));
  const proposal = wire({
    ...parseCanonical(accepted.toString('utf8')),
    status: 'qualification-proposal', productionAdmission: 'forbidden',
  });
  const ready = readinessFixture(accepted);
  assert.deepEqual(checkReleaseReadiness({
    ...ready, acceptedRecipeSha256: sha256(accepted),
  }), { sourceCommit: ready.sourceCommit, recipeSha256: sha256(accepted) });

  assert.throws(() => checkReleaseReadiness({
    ...readinessFixture(proposal), acceptedRecipeSha256: sha256(proposal),
  }), /R406-RELEASE-NOT-READY: recipe does not carry the accepted production identity/);
});

test('two distinct clean build jobs feed platform-local byte reproduction', () => {
  assert.deepEqual(workflow.jobs.build.strategy.matrix.include, [
    { os: 'ubuntu-24.04', target: 'x86_64-unknown-linux-gnu', replica: 1 },
    { os: 'ubuntu-24.04', target: 'x86_64-unknown-linux-gnu', replica: 2 },
    { os: 'windows-2022', target: 'x86_64-pc-windows-msvc', replica: 1 },
    { os: 'windows-2022', target: 'x86_64-pc-windows-msvc', replica: 2 },
  ]);
  assert.equal(workflow.jobs.build.strategy['fail-fast'], false);
  assert.equal(workflow.jobs.build.env.CARGO_HOME,
    '${{ github.workspace }}/zryna-bootstrap-cargo');
  assert.equal(steps(workflow.jobs.build, 'Fetch locked Rust dependencies')[0]['working-directory'],
    'source');
  assert.equal(steps(workflow.jobs.build, 'Fetch locked Rust dependencies')[0].run,
    'cargo fetch --locked');
  assert.equal(steps(workflow.jobs.build, 'Bind exact Unix tools')[0].if,
    "runner.os == 'Linux'");
  assert.equal(steps(workflow.jobs.build,
    'Bind exact Windows tools and developer environment')[0].if, "runner.os == 'Windows'");
  const build = steps(workflow.jobs.build,
    'Authenticate materials, prepare the recipe, and build one clean replica')[0];
  assert.match(build.run, /run-protected-build\.mjs/);
  assert.equal(build.env.ZRYNA_TARGET, '${{ matrix.target }}');
  assert.equal(build.env.ZRYNA_REPLICA, '${{ matrix.replica }}');

  assert.deepEqual(workflow.jobs.reproduce.strategy.matrix.include, [
    { os: 'ubuntu-24.04', target: 'x86_64-unknown-linux-gnu' },
    { os: 'windows-2022', target: 'x86_64-pc-windows-msvc' },
  ]);
  const compare = steps(workflow.jobs.reproduce,
    'Compare complete archive bytes and retained deterministic evidence')[0];
  assert.equal(steps(workflow.jobs.reproduce, 'Install tagged workflow dependencies').length, 1);
  assert.match(compare.run, /compare-release-builds\.mjs/);
  assert.match(compare.run, /--first \.release\/first/);
  assert.match(compare.run, /--second \.release\/second/);
});

test('both reproduced archives pass installed relocation, execution, and tamper acceptance', () => {
  const acceptance = workflow.jobs['installed-acceptance'];
  assert.equal(acceptance.needs, 'reproduce');
  assert.deepEqual(acceptance.permissions, { contents: 'read' });
  assert.deepEqual(acceptance.strategy.matrix.include, [
    { os: 'ubuntu-24.04', target: 'x86_64-unknown-linux-gnu' },
    { os: 'windows-2022', target: 'x86_64-pc-windows-msvc' },
  ]);
  assert.equal(steps(acceptance, 'Install tagged workflow dependencies').length, 1);
  const run = steps(acceptance,
    'Verify, relocate, execute, and tamper-test the installed archive')[0];
  assert.equal(run.env.ZRYNA_TARGET, '${{ matrix.target }}');
  assert.match(run.run, /run-installed-acceptance\.mjs/);
  assert.match(run.run, /--input \.release\/reproduced/);
  assert.match(run.run, /--output \.release\/acceptance/);
  assert.match(run.run, /--work "\$\{\{ runner\.temp \}\}"/);
});

test('evidence signing precedes the sole environment-gated draft publisher', () => {
  const evidence = workflow.jobs.evidence;
  assert.equal(steps(evidence, 'Attest Linux archive').length, 1);
  assert.equal(steps(evidence, 'Attest Windows archive').length, 1);
  for (const name of ['Attest Linux archive', 'Attest Windows archive']) {
    assert.equal(steps(evidence, name)[0].with['create-storage-record'], false);
  }
  assert.equal(steps(evidence, 'Install the pinned independent signature verifier').length, 1);
  assert.equal(steps(workflow.jobs.publish,
    'Install the pinned independent signature verifier').length, 1);
  const signing = steps(evidence, 'Sign and verify checksums and release notes')[0];
  assert.equal(signing.with.verify, true);
  assert.equal(signing.with['release-signing-artifacts'], false);
  assert.equal(signing.with['upload-signing-artifacts'], false);
  assert.equal(signing.with['verify-oidc-issuer'],
    'https://token.actions.githubusercontent.com');
  assert.equal(signing.with['verify-cert-identity'],
    '${{ github.server_url }}/${{ github.repository }}/.github/workflows/release.yml@${{ github.ref }}');
  assert.match(signing.with.inputs, /SHA256SUMS\n/);
  assert.match(signing.with.inputs, /RELEASE_NOTES\.md\n?$/);
  const evidenceNames = evidence.steps.map(({ name }) => name).filter(Boolean);
  const envelopeSigning = evidenceNames.indexOf('Sign and verify the canonical release envelope');
  const canonicalization = evidenceNames.indexOf('Canonicalize the envelope signature bundle');
  const independentVerification = evidenceNames.indexOf(
    'Verify exact signed publication inventory without signing authority');
  assert(envelopeSigning < canonicalization && canonicalization < independentVerification);
  assert.equal(steps(evidence, 'Canonicalize the envelope signature bundle').length, 1);
  assert.match(steps(evidence, 'Canonicalize the envelope signature bundle')[0].run,
    /canonicalize-envelope-signature\.mjs --directory \.release\/publication/);

  const publishNames = workflow.jobs.publish.steps.map(({ name }) => name).filter(Boolean);
  const phases = [
    'Verify signed bundle before any release mutation',
    'Create one draft prerelease',
    'Upload the exact envelope allowlist',
    'Re-read and verify the complete draft',
    'Publish the verified prerelease once',
  ];
  assert.deepEqual(publishNames.slice(-phases.length), phases);
  for (const [name, phase] of [
    ['Create one draft prerelease', 'create-draft'],
    ['Upload the exact envelope allowlist', 'upload'],
    ['Re-read and verify the complete draft', 'verify-draft'],
    ['Publish the verified prerelease once', 'publish'],
  ]) assert.match(steps(workflow.jobs.publish, name)[0].run, new RegExp(`--phase ${phase}`));
});

test('all workflow artifacts are immutable short-lived exact-name handoffs', () => {
  for (const job of Object.values(workflow.jobs)) {
    for (const step of job.steps.filter(({ uses }) => uses?.startsWith('actions/upload-artifact@'))) {
      assert.equal(step.with['if-no-files-found'], 'error');
      assert.equal(step.with['compression-level'], 0);
      assert.equal(step.with['include-hidden-files'], false);
      assert.equal(step.with['retention-days'], 1);
      assert.match(step.with.name, /\$\{\{ github\.run_id \}\}-\$\{\{ github\.run_attempt \}\}$/);
      assert.equal(step.with.overwrite, undefined);
    }
    for (const step of job.steps.filter(({ uses }) => uses?.startsWith('actions/download-artifact@'))) {
      assert.equal(typeof step.with.name, 'string');
      assert.equal(step.with.pattern, undefined);
      assert.equal(step.with['merge-multiple'], undefined);
    }
  }
});
