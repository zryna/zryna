import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { parse as parseYaml } from 'yaml';
import { validateRetainedCliBinding } from '../scripts/stability-gates/cli-performance-hosted-adapter.mjs';

const workflow = parseYaml(readFileSync(new URL('../.github/workflows/cli-performance.yml', import.meta.url), 'utf8'));
test('workflow excludes the complete Cargo target tree while retaining independent CLI evidence', () => {
  const upload = workflow.jobs.collect.steps.find(step =>
    step.uses?.startsWith('actions/upload-artifact@') && step.with.path.includes('m418-hosted-observations'));
  const paths = upload.with.path.trim().split('\n');
  assert(paths.includes('${{ runner.temp }}/m418-hosted-observations'));
  assert(paths.includes('!${{ runner.temp }}/m418-hosted-observations/capture/release-build/cargo-target/**'));
  assert(!paths.some(path => path.startsWith('!') && path.includes('retained-release-cli')));
});
test('retained CLI binding requires stable exact bytes mode hash and preserves original measured path', () => {
  const source = { path: '/evidence/release-build/cargo-target/host/release/zryna', bytes: 42,
    mode: '755', sha256: '1'.repeat(64) };
  const record = { sourceBefore: source, sourceAfter: { ...source },
    copy: { ...source, path: '/evidence/retained-release-cli/zryna' },
    measuredPath: source.path, targetDirectory: '/evidence/release-build/cargo-target' };
  validateRetainedCliBinding(record);
  for (const [key, value] of [['bytes', 43], ['mode', '644'], ['sha256', '2'.repeat(64)]]) {
    assert.throws(() => validateRetainedCliBinding({ ...record, copy: { ...record.copy, [key]: value } }));
    assert.throws(() => validateRetainedCliBinding({ ...record, sourceAfter: { ...source, [key]: value } }));
  }
  assert.throws(() => validateRetainedCliBinding({ ...record, copy: { ...record.copy, path: source.path } }));
  assert.throws(() => validateRetainedCliBinding({ ...record, measuredPath: record.copy.path }));
});
test('workflow uses operatorInput without assigning PowerShell automatic input', () => {
  const run = workflow.jobs.collect.steps.find(step => step.name === 'Assemble observed operator inputs and collect one fresh round').run;
  assert(run.includes('$operatorInput = @{')); assert(run.includes('($operatorInput | ConvertTo-Json'));
  assert(!/\$input\s*=/iu.test(run));
});

test('artifact search roots keep Linux absolute provider paths out of Windows uploads', () => {
  const uploads = workflow.jobs.collect.steps.filter(step => step.uses?.startsWith('actions/upload-artifact@'));
  const common = uploads.find(step => step.with.path.includes('m418-hosted-observations'));
  assert.equal(common.if, 'always()');
  assert(!common.with.path.includes('/tmp/m418-linux-provider-'));
  const provider = uploads.find(step => step.with.path.includes('/tmp/m418-linux-provider-'));
  assert.equal(provider.if, "always() && runner.os == 'Linux'");
  assert.notEqual(common.with.name, provider.with.name);
});
