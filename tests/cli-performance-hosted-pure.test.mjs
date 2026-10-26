import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { parse as parseYaml } from 'yaml';
import { baselineRound, collectorEnvironment, observedPin, windowsAdmission } from '../scripts/stability-gates/cli-performance-hosted-adapter.mjs';
import { imageProjection } from '../scripts/stability-gates/cli-performance-host.mjs';
const workflow = parseYaml(readFileSync(new URL('../.github/workflows/cli-performance.yml', import.meta.url), 'utf8'));
test('workflow expands four separate fresh jobs with finite new budget and PR concurrency', () => {
  const job = workflow.jobs.collect;
  assert.deepEqual(job.strategy.matrix.os, ['ubuntu-24.04', 'windows-2022']);
  assert.deepEqual(job.strategy.matrix.round, ['baseline-round-1', 'baseline-round-2']);
  assert.equal(job.strategy['fail-fast'], false); assert.equal(job['timeout-minutes'], 60);
  assert.equal(workflow.concurrency['cancel-in-progress'], true);
  assert(workflow.concurrency.group.includes('github.event.pull_request.number'));
  assert.deepEqual(workflow.permissions, { contents: 'read' });
});
test('workflow pins actions tools and exact checkout with dependency preparation before operator', () => {
  const steps = workflow.jobs.collect.steps;
  for (const step of steps.filter(row => row.uses)) assert.match(step.uses, /@[a-f0-9]{40}$/u);
  const checkout = steps.find(row => row.uses?.startsWith('actions/checkout@'));
  assert.equal(checkout.with['persist-credentials'], false); assert.equal(checkout.with['fetch-depth'], 0);
  assert(checkout.with.ref.includes('github.event.pull_request.head.sha'));
  assert.equal(steps.find(row => row.uses?.startsWith('actions/setup-node@')).with['node-version'], '22.22.1');
  assert.equal(String(steps.find(row => row.uses?.startsWith('dtolnay/')).with.toolchain), '1.97.1');
  assert.equal(steps.find(row => row.uses?.startsWith('pnpm/')).with.version, '11.18.0');
  const operator = steps.findIndex(row => row.name === 'Assemble observed operator inputs and collect one fresh round');
  for (const run of ['pnpm install --frozen-lockfile', 'cargo fetch --locked'])
    assert(steps.findIndex(row => row.run === run) < operator);
});
test('workflow invokes unchanged Windows calibration once and preserves wrapper and failures', () => {
  const steps = workflow.jobs.collect.steps;
  const calibration = steps.filter(row => row.run?.includes('& ./scripts/stability-gates/windows-job-metrics-v1/calibration/hosted.ps1'));
  assert.equal(calibration.length, 1); assert.equal(calibration[0].if, "runner.os == 'Windows'");
  for (const key of ['UniversalCRTSdkDir', 'UCRTVersion', 'WindowsSdkDir', 'WindowsSDKVersion'])
    assert(calibration[0].run.includes(key));
  const run = steps.find(row => row.name === 'Assemble observed operator inputs and collect one fresh round').run;
  assert(run.includes('/usr/bin/python3 $owner $node $entry'));
  assert(run.includes('operator.stderr.raw')); assert(run.includes('if ($code -ne 0)'));
  assert.equal(steps.at(-1).if, 'always()'); assert.equal(steps.at(-1).with['compression-level'], 0);
});
test('only the two unqualified observation round labels are accepted', () => {
  for (const round of ['baseline-round-1', 'baseline-round-2']) assert.equal(baselineRound(round), round);
  for (const round of ['held-out-candidate', 'io-qualification', 'production']) assert.throws(() => baselineRound(round));
});
test('collector environments project exact platform keys and exclude wrappers flags and credentials', () => {
  const env = Object.fromEntries(['PATH','HOME','TMPDIR','CARGO_HOME','RUSTUP_HOME','SystemRoot','WINDIR',
    'TEMP','TMP','USERPROFILE','LIB','INCLUDE','RUSTC_WRAPPER','RUSTFLAGS','GITHUB_TOKEN'].map(key => [key, key]));
  const linux = collectorEnvironment(env, '/pinned/rustc', false);
  const windows = collectorEnvironment(env, 'C:\\pinned\\rustc.exe', true);
  assert.deepEqual(Object.keys(linux), ['PATH','HOME','TMPDIR','CARGO_HOME','RUSTUP_HOME','RUSTUP_TOOLCHAIN','RUSTC']);
  for (const selected of [linux, windows]) for (const key of ['RUSTC_WRAPPER','RUSTFLAGS','GITHUB_TOKEN'])
    assert(!Object.hasOwn(selected, key));
  assert.equal(windows.RUSTUP_TOOLCHAIN, '1.97.1'); assert.equal(windows.RUSTC, 'C:\\pinned\\rustc.exe');
});
test('Windows operator admission has exact binding schema and helper source record', () => {
  const capture = { tools: {}, sourceCommit: '1'.repeat(40), sourceTree: '2'.repeat(40), catalogueSha256: '3'.repeat(64),
    artifactOracleSha256: '4'.repeat(64), reviewRecordSha256: '5'.repeat(64) };
  const admission = windowsAdmission(capture);
  assert.deepEqual(Object.keys(admission).sort(), ['format','tools','helperBuiltFromSha256','sourceCommit','sourceTree',
    'catalogueSha256','artifactOracleSha256','reviewRecordSha256'].sort());
  assert.equal(admission.helperBuiltFromSha256, 'ea0a832ad83b3d6473c99a359da96fb5f7a1587d1c76254f12e4a9dbeb188e3a');
  assert.deepEqual(observedPin({ path: '/actual', bytes: 7, sha256: '6'.repeat(64), mode: '755' }),
    { path: '/actual', bytes: 7, sha256: '6'.repeat(64) });
});
test('host image projection exposes missing observations without invented image claims', () => {
  const projection = imageProjection({ ImageOS: 'ubuntu24', ImageVersion: 'actual-image' });
  assert.deepEqual(projection.ImageOS, { status: 'observed-environment', value: 'ubuntu24' });
  assert.equal(projection.RUNNER_OS.status, 'unavailable'); assert(!Object.hasOwn(projection, 'exclusiveHost'));
});
