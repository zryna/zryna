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
  const selection = { profile: 'i32-v1', name: 'sum', target: 'javascript',
    args: [{ type: 'i32', value: '3' }, { type: 'i32', value: '8' }], resultType: 'i32' };
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
      await fs.writeFile(path.join(project, 'zryna.lock.json'), JSON.stringify({
        format: 'zryna.lock.v1', compatibility: record.compatibility, root: 'a'.repeat(64), packages: [{}],
      }));
    } else {
      output = path.join(project, '.zryna/out/result.run');
      await fs.mkdir(path.join(output, 'javascript'), { recursive: true });
      const emitted = Buffer.from('export function sum(a,b){return a+b;}');
      await fs.writeFile(path.join(output, 'javascript/result.mjs'), emitted);
      manifest = { version: 1, profile: 'zryna-m1-cli-v1', command: 'run', entrypoint: 'src/main.zry',
        source_sha256: hash(bytes), stem: 'result', targets: ['javascript'],
        invocation: { export: 'sum', arguments: [{ type: 'i32', value: 3 }, { type: 'i32', value: 8 }] },
        artifacts: [{ target: 'javascript', kind: 'ecmascript-module', path: 'javascript/result.mjs', bytes: emitted.length, sha256: hash(emitted) }],
        results: [{ target: 'javascript', outcome: { kind: 'returned', type: 'i32', value: 11 } }] };
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
  await fs.writeFile(path.join(project, 'src/main.zry'), 'tampered');
  await assert.rejects(verifyOutput(project, selection, hash(bytes)), /authenticated saved source/);
});

test('M2 scaffold, typed invocation and manifest-v2 binding reject foreign graph or result', async t => {
  const storage = await fs.mkdtemp(path.join(os.tmpdir(), 'zryna-run-m2-'));
  t.after(() => fs.rm(storage, { recursive: true, force: true }));
  const bytes = Buffer.from('export function choose(active:bool,count:i32):bool{return active;}');
  const selection = { profile: 'control-flow-v1', name: 'choose', target: 'webassembly',
    args: [{ type: 'bool', value: 'false' }, { type: 'i32', value: '5' }], resultType: 'bool' };
  let project;
  let output;
  let manifest;
  const calls = [];
  const call = async (_, args) => {
    calls.push(args);
    if (args[0] === 'new') {
      project = args[1];
      await fs.mkdir(path.join(project, 'src'), { recursive: true });
      await fs.writeFile(path.join(project, 'src/main.zry'), 'template');
      await fs.writeFile(path.join(project, 'zryna.package.json'), JSON.stringify({
        format: 'zryna.package.v1', name: path.basename(project), files: [{ path: 'src/main.zry' }],
        dependencies: [], compatibility: { compiler: '0.2.3', profile: 'i32-v1',
          targets: ['javascript', 'native-linux-x86_64', 'webassembly'] },
      }));
    } else if (args[0] === 'package') {
      const record = JSON.parse(await fs.readFile(path.join(project, 'zryna.package.json')));
      assert.equal(record.compatibility.profile, 'control-flow-v1');
      assert.deepEqual(record.compatibility.targets, ['javascript', 'webassembly']);
      assert.equal(record.files[0].sha256, hash(bytes));
      await fs.writeFile(path.join(project, 'zryna.lock.json'), JSON.stringify({
        format: 'zryna.lock.v1', compatibility: record.compatibility, root: 'a'.repeat(64), packages: [{}],
      }));
    } else {
      output = path.join(project, '.zryna/out/result.run');
      await fs.mkdir(path.join(output, 'webassembly'), { recursive: true });
      const emitted = Buffer.from([0, 97, 115, 109]);
      await fs.writeFile(path.join(output, 'webassembly/result.wasm'), emitted);
      manifest = { version: 2, profile: 'zryna-control-flow-v1', command: 'run',
        entrypoint: 'src/main.zry', graph_sha256: 'a'.repeat(64),
        sources: [{ id: 0, path: 'src/main.zry', sha256: hash(bytes) }], edges: [], stem: 'result',
        targets: ['webassembly'], artifacts: [{ target: 'webassembly', kind: 'core-webassembly-module',
          path: 'webassembly/result.wasm', bytes: emitted.length, sha256: hash(emitted) }],
        invocation: { export: 'choose', arguments: [{ type: 'bool', value: false }, { type: 'i32', value: 5 }] },
        results: [{ target: 'webassembly', outcome: { kind: 'returned', type: 'bool', value: false } }], diagnostics: [] };
      await fs.writeFile(path.join(output, 'zryna-manifest-v2.json'), JSON.stringify(manifest));
    }
    return { stdout: 'webassembly: bool false', stderr: '' };
  };
  const result = await runProject({ compiler: path.resolve('compiler'), storage, bytes, selection }, call);
  assert.equal(result.results[0].outcome.value, false);
  assert.deepEqual(calls.map(args => args[0]), ['new', 'package', 'run']);
  assert.ok(calls[2].includes('--arg=bool:false'));
  assert.ok(calls[2].includes('--arg=i32:5'));
  assert.deepEqual(calls[2].slice(-2), ['--profile', 'control-flow-v1']);
  for (const change of [{ graph_sha256: 'invalid' }, { sources: [{ id: 0, path: 'src/other.zry', sha256: hash(bytes) }] },
    { edges: [{ importer: 'src/main.zry', target: 'src/other.zry' }] },
    { invocation: { export: 'choose', arguments: [{ type: 'bool', value: true }, { type: 'i32', value: 5 }] } },
    { results: [{ target: 'webassembly', outcome: { kind: 'returned', type: 'i32', value: 0 } }] }]) {
    await fs.writeFile(path.join(output, 'zryna-manifest-v2.json'), JSON.stringify({ ...manifest, ...change }));
    await assert.rejects(verifyOutput(project, selection, hash(bytes)), /does not match/);
  }
  await fs.writeFile(path.join(output, 'zryna-manifest-v2.json'), JSON.stringify(manifest));
  const lock = JSON.parse(await fs.readFile(path.join(project, 'zryna.lock.json')));
  lock.compatibility.profile = 'i32-v1';
  await fs.writeFile(path.join(project, 'zryna.lock.json'), JSON.stringify(lock));
  await assert.rejects(verifyOutput(project, selection, hash(bytes)), /authenticated saved source/);
});

test('invalid selections and cancellation never reach scaffold', async () => {
  const input = { compiler: path.resolve('compiler'), storage: path.resolve('storage'),
    bytes: Buffer.from('export function main():i32{return 1;}'),
    selection: { profile: 'i32-v1', name: 'main', target: 'native', args: [], resultType: 'i32' } };
  const call = () => assert.fail('must not launch');
  await assert.rejects(runProject(input, call), /Invalid/);
  input.selection.target = 'javascript';
  input.selection.args = [{ type: 'bool', value: 'true' }];
  await assert.rejects(runProject(input, call), /Invalid/);
  input.selection.args = [];
  input.selection.profile = 'control-flow-v1';
  input.selection.resultType = 'bool';
  await assert.rejects(runProject(input, call), /Invalid/);
  input.selection.profile = 'i32-v1';
  input.selection.resultType = 'i32';
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
    selection: { profile: 'i32-v1', name: 'main', target: 'javascript', args: [], resultType: 'i32' }, token }, async (_, args) => {
    calls.push(args[0]);
    token.isCancellationRequested = true;
  }), /cancelled/);
  assert.deepEqual(calls, ['new']);
  assert.deepEqual(await fs.readdir(storage), []);
});
