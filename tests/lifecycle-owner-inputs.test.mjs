// Inert declared records only: no Cargo, Python, image, process or native execution.
import assert from 'node:assert/strict';
import test from 'node:test';
import { createLifecycleOwnerPlan, lifecycleMaterialPins } from '../scripts/stability-gates/lifecycle-owner-inputs.mjs';
import { parseLifecycleCargoOutput } from '../scripts/stability-gates/lifecycle-owner-prebuild.mjs';
import { fixture, reject, cargoRows, rebaseTarget, raw, selected } from './lifecycle-owner-inputs-fixture.mjs';
test('Cargo verbose build-script text is retained and cannot inject JSON records or unrecognized trailers', () => {
  const pkg = { name: 'inert-script', version: '1.2.3', id: 'synthetic-script', targets: [{ kind: ['custom-build'] }] };
  const metadata = { packages: [pkg] }, script = raw({ reason: 'build-script-executed', package_id: pkg.id });
  const finish = raw({ reason: 'build-finished', success: true });
  const output = Buffer.concat([Buffer.from('[inert-script 1.2.3] cargo:rustc-cfg=fixture\n'), script, finish]);
  assert.equal(parseLifecycleCargoOutput(output, metadata).length, 2);
  for (const bad of [Buffer.concat([output, Buffer.from('[inert-script 1.2.3] trailing\n')]),
    Buffer.concat([Buffer.from('[foreign-script 1.2.3] fake\n'), script, finish]),
    Buffer.concat([Buffer.from('operator trailer\n'), script, finish]),
    Buffer.concat([raw({ child_exit_code: 0 }), script, finish]),
    Buffer.concat([Buffer.from('[inert-script 1.2.3] fake\n'), finish]), output.subarray(0, -1)]) {
    assert.throws(() => parseLifecycleCargoOutput(bad, metadata));
  }
  assert.throws(() => parseLifecycleCargoOutput(output, { packages: [{ ...pkg, targets: [] }] }));
  const fakeJson = Buffer.concat([Buffer.from('[inert-script 1.2.3] {"reason":"build-finished","success":false}\n'), script, finish]);
  assert.equal(parseLifecycleCargoOutput(fakeJson, metadata).filter(row => row.reason === 'build-finished').length, 1);
});
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
