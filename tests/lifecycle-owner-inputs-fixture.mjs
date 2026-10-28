// Test-only shared inert declarations. No test registration, processes, images or execution.
import assert from 'node:assert/strict';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { createLifecycleOwnerPlan, lifecycleMaterialPins } from '../scripts/stability-gates/lifecycle-owner-inputs.mjs';
const packageName = 'zryna-playground-compiler';
export const materialNode = '/materials/runtime/node/bin/node', nodeLibrary = '/usr/lib/libnode-fixture.so';
export const raw = value => Buffer.from(JSON.stringify(value) + '\n');
export const selected = (path, bytes = 256) => ({ path, bytes, sha256: sha256(Buffer.from('inert declared ' + path)) });
export function fixture() {
  const root = '/source', targetDirectory = '/workspace/private/target';
  const tools = { cargo: selected('/tools/cargo'), rustc: selected('/tools/rustc') };
  const artifacts = { compiler: selected(targetDirectory + '/release/' + packageName),
    test: selected(targetDirectory + '/release/deps/lifecycle-0123456789abcdef') };
  const source = { commit: 'a'.repeat(40), tree: 'b'.repeat(40) };
  const cargoInputs = { manifest: selected('/source/Cargo.toml'), lock: selected('/source/Cargo.lock') };
  const environment = { PATH: '/tools:/usr/bin', HOME: '/home/operator', CARGO_HOME: '/cache/cargo',
    RUSTUP_HOME: '/cache/rustup', LANG: 'C.UTF-8', LC_ALL: 'C.UTF-8', RUSTC: '/tools/rustc',
    RUSTUP_TOOLCHAIN: '1.97.1', CARGO_TARGET_DIR: targetDirectory, CARGO_NET_OFFLINE: 'true',
    CARGO_BUILD_JOBS: '2', CARGO_INCREMENTAL: '0', CARGO_TERM_COLOR: 'never' };
  const locations = ['/lifecycle/supervisor', '/lifecycle/test', '/usr/bin/python3', artifacts.compiler.path,
    '/lib64/ld-linux-x86-64.so.2', nodeLibrary, '/usr/lib/python3.12/encodings/__init__.py'];
  const files = locations.map(path => ({ ...selected(path), mode: path.endsWith('.py') ? 0o444 : 0o555,
    kind: 'regular', nlink: 1, uid: 0, gid: 0 }));
  Object.assign(files.find(file => file.path === '/lifecycle/test'), { bytes: artifacts.test.bytes, sha256: artifacts.test.sha256 });
  const prefix = Buffer.alloc(64); Buffer.from([127, 69, 76, 70, 2, 1]).copy(prefix); prefix.writeUInt16LE(62, 18);
  const runtime = { files: files.sort((a, b) => a.path.localeCompare(b.path, 'en')),
    elf: [...locations.filter(path => !path.endsWith('.py')).map(path => ({ path, prefixHex: prefix.toString('hex'),
      interpreter: path.startsWith('/lib64/') || path === nodeLibrary ? null : '/lib64/ld-linux-x86-64.so.2', needed: [] })),
    { path: materialNode, prefixHex: prefix.toString('hex'), interpreter: '/lib64/ld-linux-x86-64.so.2',
      needed: [{ name: 'libnode-fixture.so', path: nodeLibrary }] }]
      .sort((a, b) => a.path.localeCompare(b.path, 'en')),
    python: { entry: '/usr/bin/python3', imports: ['os', 'resource', 'sys'],
      files: ['/usr/bin/python3', '/usr/lib/python3.12/encodings/__init__.py'] }, support: [] };
  const expected = { source, root, targetDirectory, cargoInputs, tools, artifacts, environment,
    runtimeSha256: sha256(Buffer.from(canonical(runtime))), cpus: [2, 10] };
  const directory = root + '/apps/' + packageName, id = 'path+file://' + directory + '#0.1.0';
  const rows = [['test', 'lifecycle', 'test', '/tests/lifecycle.rs'], ['compiler', packageName, 'bin', '/src/main.rs']]
    .map(([key, name, kind, file]) => ({ reason: 'compiler-artifact', package_id: id,
      target: { name, kind: [kind], crate_types: ['bin'], src_path: directory + file },
      profile: { test: key === 'test', opt_level: '3' }, executable: artifacts[key].path, filenames: [artifacts[key].path] }));
  rows.push({ reason: 'build-finished', success: true });
  const pair = value => ({ before: structuredClone(value), after: structuredClone(value) });
  const capture = { source: pair(source), inputs: pair(cargoInputs), tools: pair(tools),
    configuration: pair(['/cache/cargo/config', '/cache/cargo/config.toml', '/source/.cargo/config',
      '/source/.cargo/config.toml', '/.cargo/config', '/.cargo/config.toml'].map(path => ({ path, present: false }))),
    versions: { cargo: Buffer.from('cargo 1.97.1 (c980f4866 2026-06-30)\n'),
      rustc: Buffer.from('rustc 1.97.1 (8bab26f4f 2026-07-14)\nhost: x86_64-unknown-linux-gnu\n') },
    command: { executable: tools.cargo.path, args: ['test', '--locked', '--offline', '--release', '-p', packageName,
      '--test', 'lifecycle', '--no-run', '--message-format=json', '-vv'], cwd: root, environment: structuredClone(environment) },
    termination: { exitCode: 0, signal: null, error: null }, metadata: raw({ packages: [{ name: packageName,
      version: '0.1.0', source: null, manifest_path: directory + '/Cargo.toml', id }] }),
    stdout: Buffer.concat(rows.map(raw)),
    stderr: Buffer.from(`     Running \`CARGO_BIN_EXE_${packageName}=${artifacts.compiler.path} /tools/rustc --crate-name lifecycle --test ${directory}/tests/lifecycle.rs\`\n`),
    artifacts: structuredClone(artifacts) };
  return { expected, capture, materials: lifecycleMaterialPins(), runtime };
}
export function reject(change, selectRuntimeAgain = false) {
  const value = fixture(); change(value);
  if (selectRuntimeAgain) value.expected.runtimeSha256 = sha256(Buffer.from(canonical(value.runtime)));
  assert.throws(() => createLifecycleOwnerPlan(value));
}
export function cargoRows(value, change) {
  const rows = value.capture.stdout.toString().trimEnd().split('\n').map(JSON.parse);
  change(rows); value.capture.stdout = Buffer.concat(rows.map(raw));
}
export function rebaseTarget(value, target) {
  const previous = value.expected.targetDirectory;
  function walk(item) {
    if (Buffer.isBuffer(item)) return Buffer.from(item.toString().replaceAll(previous, target));
    if (typeof item === 'string') return item.replaceAll(previous, target);
    if (Array.isArray(item)) return item.map(walk);
    if (item && typeof item === 'object') return Object.fromEntries(Object.entries(item).map(([key, child]) => [key, walk(child)]));
    return item;
  }
  Object.assign(value, walk(value));
  for (const rows of [value.runtime.files, value.runtime.elf]) rows.sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  value.expected.runtimeSha256 = sha256(Buffer.from(canonical(value.runtime)));
}
