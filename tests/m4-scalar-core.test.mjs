import assert from 'node:assert/strict';
import { existsSync, readFileSync, realpathSync, rmSync } from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const workspaceRoot = path.resolve(fileURLToPath(new URL('..', import.meta.url)));
const entrypoint = 'tests/m4-fixtures/scalar-core/main.zry';
const outputRoot = path.join(workspaceRoot, '.zryna', 'out');
const cargo = process.platform === 'win32' ? 'cargo.exe' : 'cargo';
const nodeName = process.platform === 'win32' ? 'node.exe' : 'node';

const oracle = Object.freeze([
  ['add_i32', ['i32:20', 'i32:22'], 42],
  ['add_i32', ['i32:2147483647', 'i32:1'], -2147483648],
  ['add_i32', ['i32:-2147483648', 'i32:-1'], 2147483647],
  ['min_i32', ['i32:7', 'i32:-3'], -3],
  ['min_i32', ['i32:-2147483648', 'i32:2147483647'], -2147483648],
  ['min_i32', ['i32:5', 'i32:5'], 5],
  ['select_i32', ['bool:true', 'i32:17', 'i32:-9'], 17],
  ['select_i32', ['bool:false', 'i32:17', 'i32:-9'], -9],
  ['score', [], 42],
]);

const ambientCases = Object.freeze(['environment', 'filesystem', 'network', 'clock', 'randomness']);
const ambientDiagnostics = Object.freeze({
  environment: ['ZRYNA-F1103'],
  filesystem: ['ZRYNA-F1103'],
  network: ['ZRYNA-M2009', 'ZRYNA-M2012'],
  clock: ['ZRYNA-F1103'],
  randomness: ['ZRYNA-F1103'],
});
const supportsNative = process.platform === 'linux' && process.arch === 'x64' &&
  Boolean(process.report?.getReport().header.glibcVersionRuntime);
let nextStem = 0;

function compileTestBinary() {
  const build = spawnSync(
    cargo,
    ['build', '--locked', '-p', 'zryna', '--bin', 'zryna', '--message-format=json'],
    { cwd: workspaceRoot, encoding: 'utf8', windowsHide: true },
  );
  assert.equal(build.status, 0, `zryna test binary failed to build\n${build.stderr}`);
  const executable = build.stdout
    .split(/\r?\n/u)
    .filter(Boolean)
    .map(line => JSON.parse(line))
    .find(message => message.reason === 'compiler-artifact' &&
      message.target?.name === 'zryna' && message.executable)?.executable;
  assert(executable, 'cargo did not report the zryna test binary');
  return executable;
}

const zryna = compileTestBinary();
const pinnedNode = findPinnedNode();

function findPinnedNode() {
  const candidates = [
    process.env.ZRYNA_TEST_NODE,
    process.env.NODE,
    process.execPath,
    ...(process.env.PATH ?? '').split(path.delimiter).map(directory => path.join(directory, nodeName)),
  ].filter(Boolean);
  for (const candidate of candidates) {
    if (!existsSync(candidate)) continue;
    const probe = spawnSync(candidate, ['--version'], { encoding: 'utf8', windowsHide: true });
    if (probe.status === 0 && probe.stderr === '' && probe.stdout.trim() === 'v22.22.1') {
      return realpathSync(candidate);
    }
  }
  assert.fail('Node.js 22.22.1 must be available through ZRYNA_TEST_NODE, NODE or PATH');
}

function uniqueStem(label) {
  nextStem += 1;
  return `m4_scalar_core_${process.pid}_${nextStem}_${label}`;
}

function run(arguments_) {
  return spawnSync(zryna, arguments_, {
    cwd: workspaceRoot,
    encoding: 'utf8',
    windowsHide: true,
  });
}

function commandArguments(
  command,
  source,
  target,
  stem,
  extra = [],
  profile = 'control-flow-v1',
) {
  return [
    command,
    source,
    '--profile', profile,
    '--target', target,
    '--name', stem,
    '--json',
    '--root', workspaceRoot,
    '--node', pinnedNode,
    ...extra,
  ];
}

function bundlePath(stem, command) {
  return path.join(outputRoot, `${stem}.${command}`);
}

function removeBundle(stem, command) {
  rmSync(bundlePath(stem, command), { recursive: true, force: true });
}

function response(output) {
  assert.equal(output.stderr, '', `unexpected stderr\n${output.stderr}`);
  assert(output.stdout, 'command must return a JSON response');
  return JSON.parse(output.stdout);
}

function assertImportFreeArtifact(target, stem, command = 'run') {
  const bundle = bundlePath(stem, command);
  if (target === 'javascript') {
    const source = readFileSync(path.join(bundle, 'javascript', `${stem}.mjs`), 'utf8');
    assert.doesNotMatch(source, /(^|\n)\s*import(?:\s|\()/u);
    assert.doesNotMatch(
      source,
      /\b(?:process|fetch|XMLHttpRequest|WebSocket|Date|crypto)\b|Math\.random/u,
    );
    return;
  }
  const bytes = readFileSync(path.join(bundle, 'webassembly', `${stem}.wasm`));
  const module = new WebAssembly.Module(bytes);
  assert.deepEqual(WebAssembly.Module.imports(module), []);
}

test('S1 scalar core matches Q1-Q3 and score on supported targets', () => {
  const targets = supportsNative
    ? ['javascript', 'webassembly', 'native']
    : ['javascript', 'webassembly'];
  for (const target of targets) {
    for (const [exportName, arguments_, expected] of oracle) {
      const stem = uniqueStem(`${target}_${exportName}`);
      try {
        const output = run(commandArguments('run', entrypoint, target, stem, [
          '--export', exportName,
          ...arguments_.flatMap(argument => ['--arg', argument]),
        ]));
        assert.equal(output.status, 0, `${target}/${exportName} failed\n${output.stdout}`);
        const result = response(output);
        assert.deepEqual(result.results, [{
          target,
          outcome: { kind: 'returned', value: { type: 'i32', value: expected } },
        }]);
        if (target !== 'native') assertImportFreeArtifact(target, stem);
      } finally {
        removeBundle(stem, 'run');
      }
    }
  }
});

test('the same source closure verifies independently through DataOwnershipV1', () => {
  for (const target of ['javascript', 'webassembly']) {
    const stem = uniqueStem(`data_ownership_${target}`);
    try {
      const output = run(commandArguments(
        'build',
        entrypoint,
        target,
        stem,
        [],
        'data-ownership-v1',
      ));
      assert.equal(output.status, 0, `${target} DataOwnershipV1 build failed\n${output.stdout}`);
      const result = response(output);
      assert.equal(result.ok, true);
      assert.deepEqual(result.results, []);
      assert.match(result.manifest, /zryna-manifest-v3\.json$/u);
      assertImportFreeArtifact(target, stem, 'build');
    } finally {
      removeBundle(stem, 'build');
    }
  }
});

test('supported Linux native object passes the closed backend import audit', {
  skip: !supportsNative,
}, () => {
  const stem = uniqueStem('native_object_audit');
  try {
    const output = run(commandArguments('build', entrypoint, 'native', stem));
    assert.equal(output.status, 0, `native object build failed\n${output.stdout}`);
    const result = response(output);
    const manifest = JSON.parse(readFileSync(path.join(
      bundlePath(stem, 'build'),
      'zryna-manifest-v2.json',
    ), 'utf8'));
    assert.equal(result.ok, true);
    assert.deepEqual(result.results, []);
    assert.deepEqual(manifest.artifacts.map(({ target, kind }) => ({ target, kind })), [{
      target: 'native',
      kind: 'linux-x86-64-relocatable-object',
    }]);
  } finally {
    removeBundle(stem, 'build');
  }
});

test('ambient environment, filesystem, network, clock and randomness reject before emission', () => {
  for (const name of ambientCases) {
    const stem = uniqueStem(`invalid_${name}`);
    const source = `tests/m4-fixtures/scalar-core/invalid/${name}/main.zry`;
    try {
      const output = run(commandArguments('build', source, 'all', stem));
      assert.equal(output.status, 3, `${name} must be rejected as source\n${output.stdout}`);
      const result = response(output);
      assert.equal(result.ok, false);
      assert.equal(result.manifest, null);
      assert.deepEqual(result.results, []);
      assert.deepEqual(result.diagnostics.map(diagnostic => diagnostic.code), ambientDiagnostics[name]);
      assert.equal(readFileSync(path.join(workspaceRoot, source), 'utf8').length > 0, true);
      assert.equal(requireBundleAbsent(stem, 'build'), true);
    } finally {
      removeBundle(stem, 'build');
    }
  }
});

function requireBundleAbsent(stem, command) {
  try {
    readFileSync(path.join(bundlePath(stem, command), 'zryna-manifest-v2.json'));
    return false;
  } catch (error) {
    if (error?.code === 'ENOENT') return true;
    throw error;
  }
}

test('Windows native selection retains ZRYNA-N4002 and publishes no bundle', {
  skip: process.platform !== 'win32',
}, () => {
  for (const target of ['native', 'all']) {
    const stem = uniqueStem(`unsupported_${target}`);
    try {
      const output = run(commandArguments('run', entrypoint, target, stem, [
        '--export', 'score',
      ]));
      assert.equal(output.status, 4);
      const result = response(output);
      assert.equal(result.ok, false);
      assert.equal(result.manifest, null);
      assert.deepEqual(result.results, []);
      assert.equal(result.diagnostics[0].code, 'ZRYNA-N4002');
      assert.equal(requireBundleAbsent(stem, 'run'), true);
    } finally {
      removeBundle(stem, 'run');
    }
  }
});
