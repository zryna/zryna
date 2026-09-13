import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, rmSync, symlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import test from 'node:test';
import { sha256 } from '../scripts/distribution-release/canonical.mjs';
import {
  extractVerifiedProductionFiles, runInstalledAcceptance,
} from '../scripts/distribution-release/run-installed-acceptance.mjs';

function file(path, mode = 0o644, value = path) {
  return { path, mode, data: Buffer.from(value) };
}

function fixture(t, target = 'x86_64-pc-windows-msvc') {
  const root = mkdtempSync(join(tmpdir(), 'zryna-installed-acceptance-test-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const archive = Buffer.from('authenticated production archive');
  const windows = target === 'x86_64-pc-windows-msvc';
  const descriptor = {
    version: '0.2.0', filename: `zryna-0.2.0-${target}.${windows ? 'zip' : 'tar.gz'}`,
    size: archive.length, sha256: sha256(archive),
    source: { repository: 'https://github.com/zryna/zryna', ref: 'refs/tags/v0.2.0',
      commit: 'a'.repeat(40), tree: 'b'.repeat(40), sourceDateEpoch: 1_789_081_200 },
    target: { triple: target, archiveFormat: windows ? 'zip' : 'tar-gzip', platformBaseline: {} },
    recipe: { format: 'zryna.distribution-recipe.v1', sha256: 'c'.repeat(64) },
  };
  const files = [
    file('VERSION', 0o644, '0.2.0\n'),
    file(windows ? 'bin/zryna.exe' : 'bin/zryna', 0o755, 'compiled cli'),
    file('lib/zryna/bootstrap/worker.mjs', 0o644, 'provider'),
    file('metadata/distribution.json', 0o644, '{}\n'),
    file(windows ? 'runtime/node/node.exe' : 'runtime/node/bin/node', 0o755, 'runtime'),
  ];
  const distribution = { files: [
    { path: 'lib/zryna/bootstrap/worker.mjs', role: 'provider' },
    { path: windows ? 'runtime/node/node.exe' : 'runtime/node/bin/node', role: 'runtime' },
  ] };
  return { root: resolve(root), archive, descriptor, files, distribution };
}

test('accepts one authenticated production archive through relocation, portable targets, and tamper rejection', async (t) => {
  const f = fixture(t);
  const calls = [];
  const spawn = (executable, args, options) => {
    calls.push({ executable, args, cwd: options.cwd, shell: options.shell });
    const tampered = executable.includes('tampered-');
    const stdout = args[0] === '--version' ? 'zryna 0.2.0\n'
      : args[0] === 'run' ? '42\n' : '';
    return { status: tampered ? 2 : 0, signal: null,
      stdout: Buffer.from(tampered ? '' : stdout),
      stderr: Buffer.from(tampered ? 'error[ZRYNA-C4220]: changed installation\n' : '') };
  };
  const result = await runInstalledAcceptance({
    archive: f.archive, descriptor: f.descriptor, workRoot: f.root, spawn,
    verifyArchiveImpl: async () => ({
      archiveSha256: f.descriptor.sha256, files: f.files, distribution: f.distribution,
    }),
  });
  assert.deepEqual(result.checks, {
    relocated: true, version: true, project: true,
    javascript: true, webassembly: true, tamperSubjects: 3,
  });
  assert.equal(calls.length, 9);
  assert(calls.every(({ shell }) => shell === false));
  assert(calls.every(({ executable }) => executable.includes('relocated')
    || executable.includes('tampered-')));
  assert.deepEqual(calls.slice(0, 6).map(({ args }) => [args[0], args.includes('javascript'),
    args.includes('webassembly')]), [
    ['--version', false, false], ['new', false, false],
    ['build', true, false], ['run', true, false],
    ['build', false, true], ['run', false, true],
  ]);
  assert.deepEqual(calls.slice(6).map(({ args }) => args.at(-1)),
    ['tamper-0', 'tamper-1', 'tamper-2']);
});

test('uses the production Linux archive layout and relocated executable', async (t) => {
  const f = fixture(t, 'x86_64-unknown-linux-gnu');
  const executables = [];
  const result = await runInstalledAcceptance({
    archive: f.archive, descriptor: f.descriptor, workRoot: f.root,
    verifyArchiveImpl: async () => ({ archiveSha256: f.descriptor.sha256,
      files: f.files, distribution: f.distribution }),
    spawn: (executable, args) => {
      executables.push(executable);
      const tampered = executable.includes('tampered-');
      return { status: tampered ? 2 : 0, signal: null,
        stdout: Buffer.from(args[0] === '--version' ? 'zryna 0.2.0\n'
          : args[0] === 'run' ? '42\n' : ''),
        stderr: Buffer.from(tampered ? 'error[ZRYNA-C4220]: changed installation\n' : '') };
    },
  });
  assert.equal(result.target, 'x86_64-unknown-linux-gnu');
  assert(executables.every((path) => path.endsWith(join('bin', 'zryna'))));
});

test('rejects qualification names and paths before installed execution', async (t) => {
  const f = fixture(t);
  let spawned = false;
  await assert.rejects(() => runInstalledAcceptance({
    archive: f.archive,
    descriptor: { ...f.descriptor, filename: 'zryna-qualification-0.2.0.zip' },
    workRoot: f.root, spawn: () => { spawned = true; },
    verifyArchiveImpl: async () => ({ archiveSha256: f.descriptor.sha256,
      files: f.files, distribution: f.distribution }),
  }), /authenticated production archive descriptor is invalid/);
  const files = [...f.files, file('qualification/input.json')];
  await assert.rejects(() => runInstalledAcceptance({
    archive: f.archive, descriptor: f.descriptor, workRoot: f.root,
    spawn: () => { spawned = true; },
    verifyArchiveImpl: async () => ({ archiveSha256: f.descriptor.sha256,
      files, distribution: f.distribution }),
  }), /production archive verification did not close/);
  assert.equal(spawned, false);
});

test('extraction rejects traversal, case collisions, and linked roots', (t) => {
  const f = fixture(t);
  for (const [name, files, pattern] of [
    ['traversal', [file('../outside')], /unsafe/],
    ['collision', [file('lib/Worker.mjs'), file('LIB/worker.mjs')], /collide/],
  ]) assert.throws(() => extractVerifiedProductionFiles(join(f.root, name), files), pattern);

  const outside = join(f.root, 'outside');
  const linked = join(f.root, 'linked');
  mkdirSync(outside);
  try {
    symlinkSync(outside, linked, 'junction');
  } catch {
    return;
  }
  assert.throws(() => extractVerifiedProductionFiles(linked, [file('VERSION')]), /EEXIST/);
});

test('fails closed when a tampered installation command succeeds', async (t) => {
  const f = fixture(t);
  await assert.rejects(() => runInstalledAcceptance({
    archive: f.archive, descriptor: f.descriptor, workRoot: f.root,
    verifyArchiveImpl: async () => ({ archiveSha256: f.descriptor.sha256,
      files: f.files, distribution: f.distribution }),
    spawn: (_executable, args) => ({ status: 0, signal: null,
      stdout: Buffer.from(args[0] === '--version' ? 'zryna 0.2.0\n'
        : args[0] === 'run' ? '42\n' : ''), stderr: Buffer.alloc(0) }),
  }), /tampered installation reached artifact execution/);
});
