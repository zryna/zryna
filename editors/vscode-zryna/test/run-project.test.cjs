'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const os = require('node:os');
const { createHash } = require('node:crypto');
const { runProject, verifyOutput } = require('../src/run-project.cjs');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');

test('compiler scaffolds and authenticates saved bytes before run; output binding rejects tampering', async t => {
  const storage = await fs.mkdtemp(path.join(os.tmpdir(), 'zryna-run-test-'));
  t.after(() => fs.rm(storage, { recursive: true, force: true }));
  const bytes = Buffer.from('export function sum(a:i32,b:i32):i32{return a+b;}');
  const selection = { name: 'sum', target: 'javascript', args: ['3', '8'] };
  const calls = [];
  let project;
  let manifest;
  let output;
  const call = async (_, args) => {
    calls.push(args);
    if (args[0] === 'new') {
      project = args[1];
      await fs.mkdir(path.join(project, 'src'), { recursive: true });
      await fs.writeFile(path.join(project, 'src/main.zry'), 'template');
      await fs.writeFile(path.join(project, 'zryna.package.json'), JSON.stringify({
        format: 'zryna.package.v1', name: path.basename(project), files: [{ path: 'src/main.zry' }],
        dependencies: [], compatibility: { compiler: '0.2.3', profile: 'i32-v1' },
      }));
    } else if (args[0] === 'package') {
      const record = JSON.parse(await fs.readFile(path.join(project, 'zryna.package.json')));
      assert.equal(record.files[0].size, bytes.length);
      assert.equal(record.files[0].sha256, hash(bytes));
      assert.deepEqual(await fs.readFile(path.join(project, 'src/main.zry')), bytes);
    } else {
      output = path.join(project, '.zryna/out/result.run');
      await fs.mkdir(path.join(output, 'javascript'), { recursive: true });
      const emitted = Buffer.from('export function sum(a,b){return a+b;}');
      await fs.writeFile(path.join(output, 'javascript/result.mjs'), emitted);
      manifest = { version: 1, profile: 'zryna-m1-cli-v1', command: 'run', entrypoint: 'src/main.zry',
        source_sha256: hash(bytes), stem: 'result', targets: ['javascript'],
        invocation: { export: 'sum', arguments: [{ type: 'i32', value: 3 }, { type: 'i32', value: 8 }] },
        artifacts: [{ target: 'javascript', path: 'javascript/result.mjs', bytes: emitted.length, sha256: hash(emitted) }] };
      await fs.writeFile(path.join(output, 'zryna-manifest-v1.json'), JSON.stringify(manifest));
    }
    return { stdout: 'javascript: i32 11', stderr: '' };
  };
  const result = await runProject({ compiler: path.resolve('compiler'), storage, bytes, selection }, call);
  assert.deepEqual(calls.map(args => args[0]), ['new', 'package', 'run']);
  assert.ok(calls[2].includes('--arg=i32:8'));
  assert.equal(result.file, path.join(output, 'javascript/result.mjs'));
  for (const change of [{ source_sha256: 'wrong' }, { invocation: { export: 'other' } },
    { artifacts: [{ target: 'javascript', path: '../../outside.mjs' }] }]) {
    await fs.writeFile(path.join(output, 'zryna-manifest-v1.json'), JSON.stringify({ ...manifest, ...change }));
    await assert.rejects(verifyOutput(project, selection, hash(bytes)), /does not match/);
  }
  await fs.writeFile(path.join(output, 'zryna-manifest-v1.json'), JSON.stringify(manifest));
  await fs.writeFile(result.file, 'tampered');
  await assert.rejects(verifyOutput(project, selection, hash(bytes)), /hash mismatch/);
});

test('invalid selections and cancellation never reach scaffold', async () => {
  const input = { compiler: path.resolve('compiler'), storage: path.resolve('storage'),
    bytes: Buffer.from('export function main():i32{return 1;}'), selection: { name: 'main', target: 'native', args: [] } };
  const call = () => assert.fail('must not launch');
  await assert.rejects(runProject(input, call), /Invalid/);
  input.selection.target = 'javascript';
  input.token = { isCancellationRequested: true };
  await assert.rejects(runProject(input, call), /cancelled/);
});

test('cancellation after scaffold prevents package mutation and execution', async t => {
  const storage = await fs.mkdtemp(path.join(os.tmpdir(), 'zryna-run-cancel-'));
  t.after(() => fs.rm(storage, { recursive: true, force: true }));
  const token = { isCancellationRequested: false };
  const calls = [];
  await assert.rejects(runProject({ compiler: path.resolve('compiler'), storage,
    bytes: Buffer.from('export function main():i32{return 1;}'),
    selection: { name: 'main', target: 'javascript', args: [] }, token }, async (_, args) => {
    calls.push(args[0]);
    token.isCancellationRequested = true;
  }), /cancelled/);
  assert.deepEqual(calls, ['new']);
  assert.deepEqual(await fs.readdir(storage), []);
});
