// Inert declared records only: no Cargo, Python, image, process or native execution.
import assert from 'node:assert/strict';
import test from 'node:test';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { createLifecycleOwnerPlan, lifecycleMaterialPins } from '../scripts/stability-gates/lifecycle-owner-inputs.mjs';
const packageName = 'zryna-playground-compiler';
const raw = value => Buffer.from(JSON.stringify(value) + '\n');
const selected = (path, bytes = 256) => ({ path, bytes, sha256: sha256(Buffer.from('inert declared ' + path)) });
function fixture() {
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
    '/lib64/ld-linux-x86-64.so.2', '/usr/lib/python3.12/encodings/__init__.py'];
  const files = locations.map(path => ({ ...selected(path), mode: path.endsWith('.py') ? 0o444 : 0o555,
    kind: 'regular', nlink: 1, uid: 0, gid: 0 }));
  Object.assign(files.find(file => file.path === '/lifecycle/test'), { bytes: artifacts.test.bytes, sha256: artifacts.test.sha256 });
  const prefix = Buffer.alloc(64); Buffer.from([127, 69, 76, 70, 2, 1]).copy(prefix); prefix.writeUInt16LE(62, 18);
  const runtime = { files: files.sort((a, b) => a.path.localeCompare(b.path, 'en')),
    elf: locations.filter(path => !path.endsWith('.py')).map(path => ({ path, prefixHex: prefix.toString('hex'),
      interpreter: path.startsWith('/lib64/') ? null : '/lib64/ld-linux-x86-64.so.2', needed: [] }))
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
function reject(change, selectRuntimeAgain = false) {
  const value = fixture(); change(value);
  if (selectRuntimeAgain) value.expected.runtimeSha256 = sha256(Buffer.from(canonical(value.runtime)));
  assert.throws(() => createLifecycleOwnerPlan(value));
}
function cargoRows(value, change) {
  const rows = value.capture.stdout.toString().trimEnd().split('\n').map(JSON.parse);
  change(rows); value.capture.stdout = Buffer.concat(rows.map(raw));
}
function rebaseTarget(value, target) {
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
test('complete inert capture yields only the two fixed unapproved Linux modes and original limits', () => {
  const value = fixture(), plan = createLifecycleOwnerPlan(value);
  assert.equal(plan.materials.files.length, 10); assert.equal(plan.materials.bytes, 133968848);
  assert.equal(plan.parent.memoryBytes, 536870912); assert.equal(plan.parent.swapBytes, 0);
  assert.equal(plan.parent.tasks, 32); assert.deepEqual(plan.parent.cpus, [2, 10]);
  assert.deepEqual(plan.parent.inheritedLimits, { core: 0, fsize: 268435456, nofile: 512 });
  assert.equal(plan.parent.executionMs, 10000); assert.equal(plan.parent.cleanupMs, 2000);
  assert.deepEqual(plan.modes.map(mode => mode.executable), ['/lifecycle/test', '/lifecycle/test']);
  assert.deepEqual(plan.modes[0].counts, { passed: 2, failed: 0, ignored: 1, measured: 0, filtered: 0 });
  assert.deepEqual(plan.modes[0].requiredPassed, ['inherited_one_mib_hard_limit_rejects_before_material_creation',
    'malformed_request_stops_before_material_capture_on_each_host']);
  assert.deepEqual(plan.modes[1].counts, { passed: 1, failed: 0, ignored: 0, measured: 0, filtered: 2 });
  for (const key of ['sourceAuthenticated', 'ownerExecuted', 'materialsApproved', 'runtimeApproved',
    'hostApproved', 'executionApproved', 'proofQualified', 'performanceQualified']) assert.equal(plan[key], false);
  assert.equal(plan.baselineAuthority, null); assert.equal(plan.thresholds, null);
  plan.ownerExecuted = true; assert.throws(() => createLifecycleOwnerPlan(plan), 'decoded plan is not execution authority input');
  const pins = lifecycleMaterialPins(); pins[0].sha256 = '0'.repeat(64);
  assert.notEqual(lifecycleMaterialPins()[0].sha256, pins[0].sha256, 'no mutable imported material authority');
  const relative = fixture(); relative.capture.stderr = Buffer.from(relative.capture.stderr.toString().replace('/source/apps/', 'apps/'));
  assert.doesNotThrow(() => createLifecycleOwnerPlan(relative));
  const moved = fixture(); rebaseTarget(moved, '/workspace/other/target'); assert.doesNotThrow(() => createLifecycleOwnerPlan(moved));
});
test('all original material names/bytes/hashes and legacy v4 inventory stay closed', () => {
  for (const change of [v => v.materials.pop(), v => v.materials.push({ ...v.materials[0] }),
    v => { v.materials[0].path += '.extra'; }, v => { v.materials[0].bytes++; },
    v => { v.materials[0].sha256 = '0'.repeat(64); }, v => { v.materials[0].mode = 0o555; },
    v => { v.materials[0].path = v.materials[0].path.toUpperCase(); },
    v => { v.materials[7].path = '/materials/lib/zryna/bootstrap/v4/limits.mjs'; }]) reject(change);
});
test('independent source/tools/Cargo/profile/argv and raw metadata cannot drift together silently', () => {
  for (const change of [v => { v.capture.source.after.commit = 'c'.repeat(40); },
    v => { v.capture.tools.before.rustc.sha256 = '0'.repeat(64); },
    v => { v.capture.inputs.after.lock.bytes++; }, v => { v.capture.command.args.push('--ignored'); },
    v => { v.capture.command.args = v.capture.command.args.filter(arg => arg !== '--release'); },
    v => { v.capture.command.cwd = '/other'; }, v => { v.capture.termination.signal = 'SIGKILL'; },
    v => { v.capture.versions.rustc = Buffer.from('rustc 1.97.1\nhost: x86_64-pc-windows-msvc'); },
    v => { v.capture.artifacts.compiler.sha256 = '0'.repeat(64); },
    v => { v.expected.environment.RUSTFLAGS = '--cfg skip'; },
    v => { v.capture.metadata = Buffer.from(v.capture.metadata.toString().replace('"packages":', '"packages":[],"packages":')); },
    v => { v.capture.stdout = v.capture.stdout.subarray(0, -1); }]) reject(change);
  for (const change of [rows => { rows[0].profile.opt_level = '0'; }, rows => { rows[0].profile.test = false; },
    rows => { rows[0].package_id += '-other'; }, rows => { rows[1].executable = '/foreign/compiler'; },
    rows => { rows[0].filenames = rows[0].executable; }, rows => { rows[0].filenames.push(rows[0].executable); },
    rows => { rows[0].target.src_path = '/foreign/lifecycle.rs'; }, rows => { rows.push(rows[0]); },
    rows => { rows.at(-1).success = false; }]) reject(v => cargoRows(v, change));
});
test('Cargo verbose association requires the actual matching compiler, test source and rustc invocation', () => {
  for (const change of [s => s.replace('compiler=', 'compiler=/foreign'), s => s.replace('/tools/rustc', '/tools/wrapper'),
    s => s.replace('--test ', ''), s => s.replace('tests/lifecycle.rs', 'tests/other.rs'), s => s + s,
    s => s.replace(' --crate-name', ' CARGO_BIN_EXE_zryna-playground-compiler=/foreign --crate-name'),
    s => s + '\x1b[31m']) reject(v => { v.capture.stderr = Buffer.from(change(v.capture.stderr.toString())); });
});
test('an unreviewed wrapper cannot hide behind an actual-rustc argument, even after tool repinning', () => {
  reject(v => { v.capture.stderr = Buffer.from(v.capture.stderr.toString().replace('/tools/rustc', '/unreviewed/wrapper /tools/rustc')); });
  reject(v => {
    const wrapper = selected('/unreviewed/wrapper');
    v.expected.tools.rustc = wrapper; v.capture.tools.before.rustc = { ...wrapper }; v.capture.tools.after.rustc = { ...wrapper };
    v.expected.environment.RUSTC = wrapper.path; v.capture.command.environment.RUSTC = wrapper.path;
    v.capture.stderr = Buffer.from(v.capture.stderr.toString().replace('/tools/rustc', '/unreviewed/wrapper /tools/rustc'));
  });
});
test('Cargo display permits quoted metadata but not executable wrappers or unapproved prefix controls', () => {
  const value = fixture();
  const prefix = "CARGO_PKG_AUTHORS='' CARGO_PKG_LICENSE='MIT OR Apache-2.0' " +
    "CARGO_PKG_DESCRIPTION='reader'\\''s literal $() metadata' " +
    'CARGO_TARGET_TMPDIR=/workspace/private/target/tmp LD_LIBRARY_PATH=/workspace/private/target/release/deps ';
  value.capture.stderr = Buffer.from(value.capture.stderr.toString().replace('Running `', 'Running `' + prefix));
  assert.doesNotThrow(() => createLifecycleOwnerPlan(value));
  for (const before of ['RUSTC_WRAPPER=/unreviewed/wrapper ', 'RUSTFLAGS=--cfg=skip ',
    "CARGO_PKG_AUTHORS='' CARGO_PKG_AUTHORS=again ", 'LD_LIBRARY_PATH=$(unreviewed) ', "CARGO_PKG_LICENSE='unclosed "])
    reject(v => { v.capture.stderr = Buffer.from(v.capture.stderr.toString().replace('Running `', 'Running `' + before)); });
});
test('complete absent Cargo configurations are required before and after the controlled prebuild', () => {
  reject(v => { delete v.capture.configuration; });
  const rows = ['/cache/cargo/config', '/cache/cargo/config.toml', '/source/.cargo/config',
    '/source/.cargo/config.toml', '/.cargo/config', '/.cargo/config.toml'].map(path => ({ path, present: false }));
  for (const change of [v => { v.capture.configuration.before.pop(); },
    v => { v.capture.configuration.after[0].present = true; },
    v => { v.capture.configuration.before[0].present = true; v.capture.configuration.after[0].present = true; },
    v => { v.capture.configuration.before.push({ ...v.capture.configuration.before[0] }); },
    v => { v.capture.configuration.before[0].path = '/wrong/config'; },
    v => { v.capture.configuration.after[0].present = 'false'; }]) {
    const value = fixture(); value.capture.configuration = { before: structuredClone(rows), after: structuredClone(rows) };
    change(value); assert.throws(() => createLifecycleOwnerPlan(value));
  }
});
test('coherently reselected paths cannot hide the embedded compiler under mounts or fixed image roots', () => {
  for (const target of ['/tmp/target', '/materials/target', '/input/target', '/output/target', '/dev/target',
    '/proc/target', '/sys/target', '/lifecycle/target', '/usr/bin/target', '/lib/target', '/lib64/target',
    '/source/target', '/workspace/../target', '/workspace/space target',
    '/workspace/' + Array(17).fill('deep').join('/') + '/target']) reject(v => rebaseTarget(v, target));
  reject(v => { v.expected.artifacts.test.path = '/foreign/release/deps/lifecycle-0123456789abcdef'; });
});
test('independently declared runtime is closed ordinary data with all dependency edges and Python root', () => {
  reject(v => { v.runtime.files[0].sha256 = '0'.repeat(64); });
  for (const change of [v => { v.runtime.files[0].nlink = 2; }, v => { v.runtime.files[0].kind = 'symlink'; },
    v => { v.runtime.files[0].uid = 65534; }, v => { v.runtime.files[0].bytes = 134217729; },
    v => { v.runtime.files.push({ ...v.runtime.files[0] }); }, v => { v.runtime.files[0].mode = 0o777; },
    v => { v.runtime.files[0].path = '/tmp/shadowed'; },
    v => { v.runtime.elf[0].prefixHex += 'f'; }, v => { v.runtime.elf[0].prefixHex = '00'.repeat(64); },
    v => { v.runtime.elf[0].interpreter = '/missing/loader'; },
    v => { v.runtime.elf[0].needed.push({ name: 'libmissing.so', path: '/missing/library' }); },
    v => { v.runtime.elf.pop(); }, v => { v.runtime.python.files = []; },
    v => { v.runtime.python.imports.push('subprocess'); }, v => { v.runtime.python.entry = '/usr/bin/other'; },
    v => { v.runtime.files[0].bytes = 134217728; },
    v => { v.runtime.files.find(file => file.path === '/lifecycle/test').sha256 = '0'.repeat(64); }]) reject(change, true);
  reject(v => {
    for (let i = 0; i < 260; i++) {
      const path = `/resources/dir${i}/child/file.txt`;
      v.runtime.files.push({ ...selected(path), mode: 0o444, kind: 'regular', nlink: 1, uid: 0, gid: 0 });
      v.runtime.support.push(path);
    }
    v.runtime.files.sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0); v.runtime.support.sort();
  }, true);
});
