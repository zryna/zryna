// Pure injected observation tests on either host. Never executes Cargo or a native binary.
import './lifecycle-owner-capture-io.test.mjs';
import assert from 'node:assert/strict';
import test from 'node:test';
import { captureLifecyclePrebuild } from '../scripts/stability-gates/lifecycle-owner-capture.mjs';
import { captureFixture } from './lifecycle-owner-capture-fixture.mjs';
const names = ['cargo-version', 'rustc-version', 'rustc-sysroot', 'rustc-target-libdir',
  'metadata-before', 'tree-before', 'build', 'metadata-after', 'tree-after'];
const run = fixture => captureLifecyclePrebuild(fixture.input, fixture.adapters);
const stored = (f, name) => JSON.parse(f.files.get(name));

test('synthetic observations preserve all nine fixed commands/raw streams without granting authority', async () => {
  const f = captureFixture(), result = await run(f);
  assert.deepEqual(f.calls.map(call => call.name), names);
  assert.equal(result.observationMode, 'synthetic-observation-adapters');
  assert.equal(result.rawValidated, true); assert.equal(result.unchangedSource, true);
  for (const key of ['sourceAuthenticated', 'hostAuthenticated', 'executionApproved', 'toolsOriginAuthenticated',
    'artifactsIndependentlyApproved', 'runtimeApproved', 'testExecuted', 'ownerExecuted', 'proofQualified', 'performanceQualified'])
    assert.equal(result[key], false);
  assert.equal(result.thresholds, null); assert.equal(result.baselineAuthority, null);
  assert.equal(result.discoveredArtifacts.compiler.path, f.artifacts.compiler);
  assert.equal(result.discoveredArtifacts.test.path, f.artifacts.test);
  assert(f.files.get('build.stdout.raw').toString().startsWith('[autocfg 1.5.1] cargo:'));
  assert(f.files.has('cache-closure-before.json') && f.files.has('cache-closure-after.json'));
  assert(f.files.has('licenses-before/licenses/rust/fixture/LICENSE'));
  assert(f.files.has('licenses-after/licenses/rust/fixture/LICENSE'));
  for (const call of f.calls) {
    const command = stored(f, call.name + '.command.json');
    assert.equal(command.timeoutMs, call.name === 'build' ? 1200000 : 120000);
    assert.deepEqual(command.environment, f.input.environment); assert.equal(Object.keys(command.environment).length, 13);
    assert(f.files.has(call.name + '.result.json') && f.files.has(call.name + '.stderr.raw'));
    assert(!call.executable.startsWith(f.input.output), 'no built artifact is executed');
  }
  assert.deepEqual(f.calls.find(call => call.name === 'build').args, ['test', '--locked', '--offline', '--release',
    '-p', 'zryna-playground-compiler', '--test', 'lifecycle', '--no-run', '--message-format=json', '-vv']);
});
test('bad selectors/flags/shadowed paths and unknown adapters fail before preparing or commands', async () => {
  for (const edit of [f => f.input.source.tree = 'invalid', f => f.input.environment.RUSTFLAGS = '-Copt-level=0',
    f => f.input.environment.CARGO_BUILD_JOBS = '3', f => f.input.environment.RUSTC = '/other/rustc',
    f => f.input.environment.PATH = '/unreviewed/bin', f => f.input.output = '/tmp/hidden',
    f => f.input.output = f.input.root + '/capture', f => f.input.tools.cargo = '../cargo',
    f => f.adapters.unknown = () => {}, f => f.adapters.host = () => ({ platform: 'win32', arch: 'x64', node: 'v22.22.1' })]) {
    const f = captureFixture(); edit(f); await assert.rejects(() => run(f));
    assert.equal(f.state.prepared, false); assert.deepEqual(f.calls, []);
  }
});
test('distribution license naming remains intact through both capture phases', async () => {
  const f = captureFixture(), path = 'licenses/rust/toml-0.9.12+spec-1.1.0/LICENSE-MIT';
  f.hooks.materials = value => { value.licenses[0].path = path; };
  await run(f);
  assert(f.files.has('licenses-before/' + path)); assert(f.files.has('licenses-after/' + path));
});
test('output and target remain create-only; retained files are never overwritten', async () => {
  const f = captureFixture(); await run(f); const before = Buffer.from(f.files.get('discovery.json'));
  await assert.rejects(() => run(f), /fresh output/); assert(before.equals(f.files.get('discovery.json')));
});
test('uncertain build cleanup or thrown build launches no postflight subprocess or Git census', async () => {
  for (const thrown of [false, true]) {
    const f = captureFixture(); let censuses = 0;
    f.hooks.snapshot = name => { if (name === 'source') censuses++; };
    f.hooks.run = (name, value) => {
      if (name === 'build') { if (thrown) throw Error('lost child observation'); value.error = 'CLEANUP'; }
    };
    await assert.rejects(() => run(f));
    assert.deepEqual(f.calls.map(call => call.name), names.slice(0, 7)); assert.equal(censuses, 1);
    assert(f.files.has('after-unavailable.json')); assert(f.files.has('failure.json'));
    assert(!f.files.has('source-after.json')); assert(!f.files.has('discovery.json'));
    if (!thrown) assert.equal(stored(f, 'build.result.json').termination.error, 'CLEANUP');
  }
});
test('confirmed ordinary build failure still retains complete postflight before rejecting', async () => {
  const f = captureFixture(); f.hooks.run = (name, value) => { if (name === 'build') value.exitCode = 101; };
  await assert.rejects(() => run(f)); assert.deepEqual(f.calls.map(call => call.name), names);
  assert(f.files.has('source-after.json')); assert(f.files.has('cache-closure-after.json'));
  assert.equal(stored(f, 'build.result.json').termination.exitCode, 101);
});
test('out-of-range Linux exit codes are unconfirmed and cannot start postflight', async () => {
  for (const code of [-1, 256]) {
    const f = captureFixture(); f.hooks.run = (name, value) => { if (name === 'build') value.exitCode = code; };
    await assert.rejects(() => run(f)); assert.deepEqual(f.calls.map(call => call.name), names.slice(0, 7));
    assert.equal(stored(f, 'build.result.json').termination.exitCode, code); assert(f.files.has('after-unavailable.json'));
  }
});
test('final source census follows both post-build commands and catches their source mutation', async () => {
  const f = captureFixture(); let changed = false;
  f.hooks.run = name => { if (name === 'tree-after') changed = true; };
  f.hooks.snapshot = (name, value, state) => { if (name === 'source' && state.after && changed) value.inventorySha256 = 'd'.repeat(64); };
  await assert.rejects(() => run(f), /source census changed/);
  assert.equal(stored(f, 'source-after.json').inventorySha256, 'd'.repeat(64));
});
test('a confirmed failing post-build query still gets final census; uncertain query cannot start Git', async () => {
  for (const uncertain of [false, true]) {
    const f = captureFixture();
    f.hooks.run = (name, value) => { if (name === 'metadata-after') { value.exitCode = 1; if (uncertain) value.error = 'CLEANUP'; } };
    await assert.rejects(() => run(f)); assert.equal(f.files.has('source-after.json'), !uncertain);
    assert.equal(f.files.has('after-unavailable.json'), uncertain);
  }
});
test('binary classification and artifact identity come from one retained pin per artifact', async () => {
  const f = captureFixture(), counts = new Map();
  f.hooks.snapshot = (name, value) => {
    if (Object.values(f.artifacts).includes(name)) {
      counts.set(name, (counts.get(name) ?? 0) + 1);
      assert.equal(counts.get(name), 1, 'never combine one identity with another ELF pin');
    }
  };
  const result = await run(f), pins = stored(f, 'artifact-pins.json');
  for (const key of ['test', 'compiler']) assert.equal(result.discoveredArtifacts[key].sha256, pins[key].sha256);
  assert.equal(counts.size, 2);
});
test('Cargo trailers/wrappers/truncated JSON remain retained and rejected without stripping', async () => {
  for (const edit of [r => r.stdout = Buffer.concat([r.stdout, Buffer.from('{"child_exit_code":0}\n')]),
    r => r.stderr = Buffer.from(r.stderr.toString().replace('/tools/rustc', '/wrapper /tools/rustc')),
    r => r.stdout = r.stdout.subarray(0, -1)]) {
    const f = captureFixture(); let original;
    f.hooks.run = (name, value) => { if (name === 'build') { edit(value); original = Buffer.from(value.stdout); } };
    await assert.rejects(() => run(f)); assert(f.files.get('build.stdout.raw').equals(original));
    assert(f.files.has('source-after.json')); assert(!f.files.has('discovery.json'));
  }
});
test('changed inputs, tools, configurations, cache or host-native artifact fail with retained observations', async () => {
  for (const edit of [f => f.hooks.snapshot = (name, value, s) => { if (s.after && name === '/tools/rustc') value.sha256 = 'e'.repeat(64); },
    f => f.hooks.snapshot = (name, value, s) => { if (s.after && name.endsWith('Cargo.lock')) value.sha256 = 'e'.repeat(64); },
    f => f.hooks.snapshot = (name, value, s) => { if (name === 'configuration' && s.after) value[0].present = true; },
    f => f.hooks.materials = (value, s) => { if (s.after) value.closure = Buffer.from('{"changed":true}\n'); },
    f => f.hooks.snapshot = (name, value) => { if (name === f.artifacts.test) value.prefixHex = '00'.repeat(64); }]) {
    const f = captureFixture(); edit(f); await assert.rejects(() => run(f));
    assert(f.files.has('failure.json')); assert(!f.files.has('discovery.json'));
  }
});
test('pre-query workspace/config failure starts no command; caller object mutation cannot rewrite command selection', async () => {
  for (const configured of [false, true]) {
    const f = captureFixture();
    f.hooks.snapshot = (name, value) => {
      if (configured && name === 'configuration') value[0].present = true;
      if (!configured && name === 'workspace') throw Error('security workspace drift');
    };
    await assert.rejects(() => run(f)); assert.deepEqual(f.calls, []); assert(f.files.has('failure.json'));
  }
  const f = captureFixture();
  f.hooks.run = name => { if (name === 'cargo-version') f.input.environment.RUSTC = '/changed'; };
  await run(f); assert.equal(stored(f, 'build.command.json').environment.RUSTC, '/tools/rustc');
});
