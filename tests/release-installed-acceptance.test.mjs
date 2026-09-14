import assert from 'node:assert/strict';
import {
  mkdtempSync, mkdirSync, readFileSync, rmSync, symlinkSync, writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import test from 'node:test';
import {
  canonicalBounded, parseCanonical, sha256,
} from '../scripts/distribution-release/canonical.mjs';
import {
  acceptReproducedRelease, admitReleaseTransition, extractVerifiedProductionFiles,
  removeVerifiedProductionFiles, runInstalledAcceptance,
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
    version: '0.2.1', filename: `zryna-0.2.1-${target}.${windows ? 'zip' : 'tar.gz'}`,
    size: archive.length, sha256: sha256(archive),
    source: { repository: 'https://github.com/zryna/zryna', ref: 'refs/tags/v0.2.1',
      commit: 'a'.repeat(40), tree: 'b'.repeat(40), sourceDateEpoch: 1_789_081_200 },
    target: { triple: target, archiveFormat: windows ? 'zip' : 'tar-gzip', platformBaseline: {} },
    recipe: { format: 'zryna.distribution-recipe.v1', sha256: 'c'.repeat(64) },
  };
  const files = [
    file('VERSION', 0o644, '0.2.1\n'),
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

function mockInstalledCommands({ target = 'x86_64-pc-windows-msvc', calls, tamperSucceeds = false } = {}) {
  return (executable, args, options) => {
    calls?.push({ executable, args, cwd: options.cwd, shell: options.shell, env: options.env });
    const tampered = executable.includes('tampered-');
    const operation = args[0];
    const targetName = args.includes('--target') ? args[args.indexOf('--target') + 1] : undefined;
    const project = join(options.cwd, 'hello');
    if (operation === 'new') {
      mkdirSync(join(project, 'src'), { recursive: true });
      writeFileSync(join(project, 'zryna.package.json'), '{}\n');
      writeFileSync(join(project, 'zryna.lock.json'), '{}\n');
      writeFileSync(join(project, 'src', 'main.zry'), 'export function main(): i32 { return 42; }\n');
    } else if (['build', 'run'].includes(operation) && !tampered
      && !(target.endsWith('windows-msvc') && targetName === 'native')) {
      const name = args[args.indexOf('--name') + 1];
      const bundle = join(project, '.zryna', 'out', `${name}.${operation}`);
      mkdirSync(bundle, { recursive: true });
      writeFileSync(join(bundle, 'zryna-manifest-v1.json'), '{}\n');
    }
    const nativeRejected = target.endsWith('windows-msvc') && targetName === 'native';
    const rejectedTamper = tampered && !tamperSucceeds;
    const stdout = operation === '--version' ? 'zryna 0.2.1\n'
      : operation === 'run' ? `${targetName}: i32 42\n` : '';
    return { status: rejectedTamper || nativeRejected ? 2 : 0, signal: null,
      stdout: Buffer.from(rejectedTamper || nativeRejected ? '' : stdout),
      stderr: Buffer.from(rejectedTamper ? 'error[ZRYNA-C4220]: changed installation\n'
        : nativeRejected ? 'error[ZRYNA-N4002]: native unavailable\n' : '') };
  };
}

test('accepts one authenticated production archive through relocation, portable targets, and tamper rejection', async (t) => {
  const f = fixture(t);
  const calls = [];
  const spawn = mockInstalledCommands({ calls });
  const result = await runInstalledAcceptance({
    archive: f.archive, descriptor: f.descriptor, workRoot: f.root, spawn,
    verifyArchiveImpl: async () => ({
      archiveSha256: f.descriptor.sha256, files: f.files, distribution: f.distribution,
    }),
  });
  assert.deepEqual(result.checks, {
    collision: true, relocated: true, path: true, version: true, locale: 'tr_TR.UTF-8',
    project: true, javascript: true, webassembly: true, native: 'unsupported',
    tamperSubjects: 3, uninstall: true, userFilesRetained: true,
  });
  assert.equal(calls.length, 11);
  assert(calls.every(({ shell }) => shell === false));
  assert(calls.slice(0, 2).every(({ executable }) => executable === 'zryna.exe'));
  assert(calls.slice(2).every(({ executable }) => executable.includes('relocated')
    || executable.includes('tampered-')));
  assert.deepEqual(calls.slice(0, 7).map(({ args }) => [args[0], args.includes('javascript'),
    args.includes('webassembly')]), [
    ['--version', false, false], ['--version', false, false], ['new', false, false],
    ['build', true, false], ['run', true, false],
    ['build', false, true], ['run', false, true],
  ]);
  assert.equal(calls[7].args[calls[7].args.indexOf('--name') + 1], 'native-rejection');
  assert.deepEqual(calls.slice(8).map(({ args }) => args.at(-1)),
    ['tamper-0', 'tamper-1', 'tamper-2']);
  assert(calls.every(({ env }) => !Object.keys(env).some((name) => name.startsWith('ZRYNA_'))));
});

test('uses the production Linux archive layout and relocated executable', async (t) => {
  const f = fixture(t, 'x86_64-unknown-linux-gnu');
  const executables = [];
  const result = await runInstalledAcceptance({
    archive: f.archive, descriptor: f.descriptor, workRoot: f.root,
    verifyArchiveImpl: async () => ({ archiveSha256: f.descriptor.sha256,
      files: f.files, distribution: f.distribution }),
    spawn: mockInstalledCommands({ target: f.descriptor.target.triple,
      calls: { push: ({ executable }) => executables.push(executable) } }),
  });
  assert.equal(result.target, 'x86_64-unknown-linux-gnu');
  assert.equal(result.checks.native, 'built-and-ran');
  assert(executables.slice(0, 2).every((path) => path === 'zryna'));
  assert(executables.slice(2).every((path) => path.endsWith(join('bin', 'zryna'))));
});

test('accepts the exact reproduced directory and emits one canonical receipt', async (t) => {
  const f = fixture(t, 'x86_64-unknown-linux-gnu');
  const inputRoot = join(f.root, 'reproduced');
  const outputRoot = join(f.root, 'acceptance');
  const workRoot = join(f.root, 'work');
  mkdirSync(inputRoot);
  mkdirSync(workRoot);
  const artifacts = {
    archive: { path: f.descriptor.filename, size: f.archive.length, sha256: sha256(f.archive) },
    buildReceipt: { path: 'zryna-0.2.1-x86_64-unknown-linux-gnu.build-receipt.json',
      size: 2, sha256: sha256('{}') },
    sbom: { path: 'zryna-0.2.1-x86_64-unknown-linux-gnu.spdx.json',
      size: 2, sha256: sha256('{}') },
  };
  const reproduction = {
    format: 'zryna.release-reproduction.v1', version: '0.2.1',
    target: 'x86_64-unknown-linux-gnu',
    source: { ...f.descriptor.source, tagObject: 'd'.repeat(40) },
    recipe: f.descriptor.recipe, artifacts,
    builds: [
      { replica: 1, manifestSha256: 'e'.repeat(64) },
      { replica: 2, manifestSha256: 'f'.repeat(64) },
    ],
    comparison: 'byte-identical',
  };
  writeFileSync(join(inputRoot, 'reproduction.json'), `${canonicalBounded(reproduction)}\n`);
  writeFileSync(join(inputRoot, artifacts.archive.path), f.archive);
  writeFileSync(join(inputRoot, artifacts.buildReceipt.path), '{}');
  writeFileSync(join(inputRoot, artifacts.sbom.path), '{}');
  const spawn = mockInstalledCommands({ target: f.descriptor.target.triple });
  let expected;
  const result = await acceptReproducedRelease({
    inputRoot, outputRoot, workRoot, target: reproduction.target, spawn,
    verifyArchiveImpl: async (_archive, descriptor) => {
      expected = descriptor;
      return { archiveSha256: f.descriptor.sha256, files: f.files, distribution: f.distribution };
    },
  });
  assert.equal(expected.source.tagObject, undefined);
  assert.equal(expected.target.triple, f.descriptor.target.triple);
  assert.equal(expected.target.archiveFormat, 'tar-gzip');
  assert.deepEqual(expected.target.platformBaseline,
    { os: 'linux', distribution: 'ubuntu', version: '24.04', architecture: 'x86_64' });
  assert.deepEqual(parseCanonical(readFileSync(join(outputRoot, 'installed-acceptance.json'), 'utf8')),
    result);
});

test('rejects qualification names and paths before installed execution', async (t) => {
  const f = fixture(t);
  let spawned = false;
  await assert.rejects(() => runInstalledAcceptance({
    archive: f.archive,
    descriptor: { ...f.descriptor, filename: 'zryna-qualification-0.2.1.zip' },
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

test('owned removal retains foreign files and rejects changed owned bytes before mutation', (t) => {
  const f = fixture(t);
  const installed = extractVerifiedProductionFiles(join(f.root, 'installed'), f.files);
  writeFileSync(join(installed, 'foreign.txt'), 'retain\n');
  removeVerifiedProductionFiles(installed, f.files);
  assert.equal(readFileSync(join(installed, 'foreign.txt'), 'utf8'), 'retain\n');
  assert(!f.files.some(({ path }) => {
    try { readFileSync(join(installed, ...path.split('/'))); return true; } catch { return false; }
  }));

  const changed = extractVerifiedProductionFiles(join(f.root, 'changed'), f.files);
  writeFileSync(join(changed, 'VERSION'), 'changed\n');
  assert.throws(() => removeVerifiedProductionFiles(changed, f.files), /changed before removal/);
  assert.equal(readFileSync(join(changed, 'bin', 'zryna.exe'), 'utf8'), 'compiled cli');
});

test('fixture-only release transition admission accepts upgrades and rejects downgrade or reinstall', () => {
  assert.deepEqual(admitReleaseTransition('0.2.1', '0.2.2'),
    { current: '0.2.1', candidate: '0.2.2', operation: 'upgrade' });
  assert.throws(() => admitReleaseTransition('0.2.1', '0.2.0'), /downgrade is forbidden/);
  assert.throws(() => admitReleaseTransition('0.2.1', '0.2.1'), /not an upgrade/);
  assert.throws(() => admitReleaseTransition('0.2.1', '0.02.2'), /version is invalid/);
});

test('fails closed when a tampered installation command succeeds', async (t) => {
  const f = fixture(t);
  await assert.rejects(() => runInstalledAcceptance({
    archive: f.archive, descriptor: f.descriptor, workRoot: f.root,
    verifyArchiveImpl: async () => ({ archiveSha256: f.descriptor.sha256,
      files: f.files, distribution: f.distribution }),
    spawn: mockInstalledCommands({ tamperSucceeds: true }),
  }), /tampered installation reached artifact execution/);
});
