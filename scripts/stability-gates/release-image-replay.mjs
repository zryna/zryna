// Retained build/image consistency only. No archived command or image is executed.
import assert from 'node:assert/strict';
import { posix, win32 } from 'node:path';
import { TextDecoder } from 'node:util';
import { canonical, exact, MAX_DOCUMENT, MAX_LOG, portable, sha256 } from './input.mjs';
import { buildArguments, TARGETS, validateBinary } from './release-build-input.mjs';
import { validateToolProbe } from './release-build-capture.mjs';
import { collectorEnvironment } from './cli-performance-hosted-adapter.mjs';
const KEYS = ['cargo', 'rustc', 'cc', 'linker', 'node', 'git'];
const COMMON = ['PATH', 'HOME', 'CARGO_HOME', 'RUSTUP_HOME', 'LANG', 'LC_ALL'];
const WINDOWS = ['SystemRoot', 'WINDIR', 'TEMP', 'TMP', 'USERPROFILE', 'INCLUDE', 'LIB', 'LIBPATH',
  'WindowsSdkDir', 'WindowsSDKVersion', 'UniversalCRTSdkDir', 'UCRTVersion', 'VCToolsInstallDir', 'VCToolsVersion'];
const HOSTED = ['GITHUB_ACTIONS', 'GITHUB_REPOSITORY', 'GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT',
  'RUNNER_OS', 'RUNNER_ARCH', 'ImageOS', 'ImageVersion'];
const FLAGS = { authenticated: false, sourceAuthenticated: false, productionReleaseAdmission: false,
  baselineEligible: false, thresholdsEnabled: false, baseline: null, thresholds: null };
const hex = (value, width) => assert(typeof value === 'string' && new RegExp(`^[a-f0-9]{${width}}$`, 'u').test(value));
const decoder = new TextDecoder('utf-8', { fatal: true });
function json(bytes) {
  const text = decoder.decode(bytes), value = JSON.parse(text), normalized = canonical(value);
  assert(text === normalized || text === `${JSON.stringify(value)}\n`, 'closed producer or canonical JSON; no duplicate keys');
  return value;
}

export function replayReleaseImage({ host, expectedSource, read }) {
  assert(['linux-x86_64', 'windows-x86_64'].includes(host));
  exact(expectedSource, ['commit', 'tree'], 'selected source');
  hex(expectedSource.commit, 40); hex(expectedSource.tree, 40);
  const source = { ...expectedSource }, windows = host === 'windows-x86_64', path = windows ? win32 : posix;
  const target = windows ? 'x86_64-pc-windows-msvc' : 'x86_64-unknown-linux-gnu', name = windows ? 'zryna.exe' : 'zryna';
  const absolute = value => {
    assert(typeof value === 'string' && value.length < 4096 && !/[\x00-\x1f]/u.test(value));
    assert(path.isAbsolute(value) && path.normalize(value) === value, 'normalized selected-host path');
    if (windows) assert(/^[A-Za-z]:\\/u.test(value) && !value.slice(2).includes(':'), 'local Windows path without stream');
    return value;
  };
  const outside = (root, value) => {
    const relative = path.relative(root, value);
    return relative !== '' && (relative === '..' || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative));
  };
  const load = (name, maximum) => {
    portable(name); const bytes = read(name, maximum);
    assert(Buffer.isBuffer(bytes) && bytes.length <= maximum, 'bounded retained bytes');
    const copy = Buffer.alloc(bytes.length); bytes.copy(copy); return copy;
  };
  const document = name => json(load(name, MAX_DOCUMENT));
  const pin = (value, maximum = 512 * 1024 * 1024) => {
    exact(value, ['path', 'bytes', 'sha256', 'mode', 'prefixHex'], 'retained file pin'); absolute(value.path);
    assert(Number.isSafeInteger(value.bytes) && value.bytes > 0 && value.bytes <= maximum);
    hex(value.sha256, 64);
    assert(typeof value.mode === 'string' && /^[0-7]{1,3}$/u.test(value.mode) && parseInt(value.mode, 8).toString(8) === value.mode);
    hex(value.prefixHex, 2 * Math.min(128, value.bytes));
  };
  const projectedPin = value => ({ path: value.path, bytes: value.bytes, sha256: value.sha256 });
  const operator = document('operator-input.json'), build = document('release-build/capture.json');
  exact(operator, ['calibrationDirectory', 'environment', 'expectedHead', 'expectedTree', 'root', 'round', 'tools'], 'operator input');
  absolute(operator.root); exact(operator.tools, KEYS, 'six build tools');
  for (const value of Object.values(operator.tools)) absolute(value);
  assert.equal(operator.expectedHead, source.commit); assert.equal(operator.expectedTree, source.tree);
  assert(['baseline-round-1', 'baseline-round-2'].includes(operator.round));
  if (windows) absolute(operator.calibrationDirectory); else assert.equal(operator.calibrationDirectory, null);
  const fields = ['protocol', 'kind', 'target', 'hostPlatform', 'runnerFamily', 'source', 'environment', 'tools',
    'sdk', 'command', 'build', 'cargoConfiguration', 'logs', 'cli', 'profile', ...Object.keys(FLAGS)];
  exact(build, fields, 'release build receipt');
  assert.equal(build.protocol, 's418-release-build-capture-v1'); assert.equal(build.kind, 'observed-release-profile-build');
  assert.equal(build.target, target); assert.equal(build.hostPlatform, TARGETS[target][0]); assert.equal(build.runnerFamily, TARGETS[target][1]);
  for (const [key, value] of Object.entries(FLAGS)) assert.equal(build[key], value);
  exact(build.source, ['before', 'after'], 'build source records');
  assert.deepEqual(build.source.before, build.source.after, 'stable source record across build');
  for (const [key, value] of Object.entries(source)) assert.equal(build.source.before[key], value);
  // Full manifest/shard inventories belong to the separate source census replay helper.
  hex(build.source.before.manifestSha256, 64); hex(build.source.before.inventorySha256, 64);
  const env = operator.environment; exact(env, [...COMMON, ...(windows ? WINDOWS : ['TMPDIR']), ...HOSTED], 'explicit build environment');
  for (const value of Object.values(env)) assert(typeof value === 'string' && value.length > 0 && value.length <= 32768 && !value.includes('\0'));
  for (const key of ['HOME', 'CARGO_HOME', 'RUSTUP_HOME']) absolute(env[key]);
  assert.equal(env.GITHUB_ACTIONS, 'true'); assert.equal(env.GITHUB_REPOSITORY, 'zryna/zryna'); assert.equal(env.RUNNER_ARCH, 'X64');
  assert.equal(env.RUNNER_OS, windows ? 'Windows' : 'Linux'); assert.equal(env.ImageOS, windows ? 'win22' : 'ubuntu24');
  for (const key of ['GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT']) assert(/^[1-9][0-9]*$/u.test(env[key]));
  if (windows) { assert(/^10\.0\.[0-9]+\.0\\?$/u.test(env.WindowsSDKVersion)); assert(/^10\.0\.[0-9]+\.0$/u.test(env.UCRTVersion)); }
  const targetDirectory = absolute(build.environment.CARGO_TARGET_DIR), directory = path.dirname(targetDirectory), output = path.dirname(directory);
  assert.equal(path.basename(targetDirectory), 'cargo-target'); assert.equal(path.basename(directory), 'release-build');
  assert(outside(operator.root, output), 'external fresh evidence target');
  const effective = { ...env, CARGO_TARGET_DIR: targetDirectory, CARGO_BUILD_JOBS: '2', CARGO_INCREMENTAL: '0',
    ...(windows ? { RUSTFLAGS: '-Clink-arg=/Brepro' } : {}), CARGO_NET_OFFLINE: 'true', RUSTUP_TOOLCHAIN: '1.97.1',
    RUSTC: operator.tools.rustc, CC: operator.tools.cc,
    [windows ? 'CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER' : 'CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER']: operator.tools.linker };
  assert.deepEqual(build.environment, effective, 'unchanged controlled build environment');
  const observed = document('release-build/observed.json');
  assert.deepEqual(build.profile, { requestedByActualCommand: 'release', observed: 'release',
    verifiedFrom: ['actual-command', 'actual-build-output', 'fresh-target-release-path'] });
  assert.deepEqual(observed, { ...build, profile: { requestedByActualCommand: 'release', observed: null } }, 'exact observed-to-final transition');
  exact(build.tools, ['before', 'after'], 'tool epochs');
  assert.deepEqual(build.tools.before, build.tools.after, 'six tool pins stable before/after'); exact(build.tools.before, KEYS, 'six pinned tools');
  for (const key of KEYS) { pin(build.tools.before[key]); assert.equal(build.tools.before[key].path, operator.tools[key]); }
  exact(build.sdk, ['before', 'after'], 'SDK epochs'); assert.deepEqual(build.sdk.before, build.sdk.after);
  if (windows) {
    const version = env.WindowsSDKVersion.replace(/\\$/u, '');
    const sdk = [path.join(env.WindowsSdkDir, 'Include', version, 'shared', 'sdkddkver.h'),
      path.join(env.UniversalCRTSdkDir, 'Include', env.UCRTVersion, 'ucrt', 'corecrt.h')];
    assert(Array.isArray(build.sdk.before)); assert.equal(build.sdk.before.length, 2);
    build.sdk.before.forEach((value, index) => { pin(value, 2 * 1024 * 1024); assert.equal(value.path, sdk[index]); });
  } else assert.equal(build.sdk.before, null);
  const configurationDirectories = [env.CARGO_HOME];
  for (let ancestor = operator.root; ; ancestor = path.dirname(ancestor)) {
    configurationDirectories.push(path.join(ancestor, '.cargo')); if (ancestor === path.dirname(ancestor)) break;
  }
  const configurations = [...new Set(configurationDirectories.flatMap(value => ['config', 'config.toml'].map(name => path.join(value, name))))]
    .map(value => ({ path: value, present: false }));
  exact(build.cargoConfiguration, ['before', 'after'], 'Cargo configuration epochs');
  for (const epoch of ['before', 'after']) {
    assert.deepEqual(build.cargoConfiguration[epoch], configurations, 'all Cargo configurations absent');
  }
  assert.deepEqual(document('release-build/cargo-configuration-before.json'), configurations);
  function command(key, args, timeoutMs) {
    const prefix = `release-build/commands/${key}`, raw = document(`${prefix}.command.json`), result = document(`${prefix}.result.json`);
    const expected = { executable: operator.tools[key === 'release-build' ? 'cargo' : key], args,
      cwd: operator.root, environment: effective, timeoutMs, shell: false };
    assert.deepEqual(raw, expected, 'original exact command'); exact(result, ['command', 'termination', 'logs'], 'command result');
    assert.deepEqual(result.command, raw);
    exact(result.termination, ['exitCode', 'signal', 'error', 'elapsedMs'], 'command termination');
    assert.equal(result.termination.error, null); assert.equal(result.termination.signal, null);
    assert(Number.isFinite(result.termination.elapsedMs) && result.termination.elapsedMs >= 0);
    exact(result.logs, ['stdout', 'stderr'], 'command logs'); let total = 0;
    const streams = {};
    for (const name of ['stdout', 'stderr']) {
      const descriptor = result.logs[name]; exact(descriptor, ['path', 'bytes', 'sha256'], 'raw log binding');
      assert.equal(descriptor.path, path.join(directory, 'commands', `${key}.${name}.raw`)); hex(descriptor.sha256, 64);
      assert(Number.isSafeInteger(descriptor.bytes) && descriptor.bytes >= 0 && descriptor.bytes <= MAX_LOG);
      const bytes = load(`${prefix}.${name}.raw`, MAX_LOG); total += bytes.length;
      assert.equal(bytes.length, descriptor.bytes); assert.equal(sha256(bytes), descriptor.sha256); streams[name] = decoder.decode(bytes);
    }
    assert(total <= MAX_LOG); return { ...result, ...streams };
  }
  const specs = { cargo: ['--version'], rustc: ['-vV'], cc: windows ? [] : ['--version'],
    linker: windows ? ['/?'] : ['--version'], node: ['--version'], git: ['--version'] };
  const probes = Object.fromEntries(KEYS.map(key => {
    const result = command(key, specs[key], 30000); validateToolProbe(result, key, windows); return [key, result];
  }));
  // validateVersions additionally checks the collector's process.version; replay checks retained versions only.
  assert.equal(probes.cargo.stdout.trim(), 'cargo 1.97.1 (c980f4866 2026-06-30)');
  assert.equal(probes.rustc.stdout.trim().split(/\r?\n/u)[0], 'rustc 1.97.1 (8bab26f4f 2026-07-14)');
  assert(probes.rustc.stdout.trim().split(/\r?\n/u).includes(`host: ${target}`)); assert.equal(probes.node.stdout.trim(), 'v22.22.1');
  const originalBuild = command('release-build', buildArguments(target), 20 * 60 * 1000);
  assert.equal(originalBuild.termination.exitCode, 0); assert.deepEqual(build.command, originalBuild.command);
  assert.deepEqual(build.build, originalBuild.termination); assert.deepEqual(build.logs, originalBuild.logs);
  assert(/^\s*Finished `release` profile \[optimized\].*$/mu.test(originalBuild.stderr) &&
    !/^\s*Finished `release` profile \[unoptimized/mu.test(originalBuild.stderr), 'actual retained optimized release marker');
  exact(build.cli, ['before', 'after', 'expectedPath'], 'fresh release image');
  const executable = path.join(targetDirectory, target, 'release', name);
  assert.deepEqual(build.cli.before, { expectedPath: executable, exists: false, sha256: null });
  assert.equal(build.cli.expectedPath, executable); pin(build.cli.after, 64 * 1024 * 1024); assert.equal(build.cli.after.path, executable);
  validateBinary(build.cli.after, target);
  const retained = document('retained-release-cli.json');
  exact(retained, ['format', 'sourceBefore', 'sourceAfter', 'copy', 'links', 'targetDirectory', 'measuredPath',
    'sourceAuthenticated', 'performanceQualified', 'baselineAuthority', 'thresholds'], 'retained CLI copy');
  assert.equal(retained.format, 'zryna.cli-performance-retained-release-cli.v2');
  for (const [key, value] of Object.entries({ sourceAuthenticated: false, performanceQualified: false, baselineAuthority: null, thresholds: null }))
    assert.equal(retained[key], value);
  assert.deepEqual(retained.sourceBefore, build.cli.after); assert.deepEqual(retained.sourceAfter, build.cli.after);
  assert.equal(retained.targetDirectory, targetDirectory);
  const measured = path.join(output, 'retained-release-cli', name);
  assert.equal(retained.measuredPath, measured); assert(outside(targetDirectory, measured));
  assert.deepEqual(retained.copy, { ...build.cli.after, path: measured }); exact(retained.links, ['builtBefore', 'builtAfter', 'copy'], 'copy links');
  for (const value of Object.values(retained.links)) assert(Number.isSafeInteger(value) && value > 0);
  assert.equal(retained.links.builtBefore, retained.links.builtAfter); assert.equal(retained.links.copy, 1);
  const image = load(`retained-release-cli/${name}`, 64 * 1024 * 1024); // Existing primary ZIP per-entry bound.
  assert.equal(image.length, retained.copy.bytes); assert.equal(sha256(image), retained.copy.sha256);
  assert.equal(image.subarray(0, 128).toString('hex'), retained.copy.prefixHex);
  if (windows) {
    const offset = image.readUInt32LE(60);
    assert(offset >= 64 && offset <= 2 * 1024 * 1024 - 8 && offset + 8 <= image.length, 'bounded retained PE header');
    assert(image.subarray(offset, offset + 4).equals(Buffer.from([80, 69, 0, 0])) && image.readUInt16LE(offset + 4) === 0x8664);
  }
  const capture = document('round/capture.json');
  assert.equal(capture.sourceCommit, source.commit); assert.equal(capture.sourceTree, source.tree); assert.equal(capture.host.platform, host);
  assert.equal(capture.round, operator.round); assert.equal(capture.cliBuildClass, 'release');
  assert.deepEqual(capture.tools.cli, projectedPin(retained.copy)); assert.deepEqual(capture.tools.node, projectedPin(build.tools.after.node));
  assert.deepEqual(capture.environment, collectorEnvironment(env, operator.tools.rustc, windows));
  for (const [key, value] of Object.entries({ sourceAuthenticated: false, performanceQualified: false,
    productionReleaseAdmission: false, baselineAuthority: null, thresholds: null })) assert.equal(capture[key], value);
  return { format: 'zryna.release-build-image-reconciliation.v1', host, source, round: operator.round,
    rawBuildImageReconciled: true, target, image: projectedPin(retained.copy), tools: KEYS.map(key => projectedPin(build.tools.after[key])),
    sourceAuthenticated: false, hostAuthenticated: false, performanceQualified: false, productionReleaseAdmission: false,
    baselineAuthority: null, thresholds: null,
    scope: 'Retained release commands/logs/pin claims/image bytes agree; tool provenance, actual build/source execution and host qualification remain unproven' };
}
