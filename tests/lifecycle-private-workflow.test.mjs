import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { parseDocument } from 'yaml';

test('parsed workflow keeps private operations manual and selection out of shell text', () => {
  const document = parseDocument(readFileSync(new URL('../.github/workflows/native-host-facts.yml', import.meta.url), 'utf8'), { uniqueKeys: true });
  assert.deepEqual(document.errors, []);
  const workflow = document.toJS(), job = workflow.jobs.lifecycle;
  assert.equal(job.if, "github.event_name == 'workflow_dispatch' && inputs.lifecycle_operation != 'none'");
  assert.deepEqual(workflow.on.workflow_dispatch.inputs.lifecycle_operation.options, ['none', 'capture', 'execute']);
  assert.equal(workflow.on.workflow_dispatch.inputs.lifecycle_operation.default, 'none');
  assert.equal(job.env.LIFECYCLE_SELECTION, '${{ inputs.lifecycle_selection }}');
  assert.equal(job.env.LIFECYCLE_OPERATION, '${{ inputs.lifecycle_operation }}');
  assert.equal(job.env.ZRYNA_NATIVE_RUNNER_ENVIRONMENT, undefined);
  const execution = job.steps.find(step => step.id === 'execute');
  assert.equal(execution.if, "inputs.lifecycle_operation == 'execute'");
  assert.equal(execution.env.ZRYNA_NATIVE_RUNNER_ENVIRONMENT, '${{ runner.environment }}');
  assert.equal(execution.env.GITHUB_TOKEN, '${{ github.token }}');
  for (const value of Object.values(workflow.jobs)) {
    assert(!JSON.stringify(value.env ?? {}).includes('runner.environment'));
    for (const step of value.steps) {
      assert(!step.uses?.startsWith('actions/download-artifact@'));
      assert(!step.run?.includes('${{ inputs.'));
      assert(!/\b(?:dpkg-deb|tar|unzip)\b/.test(step.run ?? ''));
    }
  }
  const capture = job.steps.find(step => step.name === 'Capture immutable lifecycle inputs');
  assert.equal(capture.if, "inputs.lifecycle_operation == 'capture'");
  assert.equal(capture.run, 'node scripts/stability-gates/lifecycle-private.mjs');
  const dependencies = job.steps.findIndex(step => step.run === 'pnpm install --frozen-lockfile');
  assert(dependencies > job.steps.findIndex(step => step.uses?.startsWith('pnpm/action-setup@')));
  assert(dependencies < job.steps.indexOf(capture), 'each isolated lifecycle job needs its own static imports');
  const uploads = job.steps.filter(step => step.uses?.startsWith('actions/upload-artifact@'));
  assert.equal(uploads.length, 2);
  assert.match(uploads[1].with.path, /\/observations\/\*\*/u);
  const contracts = workflow.jobs.contracts;
  assert.deepEqual(contracts.strategy.matrix.os, ['ubuntu-24.04', 'windows-2022']);
  assert(contracts.steps.some(step => step.run === 'node --test tests/lifecycle-private.test.mjs tests/lifecycle-private-platform.test.mjs tests/lifecycle-private-workflow.test.mjs'));
  assert(contracts.steps.some(step => step.run === 'pnpm install --frozen-lockfile'));
  assert(contracts.steps.some(step => step.run === "python -I -B -m unittest discover -s examples/playground/restricted/daemon/lifecycle-owner -p 'test_*.py'"));
  assert(contracts.steps.some(step => step.run === 'node --test tests/lifecycle-material-auth.test.mjs'));
  for (const path of ['examples/playground/restricted/**', 'scripts/stability-gates/lifecycle-owner-*',
    'tests/lifecycle-owner-*', 'tests/lifecycle-material-auth.test.mjs']) {
    assert(workflow.on.pull_request.paths.includes(path), 'owning synthetic contracts must run on ' + path);
  }
});

test('public observer contract and privileged authentication ordering remain explicit', () => {
  const document = parseDocument(readFileSync(new URL('../.github/workflows/native-host-facts.yml', import.meta.url), 'utf8'));
  const observer = document.toJS().jobs.observe;
  assert.equal(observer.needs, 'contracts'); assert.equal(observer['timeout-minutes'], 5);
  const sudo = observer.steps.find(step => step.name === 'Collect exact bounded sudo observer only for denied namespace reads');
  assert.equal(sudo.if, "steps.ordinary.outputs.namespace_denied == 'true'");
  const projected = sudo.run.match(/--preserve-env=([^\s]+)/u)[1].split(',');
  assert.deepEqual(projected, ['GITHUB_REPOSITORY', 'GITHUB_SHA', 'GITHUB_REF', 'GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT',
    'GITHUB_JOB', 'GITHUB_WORKFLOW_REF', 'GITHUB_WORKFLOW_SHA', 'ImageOS', 'ImageVersion', 'RUNNER_OS', 'RUNNER_ARCH',
    'ZRYNA_NATIVE_SOURCE_COMMIT', 'ZRYNA_NATIVE_RUNNER_ENVIRONMENT']);
  assert.match(sudo.run, /12s[\s\S]*python3 -I -S -B scripts\/stability-gates\/collect-native-host-facts\.py/u);
  const source = name => readFileSync(new URL('../scripts/stability-gates/lifecycle-private-' + name + '.mjs', import.meta.url), 'utf8');
  const execute = source('execute'), artifact = source('artifact');
  assert(execute.indexOf('validateAuth(input.auth') < execute.indexOf('protectedWork()'));
  assert(execute.indexOf('verifyCompilerSignature(envelope') < execute.indexOf("command('/usr/bin/dpkg-deb'"));
  assert(execute.indexOf('await verifyArchive(') < execute.indexOf("command('/usr/bin/tar'"));
  assert(artifact.indexOf('\n  verifyCaptureMetadata(e, run') < artifact.indexOf('await downloadPublicationArchive'));
  assert(artifact.indexOf('const entries = zipEntries(zip)') < artifact.indexOf('zipEntries(zip, (entry'));
  assert(artifact.indexOf('await decodeTar(') < artifact.indexOf('return files;'));
});
