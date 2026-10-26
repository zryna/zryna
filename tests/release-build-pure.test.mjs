import assert from 'node:assert/strict';
import test from 'node:test';
import { delimiter, join, resolve } from 'node:path';
import { buildArguments, cargoConfigurationPaths, controlledEnvironment, outside, validateBinary } from '../scripts/stability-gates/release-build-input.mjs';
import { CENSUS_LIMITS, indexMatchesHead, parseIndex, shardRecords, sourceUnchanged } from '../scripts/stability-gates/source-census-v2.mjs';

const blob = '1'.repeat(40);
const environment = () => ({ PATH: [resolve('/tools'), resolve('/usr/bin')].join(delimiter), HOME: resolve('/home/operator'), CARGO_HOME: resolve('/cache/cargo'),
  RUSTUP_HOME: resolve('/cache/rustup'), LANG: 'C.UTF-8', LC_ALL: 'C.UTF-8', TMPDIR: resolve('/tmp'),
  GITHUB_ACTIONS: 'true', GITHUB_REPOSITORY: 'zryna/zryna', GITHUB_RUN_ID: '123',
  GITHUB_RUN_ATTEMPT: '1', RUNNER_OS: 'Linux', RUNNER_ARCH: 'X64', ImageOS: 'ubuntu24', ImageVersion: '20261024.1' });
const options = { target: 'x86_64-unknown-linux-gnu', targetDirectory: resolve('/evidence/fresh-target'),
  tools: { rustc: resolve('/tools/rustc'), cc: resolve('/tools/cc'), linker: resolve('/tools/ld') } };
test('release argv fixes locked offline native target package binary and release profile', () => {
  assert.deepEqual(buildArguments(options.target), ['build', '--locked', '--offline', '--release', '--target',
    options.target, '-p', 'zryna', '--bin', 'zryna']);
});
test('controlled environment explicitly sets jobs incremental compiler linker and target', () => {
  const actual = controlledEnvironment(environment(), options);
  assert.equal(actual.CARGO_BUILD_JOBS, '2'); assert.equal(actual.CARGO_INCREMENTAL, '0');
  assert.equal(actual.CARGO_NET_OFFLINE, 'true'); assert.equal(actual.RUSTUP_TOOLCHAIN, '1.97.1');
  assert.equal(actual.CARGO_TARGET_DIR, options.targetDirectory); assert.equal(actual.RUSTC, options.tools.rustc);
  assert.equal(actual.CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER, options.tools.linker);
});
test('controlled environment rejects unprojected flags credentials and missing keys', () => {
  for (const key of ['RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'RUSTC_BOOTSTRAP',
    'CARGO_PROFILE_RELEASE_OPT_LEVEL', 'GITHUB_TOKEN'])
    assert.throws(() => controlledEnvironment({ ...environment(), [key]: 'injected' }, options));
  const missing = environment(); delete missing.PATH;
  assert.throws(() => controlledEnvironment(missing, options));
  assert.throws(() => controlledEnvironment({ ...environment(), PATH: 'a\0b' }, options));
});
test('Cargo configuration census includes both names at source every ancestor and Cargo home', () => {
  const fixture = process.platform === 'win32' ? {
    root: 'C:\\work\\source', cargoHome: 'C:\\cache\\cargo', expected: [
      'C:\\cache\\cargo\\config', 'C:\\cache\\cargo\\config.toml',
      'C:\\work\\source\\.cargo\\config', 'C:\\work\\source\\.cargo\\config.toml',
      'C:\\work\\.cargo\\config', 'C:\\work\\.cargo\\config.toml', 'C:\\.cargo\\config', 'C:\\.cargo\\config.toml'],
  } : { root: '/work/source', cargoHome: '/cache/cargo', expected: [
    '/cache/cargo/config', '/cache/cargo/config.toml', '/work/source/.cargo/config', '/work/source/.cargo/config.toml',
    '/work/.cargo/config', '/work/.cargo/config.toml', '/.cargo/config', '/.cargo/config.toml'] };
  assert.deepEqual(cargoConfigurationPaths(fixture.root, fixture.cargoHome), fixture.expected);
  assert.throws(() => cargoConfigurationPaths(fixture.root, 'relative-cache'));
});
test('controlled environment rejects wrong runner family and nonnative target declaration', () => {
  assert.throws(() => controlledEnvironment({ ...environment(), ImageOS: 'ubuntu22' }, options));
  assert.throws(() => controlledEnvironment(environment(), { ...options, target: 'aarch64-unknown-linux-gnu' }));
});
test('external path check rejects source root and children but permits sibling output', () => {
  const source = resolve('/source');
  assert.equal(outside(source, source), false); assert.equal(outside(source, join(source, 'output')), false);
  assert.equal(outside(source, resolve('/evidence/output')), true);
});
test('source index requires regular stage zero and unique portable case folded paths', () => {
  assert.deepEqual(parseIndex(`100644 ${blob} 0\tCargo.toml\0`), [{ mode: '100644', blob, path: 'Cargo.toml' }]);
  for (const text of [`100644 ${blob} 1\tfile\0`, `120000 ${blob} 0\tlink\0`,
    `100644 ${blob} 0\tFile\0` + `100644 ${blob} 0\tfile\0`, `100644 ${blob} 0\t../escape\0`,
    `100644 ${blob} 0\tfile`, '']) assert.throws(() => parseIndex(text));
});
test('source index must equal complete HEAD mode blob path and cardinality', () => {
  const entries = parseIndex(`100755 ${blob} 0\ttool\0`);
  indexMatchesHead(entries, `100755 blob ${blob}\ttool\0`);
  for (const text of [`100644 blob ${blob}\ttool\0`, `100755 blob ${'2'.repeat(40)}\ttool\0`,
    `100755 blob ${blob}\tother\0`, '']) assert.throws(() => indexMatchesHead(entries, text));
});
test('source inventory shard boundary preserves every record within 128 KiB', () => {
  const records = ['a'.repeat(CENSUS_LIMITS.shardBytes), 'second\n', 'third\n'];
  const shards = [...shardRecords(records)];
  assert.equal(shards.length, 2); assert.equal(shards.join(''), records.join(''));
  assert(shards.every(shard => Buffer.byteLength(shard) <= 128 * 1024));
  assert.throws(() => [...shardRecords(['a'.repeat(CENSUS_LIMITS.shardBytes + 1)])]);
});
test('source before after comparison rejects inventory commit and shard changes', () => {
  const before = { commit: blob, inventorySha256: '2'.repeat(64), shards: [{ sha256: '3'.repeat(64) }] };
  sourceUnchanged(before, structuredClone(before));
  for (const after of [{ ...before, commit: '4'.repeat(40) }, { ...before, inventorySha256: '5'.repeat(64) },
    { ...before, shards: [{ sha256: '6'.repeat(64) }] }]) assert.throws(() => sourceUnchanged(before, after));
});
test('Linux CLI validation requires ELF class machine and executable mode together', () => {
  const prefix = Buffer.alloc(64); Buffer.from([127, 69, 76, 70, 2, 1]).copy(prefix); prefix.writeUInt16LE(62, 18);
  const pin = { prefixHex: prefix.toString('hex'), mode: '755' };
  validateBinary(pin, options.target);
  assert.throws(() => validateBinary({ ...pin, mode: '644' }, options.target));
  prefix.writeUInt16LE(183, 18);
  assert.throws(() => validateBinary({ ...pin, prefixHex: prefix.toString('hex') }, options.target));
});

test('release capture entry module links and exports the real operator API', async () => {
  const entry = await import('../scripts/stability-gates/release-build-capture.mjs');
  assert.equal(typeof entry.captureReleaseBuild, 'function');
});
