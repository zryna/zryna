// Synthetic original Windows host records only; no process or workload execution.
import assert from 'node:assert/strict';
import { win32 } from 'node:path';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { bindWindowsHostObservations, COMPANION_FLAGS, producerDocument } from '../scripts/stability-gates/cli-performance-campaign-windows-host-v1.mjs';
import { readNumericCampaignOriginal } from '../scripts/stability-gates/cli-performance-campaign-source-v1.mjs';
import { decodeObservedParentAffinity } from '../scripts/stability-gates/windows-parent-affinity-v1/decode.mjs';
export const PREFIX = 'm418-hosted-observations/capture/';
export const DIRECTORY = 'scripts/stability-gates/windows-parent-affinity-v1';
const FILES = ['probe.c', 'build-msvc.cmd', 'decode.mjs', 'decode.test.mjs', 'hosted.ps1', 'live-controls.mjs'];
export function fixture(base = null, mutateHelper = null) {
  const files = new Map(), originals = new Map(), physical = new Map();
  const root = base?.release.operator.root ?? 'C:\\source';
  const calibrationDirectory = base?.release.operator.calibrationDirectory ?? 'C:\\evidence\\windows-job-metrics-calibration';
  const evidence = 'C:\\evidence\\windows-parent-affinity-observation';
  const helper = win32.join(evidence, 'build', 'windows-parent-affinity-v1.exe');
  const buildPath = win32.join(evidence, 'affinity-build.json');
  const source = base ? { commit: base.source.commit, tree: base.source.tree } : { commit: 'a'.repeat(40), tree: 'b'.repeat(40) };
  const pin = (path, raw) => { physical.set(path, raw); return { path, bytes: raw.length, sha256: sha256(raw) }; };
  const inert = path => pin(path, Buffer.from('inert nonexecuted original ' + path));
  const tool = (role, path) => {
    if (!base) return inert(path);
    const value = base.release.build.tools.before[role]; physical.set(value.path, Buffer.from(value.prefixHex, 'hex'));
    return { path: value.path, bytes: value.bytes, sha256: value.sha256 };
  };
  const tools = { buildBefore: { cc: tool('cc', 'C:\\tools\\cl.exe'), linker: tool('linker', 'C:\\tools\\link.exe'), node: tool('node', 'C:\\tools\\node.exe') } };
  tools.buildAfter = structuredClone(tools.buildBefore);
  const environment = base?.release.operator.environment ?? { INCLUDE: 'C:\\sdk\\include', LIB: 'C:\\sdk\\lib', WindowsSdkDir: 'C:\\sdk\\',
    WindowsSDKVersion: '10.0.1.0\\', VCToolsInstallDir: 'C:\\tools\\', VCToolsVersion: '14.1' };
  const operator = { root, calibrationDirectory, tools: Object.fromEntries(Object.entries(tools.buildBefore).map(([key, value]) => [key, value.path])), environment };
  const toolchain = { compiler: tools.buildBefore.cc, linker: tools.buildBefore.linker, node: tools.buildBefore.node };
  const headers = ['windows.h', 'psapi.h', 'inttypes.h', 'stdio.h', 'wchar.h', 'stdint.h', 'stdbool.h', 'stddef.h', 'limits.h', 'string.h', 'stdarg.h', 'assert.h'];
  const libraries = ['kernel32.lib', 'libcmt.lib', 'libvcruntime.lib', 'libucrt.lib', 'oldnames.lib'];
  const sdk = { includeSearch: [environment.INCLUDE], libSearch: [environment.LIB], sdkDirectory: environment.WindowsSdkDir,
    sdkVersion: environment.WindowsSDKVersion, vcToolsDirectory: environment.VCToolsInstallDir, vcToolsVersion: environment.VCToolsVersion };
  const calibration = { ...toolchain, ...sdk, compilerFileVersion: 'synthetic', linkerFileVersion: 'synthetic',
    selectedDirectHeaders: headers.map(name => inert(win32.join(environment.INCLUDE, name))),
    selectedStaticLibraries: libraries.map(name => inert(win32.join(environment.LIB, name))), selectionScope: 'synthetic declarations only' };
  const powershell = value => Buffer.from(JSON.stringify(value, null, 2).replaceAll('\n', '\r\n') + '\n');
  const calibrationRaw = powershell(calibration);
  const calibratedToolchain = pin(win32.join(calibrationDirectory, 'toolchain-before-build.json'), calibrationRaw);
  files.set('windows-job-metrics-calibration/toolchain-before-build.json', calibrationRaw);
  const sourceBefore = FILES.map(name => {
    const logical = `${DIRECTORY}/${name}`, raw = base ? readNumericCampaignOriginal(base.selection, base.sourceKey, logical, 2 * 1024 * 1024)
      : Buffer.from('inert source data ' + logical + '\n'); originals.set(logical, raw);
    return pin(win32.join(root, ...logical.split('/')), raw);
  });
  const logicalWorkflow = '.github/workflows/windows-job-metrics.yml';
  originals.set(logicalWorkflow, base ? readNumericCampaignOriginal(base.selection, base.sourceKey, logicalWorkflow, 65536) : Buffer.from('inert workflow data\n'));
  const workflow = pin(win32.join(root, ...logicalWorkflow.split('/')), originals.get(logicalWorkflow));
  const helperBytes = Buffer.alloc(128); helperBytes.write('MZ');
  helperBytes.writeUInt32LE(64, 60); Buffer.from([80, 69, 0, 0]).copy(helperBytes, 64);
  helperBytes.writeUInt16LE(0x8664, 68); if (mutateHelper) mutateHelper(helperBytes);
  const helperPin = pin(helper, helperBytes); files.set('windows-parent-affinity-observation/build/windows-parent-affinity-v1.exe', helperBytes);
  const header = inert(win32.join(environment.INCLUDE, 'tlhelp32.h'));
  const dependencies = [...calibration.selectedDirectHeaders, ...calibration.selectedStaticLibraries, header];
  const expected = { checkout: { head: source.commit, tree: source.tree }, sourceBefore, workflow, toolchain,
    dependencies, calibratedToolchain, supplementalDirectHeader: header, helper: helperPin };
  const build = { format: 'zryna.windows-parent-affinity-build.v1', ...expected,
    sourceAuthenticated: false, proofQualified: false, performanceQualified: false, hostAdmitted: false, closureStatus: 'blocked', thresholds: null };
  const buildRaw = powershell(build), buildRecord = pin(buildPath, buildRaw);
  files.set('windows-parent-affinity-observation/affinity-build.json', buildRaw);
  const identities = new Map();
  const observed = value => {
    if (!identities.has(value.path)) identities.set(value.path, identities.size + 1);
    const raw = physical.get(value.path); assert(raw);
    return { ...value, mode: '444', prefixHex: raw.subarray(0, 128).toString('hex'), identity: { dev: '1', ino: String(identities.get(value.path)),
      size: String(value.bytes), mode: String(0o100444), nlink: '1', mtimeNs: '1', ctimeNs: '1' } };
  };
  const context = { expected, declaredDependencies: dependencies.slice(0, -1), calibratedTools: toolchain,
    sdkExpected: sdk, sdkDeclared: structuredClone(sdk), runtimeNode: toolchain.node,
    runtimeNodeRealpath: toolchain.node.path, selectedNodeRealpath: toolchain.node.path,
    buildRecord, buildRawSha256: sha256(buildRaw) };
  const sequence = [...sourceBefore, workflow, toolchain.compiler, toolchain.linker, toolchain.node, ...dependencies,
    calibratedToolchain, header, helperPin, toolchain.node, buildRecord];
  context.observed = sequence.map(observed);
  const companions = [], commands = [];
  for (const [index, phase] of ['before', 'after'].entries()) {
    const prefix = PREFIX + `numeric-host-${phase}-affinity`;
    const record = { format: 'zryna.windows-parent-affinity.v1', status: 'observed', stage: 0, error: 0, cleanupError: 0,
      childPid: 456 + index, parentPid: 123, childCreated: String(133000000000000001n + BigInt(index)),
      parentCreated: '133000000000000000', parentCreatedAfter: '133000000000000000', parentLiveBefore: true, parentLiveAfter: true,
      snapshotsMatch: true, snapshotClosed: true, parentHandleClosed: true, groupCount: 1, groupId: 0,
      activeProcessorCount: 4, processMask: '15', systemMask: '15', maskBitCount: 4,
      afterGroupCount: 1, afterGroupId: 0, afterActiveProcessorCount: 4, afterProcessMask: '15', afterSystemMask: '15', elapsedMs: 5 };
    const stdout = Buffer.from(JSON.stringify(record) + '\n'), stderr = Buffer.alloc(0);
    const command = { executable: helper, args: ['--parent-pid', '123'], cwd: root, timeoutMs: 10000 }; commands.push(command);
    const result = { exitCode: 0, signal: null, error: null, elapsedMs: 10,
      streams: { stdout: { bytes: stdout.length, sha256: sha256(stdout) }, stderr: { bytes: 0, sha256: sha256(stderr) } }, ...COMPANION_FLAGS };
    const decoded = decodeObservedParentAffinity({ stdout, stderr, exitCode: 0, signal: null, error: null, elapsedMs: 10 },
      { parentPid: 123, parentCreated: index ? record.parentCreated : null });
    companions.push({ collectorNodePid: 123, native: { decoded, buildRecord,
      capture: { command, stdoutSha256: sha256(stdout), stderrSha256: sha256(stderr) } } });
    for (const [suffix, value] of [['.command.json', command], ['.result.json', result],
      ['-materials-before.json', context], ['-materials-after.json', context]]) files.set(prefix + suffix, Buffer.from(canonical(JSON.parse(JSON.stringify(value)))));
    files.set(prefix + '.stdout.raw', stdout); files.set(prefix + '.stderr.raw', stderr); files.set(prefix + '-build.raw.json', buildRaw);
  }
  const read = (path, maximum) => { const raw = files.get(path); assert(raw && raw.length <= maximum, 'bounded synthetic original'); return Buffer.from(raw); };
  const readOriginal = (path, maximum) => { const raw = originals.get(path); assert(raw && raw.length <= maximum); return Buffer.from(raw); };
  return { source, operator, tools, files, originals, companions, read, readOriginal, calibration, context };
}
export const run = f => bindWindowsHostObservations(f);
export function mutateDocument(f, path, change) {
  const value = producerDocument(f.files.get(path)); change(value); f.files.set(path, Buffer.from(canonical(JSON.parse(JSON.stringify(value)))));
}
