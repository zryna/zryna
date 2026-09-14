import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { parseDocument } from 'yaml';

const document = parseDocument(readFileSync(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8'));
assert.deepEqual(document.errors, []);
const workflow = document.toJS();
const job = workflow.jobs['published-release-clean-host'];

function step(name) {
  return job.steps.find((candidate) => candidate.name === name);
}

test('published release acceptance uses exact clean host images and immutable acquisition', () => {
  assert.equal(job.needs, 'route-contracts');
  assert.equal(job.if, "needs.route-contracts.outputs.distribution_release == 'true'");
  assert.equal(job['runs-on'], '${{ matrix.os }}');
  assert.equal(job['timeout-minutes'], 30);
  assert.deepEqual(job.permissions, { contents: 'read' });
  assert.equal(job.strategy['fail-fast'], false);
  assert.deepEqual(job.strategy.matrix.include, [
    { os: 'ubuntu-24.04', target: 'x86_64-unknown-linux-gnu' },
    { os: 'windows-2022', target: 'x86_64-pc-windows-msvc' },
  ]);
  assert(job.steps.some(({ uses }) => uses
    === 'sigstore/cosign-installer@6f9f17788090df1f26f669e9d70d6ae9567deba6'));
  const acquisition = step('Acquire the exact immutable public release');
  assert.deepEqual(acquisition.env, { GH_TOKEN: '${{ github.token }}' });
  assert.match(acquisition.run, /releases\/tags\/v0\.2\.1/);
  assert.match(acquisition.run, /gh release download v0\.2\.1 --repo zryna\/zryna/);
});

test('each host runs the same published bytes without repository compiler substitution', () => {
  const linux = step('Verify and exercise the Linux release as an unprivileged user');
  assert.equal(linux.if, "runner.os == 'Linux'");
  assert.match(linux.run, /run-published-acceptance\.mjs/);
  assert.match(linux.run, /cosign_path="\$\(command -v cosign\)"/);
  assert.match(linux.run, /--cosign "\$cosign_path"/);
  assert.match(linux.run, /hostile-bin\/zryna/);
  assert.match(linux.run, /PATH="\$\(realpath \.release\/hostile-bin\):\/usr\/bin:\/bin"/);
  assert.doesNotMatch(linux.run, /cargo|target\/release|target\/debug/);

  const windows = step('Verify and exercise the Windows release as a standard user');
  assert.equal(windows.if, "runner.os == 'Windows'");
  for (const pattern of [
    /\$passwordText = "Zr423-\$env:GITHUB_RUN_ATTEMPT!"/,
    /net user \$user \$passwordText \/add/,
    /Start-Process -FilePath \$node/,
    /-Credential \$credential -LoadUserProfile/,
    /hostile-bin/,
    /\$env:PATH = "\$hostile;\$env:PATH"/,
    /icacls \$userRoot \/setowner \$user/,
    /icacls \$outputRoot \/setowner \$user/,
    /run-published-acceptance\.mjs/,
    /net user \$user \/delete/,
  ]) assert.match(windows.run, pattern);
  assert.doesNotMatch(windows.run, /cargo|target\\release|target\\debug/);
});

test('clean host receipts are exact short-lived evidence', () => {
  const upload = step('Upload exact clean-host receipt');
  assert.equal(upload.uses,
    'actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a');
  assert.match(upload.with.name,
    /^published-beta-clean-host-\$\{\{ matrix\.target \}\}-\$\{\{ github\.run_id \}\}-\$\{\{ github\.run_attempt \}\}$/);
  assert.equal(upload.with.path, 'release-evidence/*-output/published-clean-host-acceptance.json');
  assert.equal(upload.with['if-no-files-found'], 'error');
  assert.equal(upload.with['compression-level'], 0);
  assert.equal(upload.with['include-hidden-files'], false);
  assert.equal(upload.with['retention-days'], 7);
});
