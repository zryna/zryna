// Numeric-only companion observations; original host v1 files and legacy defaults stay intact.
import assert from 'node:assert/strict';
import { lstatSync, realpathSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { canonical, exact, MAX_DOCUMENT, parse, readSafe, sha256 } from './input.mjs';
import { pinFile } from './release-build-input.mjs';
import { execute } from './process.mjs';
import { decodeObservedParentAffinity } from './windows-parent-affinity-v1/decode.mjs';
const sessions = new WeakMap();
const FILES = ['probe.c', 'build-msvc.cmd', 'decode.mjs', 'decode.test.mjs', 'hosted.ps1', 'live-controls.mjs'];
const cleanFlags = () => ({ sourceAuthenticated: false, proofQualified: false, performanceQualified: false,
  hostAdmitted: false, baselineAuthority: null, thresholdAuthority: null, thresholds: null, closureStatus: 'blocked' });
export function numericRound(round) {
  assert(['baseline-round-1', 'baseline-round-2', 'held-out-candidate'].includes(round), 'fixed numeric round role');
  return round;
}
export function numericOperatorInput(input) {
  const copy = parse(Buffer.from(canonical(input)));
  exact(copy, ['calibrationDirectory', 'environment', 'expectedHead', 'expectedTree', 'root', 'round', 'tools'], 'numeric operator input');
  for (const key of ['expectedHead', 'expectedTree']) assert.match(copy[key], /^[a-f0-9]{40}$/u);
  numericRound(copy.round);
  return copy;
}
function save(output, name, value) {
  writeFileSync(join(output, name), canonical(value), { flag: 'wx', mode: 0o600 });
}
const projection = pin => ({ path: pin.path, bytes: pin.bytes, sha256: pin.sha256 });
export function affinityLocations(root, calibrationDirectory) {
  assert(typeof calibrationDirectory === 'string' && resolve(calibrationDirectory) === calibrationDirectory);
  const evidence = join(dirname(calibrationDirectory), 'windows-parent-affinity-observation');
  return { evidence, helper: join(evidence, 'build', 'windows-parent-affinity-v1.exe'),
    source: join(root, 'scripts', 'stability-gates', 'windows-parent-affinity-v1'),
    workflow: join(root, '.github', 'workflows', 'windows-job-metrics.yml') };
}
export function validateAffinityBuild(build, context) {
  exact(build, ['format', 'checkout', 'sourceBefore', 'workflow', 'toolchain', 'dependencies', 'calibratedToolchain',
    'supplementalDirectHeader', 'sourceAuthenticated', 'proofQualified', 'performanceQualified', 'hostAdmitted',
    'closureStatus', 'thresholds', 'helper'], 'actual affinity build record');
  assert.equal(build.format, 'zryna.windows-parent-affinity-build.v1');
  for (const key of ['sourceAuthenticated', 'proofQualified', 'performanceQualified', 'hostAdmitted']) assert.equal(build[key], false);
  assert.equal(build.closureStatus, 'blocked'); assert.equal(build.thresholds, null);
  for (const [key, value] of Object.entries(context.expected)) assert.deepEqual(build[key], value, `same actual affinity ${key}`);
  assert.deepEqual(context.declaredDependencies, context.expected.dependencies.slice(0, -1), 'same calibrated SDK/CRT material pins');
  assert.deepEqual(context.calibratedTools, context.expected.toolchain, 'same actual calibrated compiler/linker/Node');
  assert.deepEqual(context.sdkDeclared, context.sdkExpected, 'same actual SDK search/version selection');
  assert.equal(context.runtimeNodeRealpath, context.selectedNodeRealpath, 'collector is the calibrated Node');
  assert.equal(context.runtimeNode.sha256, context.expected.toolchain.node.sha256);
  assert.equal(context.runtimeNode.bytes, context.expected.toolchain.node.bytes);
  assert.equal(context.buildRecord.sha256, context.buildRawSha256, 'same bounded build metadata bytes');
}
function bindings(input, root, locations, raw) {
  const observed = [];
  const pin = path => {
    const full = pinFile(path, path.endsWith('.json') ? MAX_DOCUMENT : undefined);
    const stat = lstatSync(path, { bigint: true });
    observed.push({ ...full, identity: Object.fromEntries(['dev', 'ino', 'size', 'mode', 'nlink', 'mtimeNs', 'ctimeNs']
      .map(key => [key, String(stat[key])])) });
    return projection(full);
  };
  const calibratedPath = join(input.calibrationDirectory, 'toolchain-before-build.json');
  const calibration = JSON.parse(readSafe(input.calibrationDirectory, 'toolchain-before-build.json').toString('utf8'));
  const declaredDependencies = [...calibration.selectedDirectHeaders, ...calibration.selectedStaticLibraries];
  assert.equal(calibration.selectedDirectHeaders.length, 12); assert.equal(calibration.selectedStaticLibraries.length, 5);
  const includes = input.environment.INCLUDE.split(';').filter(Boolean), libraries = input.environment.LIB.split(';').filter(Boolean);
  const header = includes.map(directory => join(directory, 'tlhelp32.h')).find(path => lstatSync(path, { throwIfNoEntry: false }));
  assert(header, 'actual first selected tlhelp32.h');
  const expected = { checkout: { head: input.expectedHead, tree: input.expectedTree },
    sourceBefore: FILES.map(name => pin(join(locations.source, name))), workflow: pin(locations.workflow),
    toolchain: { compiler: pin(input.tools.cc), linker: pin(input.tools.linker), node: pin(input.tools.node) },
    dependencies: [...declaredDependencies.map(row => pin(row.path)), pin(header)],
    calibratedToolchain: pin(calibratedPath), supplementalDirectHeader: pin(header), helper: pin(locations.helper) };
  const sdk = { includeSearch: includes, libSearch: libraries, sdkDirectory: input.environment.WindowsSdkDir,
    sdkVersion: input.environment.WindowsSDKVersion, vcToolsDirectory: input.environment.VCToolsInstallDir,
    vcToolsVersion: input.environment.VCToolsVersion };
  return { expected, declaredDependencies,
    calibratedTools: { compiler: calibration.compiler, linker: calibration.linker, node: calibration.node },
    sdkExpected: sdk, sdkDeclared: Object.fromEntries(Object.keys(sdk).map(key => [key, calibration[key]])),
    runtimeNode: pin(process.execPath), runtimeNodeRealpath: realpathSync(process.execPath),
    selectedNodeRealpath: realpathSync(input.tools.node), buildRecord: pin(join(locations.evidence, 'affinity-build.json')),
    buildRawSha256: sha256(raw), observed };
}
async function windowsObservation(input, root, output, prefix, before) {
  const locations = affinityLocations(root, input.calibrationDirectory);
  const raw = readSafe(locations.evidence, 'affinity-build.json');
  writeFileSync(join(output, `${prefix}-build.raw.json`), raw, { flag: 'wx', mode: 0o600 });
  const build = JSON.parse(raw.toString('utf8')), pre = bindings(input, root, locations, raw);
  save(output, `${prefix}-materials-before.json`, pre);
  validateAffinityBuild(build, pre);
  const command = { executable: locations.helper, args: ['--parent-pid', String(process.pid)], cwd: root, timeoutMs: 10000 };
  save(output, `${prefix}.command.json`, command);
  let result, post, failure;
  try {
    result = await execute(command.executable, command.args, { root, env: input.environment, timeoutMs: command.timeoutMs });
  } catch (error) {
    failure = error;
    save(output, `${prefix}.execution-throw.json`, { message: String(error.stack ?? error), ...cleanFlags() });
  } finally {
    if (result) {
      const streams = {};
      for (const name of ['stdout', 'stderr']) {
        writeFileSync(join(output, `${prefix}.${name}.raw`), result[name], { flag: 'wx', mode: 0o600 });
        streams[name] = { bytes: result[name].length, sha256: sha256(result[name]) };
      }
      save(output, `${prefix}.result.json`, { exitCode: result.exitCode, signal: result.signal,
        error: result.error, elapsedMs: result.elapsedMs, streams, ...cleanFlags() });
    }
    try {
      post = bindings(input, root, locations, raw);
      save(output, `${prefix}-materials-after.json`, post);
    } catch (error) {
      save(output, `${prefix}-materials-after-failure.json`, { message: String(error.stack ?? error), ...cleanFlags() });
      failure ??= error;
    }
  }
  if (failure) throw failure;
  assert.deepEqual(post, pre, 'same observed build/source/workflow/tool/helper material after native query');
  const decoded = decodeObservedParentAffinity(result, { parentPid: process.pid, parentCreated: before?.parentCreated ?? null });
  return { binding: pre, record: { decoded, buildRecord: pre.buildRecord, capture: { command,
    stdoutSha256: sha256(result.stdout), stderrSha256: sha256(result.stderr) } } };
}
export async function captureNumericCompanion(input, root, output, phase, previous = null) {
  assert(['before', 'after'].includes(phase));
  const source = { commit: input.expectedHead, tree: input.expectedTree };
  let state;
  if (phase === 'before') {
    assert.equal(previous, null); state = { pid: process.pid, root, output, source, parentCreated: null, completed: false };
  } else {
    state = sessions.get(previous);
    assert(state && !state.completed && state.pid === process.pid && state.root === root && state.output === output,
      'same live collector companion, once only');
    assert.deepEqual(state.source, source);
    state.completed = true; // Consume before any await; a failed after observation cannot be replayed.
  }
  const bytes = readSafe(output, `host-${phase}.json`), host = parse(bytes);
  assert.equal(host.phase, phase);
  const platform = process.platform === 'win32' ? 'windows-x86_64' : 'linux-x86_64';
  assert.equal(host.platform, platform); assert.equal(host.sourceAuthenticated, false); assert.equal(host.performanceQualified, false);
  const record = { format: 'zryna.cli-performance-numeric-host-companion.v1', phase, platform, source,
    collectorNodePid: process.pid, original: { path: `host-${phase}.json`, bytes: bytes.length, sha256: sha256(bytes) },
    kernel: host.kernel, ...cleanFlags() };
  if (process.platform === 'win32') {
    const native = await windowsObservation(input, root, output, `numeric-host-${phase}-affinity`, state);
    record.native = native.record;
    if (phase === 'after') assert.deepEqual(native.binding, state.nativeBinding, 'same material across both phases');
    else state.nativeBinding = native.binding;
    state.parentCreated ??= native.record.decoded.parent.created;
  }
  save(output, `numeric-host-${phase}.json`, record);
  if (phase === 'before') {
    state.inputSha256 = sha256(canonical(input)); state.hostSha256 = sha256(bytes);
    state.companionSha256 = sha256(readSafe(output, 'numeric-host-before.json'));
    const key = Object.freeze({}); sessions.set(key, state); return key;
  }
  return null;
}

// A copied record cannot stand in for this collector's live before session. No extra native query.
export function inspectNumericCompanionBefore(key, input, root, output) {
  assert.equal(arguments.length, 4, 'fixed private companion inspector');
  const state = sessions.get(key);
  assert(state && !state.completed && state.pid === process.pid && state.root === root && state.output === output,
    'same live before companion required');
  assert.equal(state.inputSha256, sha256(canonical(input)), 'same original numeric operator');
  assert.deepEqual(state.source, { commit: input.expectedHead, tree: input.expectedTree });
  const rawHost = readSafe(output, 'host-before.json'), rawCompanion = readSafe(output, 'numeric-host-before.json');
  assert.equal(sha256(rawHost), state.hostSha256, 'original before host bytes unchanged');
  assert.equal(sha256(rawCompanion), state.companionSha256, 'original before companion bytes unchanged');
  const host = parse(rawHost), companion = parse(rawCompanion);
  assert.equal(companion.collectorNodePid, process.pid); assert.deepEqual(companion.source, state.source);
  if (process.platform === 'win32') {
    const locations = affinityLocations(root, input.calibrationDirectory), raw = readSafe(locations.evidence, 'affinity-build.json');
    const current = bindings(input, root, locations, raw);
    assert.deepEqual(current, state.nativeBinding, 'original native material identities remain live');
    validateAffinityBuild(JSON.parse(raw.toString('utf8')), current);
    assert.equal(companion.native.decoded.parent.created, state.parentCreated);
  }
  return structuredClone({ host, companion, nativeBinding: state.nativeBinding ?? null });
}
