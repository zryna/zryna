import assert from 'node:assert/strict';
import { linkSync, lstatSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pinFile } from '../scripts/stability-gates/release-build-input.mjs';
import { snapshot as snapshotWindowsBoundFile } from '../scripts/stability-gates/cli-performance-windows-binding.mjs';
import test from 'node:test';
import { parse as parseYaml } from 'yaml';
import { retainReleaseCli, validateRetainedCliBinding } from '../scripts/stability-gates/cli-performance-hosted-adapter.mjs';

const workflow = parseYaml(readFileSync(new URL('../.github/workflows/cli-performance-collector.yml', import.meta.url), 'utf8'));
test('workflow excludes the complete Cargo target tree while retaining independent CLI evidence', () => {
  const upload = workflow.jobs.collect.steps.find(step =>
    step.uses?.startsWith('actions/upload-artifact@') && step.with.path.includes('m418-hosted-observations'));
  const paths = upload.with.path.trim().split('\n');
  assert(paths.includes('${{ runner.temp }}/m418-hosted-observations'));
  assert(paths.includes('!${{ runner.temp }}/m418-hosted-observations/capture/release-build/cargo-target/**'));
  assert(!paths.some(path => path.startsWith('!') && path.includes('retained-release-cli')));
});
test('retained CLI binding requires stable exact built bytes and measures only its private copy', () => {
  const source = { path: '/evidence/release-build/cargo-target/host/release/zryna', bytes: 42,
    mode: '755', sha256: '1'.repeat(64) };
  const record = { format: 'zryna.cli-performance-retained-release-cli.v2', links: { builtBefore: 2, builtAfter: 2, copy: 1 }, sourceBefore: source, sourceAfter: { ...source },
    copy: { ...source, path: '/evidence/retained-release-cli/zryna' },
    measuredPath: '/evidence/retained-release-cli/zryna', targetDirectory: '/evidence/release-build/cargo-target' };
  validateRetainedCliBinding(record);
  for (const [key, value] of [['bytes', 43], ['mode', '644'], ['sha256', '2'.repeat(64)]]) {
    assert.throws(() => validateRetainedCliBinding({ ...record, copy: { ...record.copy, [key]: value } }));
    assert.throws(() => validateRetainedCliBinding({ ...record, sourceAfter: { ...source, [key]: value } }));
  }
  assert.throws(() => validateRetainedCliBinding({ ...record, copy: { ...record.copy, path: source.path } }));
  for (const links of [{ ...record.links, copy: 2 }, { ...record.links, builtAfter: 3 },
    { ...record.links, builtBefore: 0 }]) assert.throws(() => validateRetainedCliBinding({ ...record, links }));
  assert.throws(() => validateRetainedCliBinding({ ...record, measuredPath: source.path }));
  assert.throws(() => validateRetainedCliBinding({ ...record, measuredPath: '/other/zryna' }));
  assert.throws(() => validateRetainedCliBinding({ ...record, format: 'zryna.cli-performance-retained-release-cli.v1' }));
  assert.throws(() => validateRetainedCliBinding({ ...record, sourceBefore: { ...source, path: '/other/zryna' },
    sourceAfter: { ...source, path: '/other/zryna' } }));
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

test('actual multiply linked Cargo image becomes a byte-identical single-link measured copy', () => {
  const root = mkdtempSync(join(realpathSync(process.env.RUNNER_TEMP ?? tmpdir()), 'm418-retained-image-'));
  try {
    const target = join(root, 'cargo-target'); mkdirSync(target);
    const output = join(root, 'capture'); mkdirSync(output);
    const source = join(target, 'zryna.exe'); writeFileSync(source, 'isolated image-copy fixture, never executed');
    linkSync(source, join(target, 'cargo-deps-peer.exe'));
    assert.equal(lstatSync(source).nlink, 2);
    const build = { cli: { after: pinFile(source) }, environment: { CARGO_TARGET_DIR: target } };
    assert.throws(() => snapshotWindowsBoundFile(build.cli.after), /private single-link operator input/u);
    const retained = retainReleaseCli(output, build);
    assert.equal(snapshotWindowsBoundFile(retained.copy).identity.nlink, 1);
    assert.equal(lstatSync(retained.measuredPath).nlink, 1, 'measured input satisfies unchanged private-file guard');
    assert.equal(retained.measuredPath, retained.copy.path);
    assert.equal(retained.sourceBefore.path, source);
    assert.deepEqual(retained.links, { builtBefore: 2, builtAfter: 2, copy: 1 });
    assert(readFileSync(retained.measuredPath).equals(readFileSync(source)));
    assert.equal(retained.copy.sha256, build.cli.after.sha256);
    validateRetainedCliBinding(retained);
  } finally { rmSync(root, { recursive: true, force: true }); }
});
