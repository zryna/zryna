// Pure original-record binding only. The calling owner supplies fixed key-bound readers.
import assert from 'node:assert/strict';
import { win32 } from 'node:path';
import { canonical, exact, MAX_DOCUMENT, sha256 } from './input.mjs';
import { validateAffinityBuild } from './cli-performance-numeric-host.mjs';
import { validateBinary } from './release-build-input.mjs';
import { decodeObservedParentAffinity, RECORD_BYTES } from './windows-parent-affinity-v1/decode.mjs';
export const COMPANION_FLAGS = Object.freeze({ sourceAuthenticated: false, proofQualified: false, performanceQualified: false,
  hostAdmitted: false, baselineAuthority: null, thresholdAuthority: null, thresholds: null, closureStatus: 'blocked' });
const FILES = ['probe.c', 'build-msvc.cmd', 'decode.mjs', 'decode.test.mjs', 'hosted.ps1', 'live-controls.mjs'];
const DIRECTORY = 'scripts/stability-gates/windows-parent-affinity-v1';
const WORKFLOW = '.github/workflows/windows-job-metrics.yml';
const HEADER_NAMES = ['windows.h', 'psapi.h', 'inttypes.h', 'stdio.h', 'wchar.h', 'stdint.h',
  'stdbool.h', 'stddef.h', 'limits.h', 'string.h', 'stdarg.h', 'assert.h'];
const LIBRARY_NAMES = ['kernel32.lib', 'libcmt.lib', 'libvcruntime.lib', 'libucrt.lib', 'oldnames.lib'];
const projection = pin => ({ path: pin.path, bytes: pin.bytes, sha256: pin.sha256 });
export function producerDocument(raw) {
  assert(Buffer.isBuffer(raw) && raw.length > 0 && raw.length <= MAX_DOCUMENT);
  const text = new TextDecoder('utf-8', { fatal: true }).decode(raw), value = JSON.parse(text);
  const compact = `${JSON.stringify(value)}\n`, pretty = JSON.stringify(value, null, 2);
  assert([canonical(value), compact, `${pretty}\n`, `${pretty.replaceAll('\n', '\r\n')}\n`].includes(text),
    'complete canonical/producer/PowerShell JSON; no duplicate keys or mixed spelling');
  return value;
}
function absolute(path) {
  assert(typeof path === 'string' && path.length < 4096 && /^[A-Za-z]:\\/u.test(path));
  assert.equal(win32.normalize(path), path); assert(!/[\x00-\x1f]/u.test(path));
  assert(!path.slice(2).includes(':')); return path;
}
function pin(value) {
  exact(value, ['path', 'bytes', 'sha256'], 'original material pin'); absolute(value.path);
  assert(Number.isSafeInteger(value.bytes) && value.bytes > 0 && value.bytes <= 512 * 1024 * 1024);
  assert(typeof value.sha256 === 'string' && /^[a-f0-9]{64}$/u.test(value.sha256)); return value;
}
function observed(row) {
  exact(row, ['path', 'bytes', 'sha256', 'mode', 'prefixHex', 'identity'], 'original observed material'); pin(projection(row));
  assert(typeof row.mode === 'string' && /^[0-7]{1,3}$/u.test(row.mode) && parseInt(row.mode, 8).toString(8) === row.mode);
  assert(typeof row.prefixHex === 'string' && new RegExp(`^[a-f0-9]{${Math.min(row.bytes, 128) * 2}}$`, 'u').test(row.prefixHex));
  exact(row.identity, ['dev', 'ino', 'size', 'mode', 'nlink', 'mtimeNs', 'ctimeNs'], 'original observation identity');
  for (const value of Object.values(row.identity)) assert(typeof value === 'string' && /^(0|[1-9][0-9]*)$/u.test(value) && value.length <= 32);
  assert.equal(row.identity.size, String(row.bytes)); assert(BigInt(row.identity.nlink) > 0n, 'observed positive link count');
  assert.equal(BigInt(row.identity.mode) & 0o170000n, 0o100000n);
  assert.equal((BigInt(row.identity.mode) & 0o777n).toString(8), row.mode);
}
function originalPin(path, raw) { return { path, bytes: raw.length, sha256: sha256(raw) }; }
function sdk(calibration, operator) {
  const env = operator.environment;
  const expected = { includeSearch: env.INCLUDE.split(';').filter(Boolean), libSearch: env.LIB.split(';').filter(Boolean),
    sdkDirectory: env.WindowsSdkDir, sdkVersion: env.WindowsSDKVersion,
    vcToolsDirectory: env.VCToolsInstallDir, vcToolsVersion: env.VCToolsVersion };
  assert.deepEqual(Object.fromEntries(Object.keys(expected).map(key => [key, calibration[key]])), expected);
  for (const [rows, names, directories] of [[calibration.selectedDirectHeaders, HEADER_NAMES, expected.includeSearch],
    [calibration.selectedStaticLibraries, LIBRARY_NAMES, expected.libSearch]]) {
    assert(Array.isArray(rows) && rows.length === names.length);
    rows.forEach((row, index) => { pin(row); assert.equal(win32.basename(row.path).toLowerCase(), names[index]);
      assert(directories.includes(win32.dirname(row.path))); });
  }
  return expected;
}
function materials(value, expected, operator, calibration, rawBuild, buildPath, originals) {
  exact(value, ['expected', 'declaredDependencies', 'calibratedTools', 'sdkExpected', 'sdkDeclared', 'runtimeNode',
    'runtimeNodeRealpath', 'selectedNodeRealpath', 'buildRecord', 'buildRawSha256', 'observed'], 'retained native material context');
  const known = { ...expected };
  const header = value.expected.supplementalDirectHeader; pin(header);
  assert.equal(win32.basename(header.path).toLowerCase(), 'tlhelp32.h');
  assert(operator.environment.INCLUDE.split(';').filter(Boolean).includes(win32.dirname(header.path)));
  known.dependencies = [...calibration.selectedDirectHeaders, ...calibration.selectedStaticLibraries, header];
  known.supplementalDirectHeader = header;
  assert.deepEqual(value.expected, known, 'recomputed source/tool/helper/calibration context');
  assert(Array.isArray(value.observed) && value.observed.length <= 64);
  value.observed.forEach(observed);
  const sequence = [...known.sourceBefore, known.workflow, known.toolchain.compiler, known.toolchain.linker,
    known.toolchain.node, ...known.dependencies, known.calibratedToolchain, header, known.helper,
    value.runtimeNode, originalPin(buildPath, rawBuild)];
  assert.deepEqual(value.observed.map(projection), sequence, 'complete original material observation sequence');
  const byPath = new Map();
  for (const row of value.observed) {
    if (byPath.has(row.path)) assert.deepEqual(row, byPath.get(row.path), 'duplicate observed path has one identity');
    byPath.set(row.path, row);
  }
  for (const [path, raw] of originals)
    assert.equal(byPath.get(path)?.prefixHex, raw.subarray(0, 128).toString('hex'), 'observed prefix matches actual retained/source bytes');
  const actual = byPath.get(known.helper.path);
  validateBinary(actual, 'x86_64-pc-windows-msvc');
  pin(value.runtimeNode); absolute(value.runtimeNodeRealpath); absolute(value.selectedNodeRealpath);
  assert.equal(value.runtimeNodeRealpath, value.selectedNodeRealpath);
  assert.equal(value.runtimeNode.bytes, known.toolchain.node.bytes); assert.equal(value.runtimeNode.sha256, known.toolchain.node.sha256);
  assert.equal(value.runtimeNodeRealpath.toLowerCase(), value.runtimeNode.path.toLowerCase());
  assert.equal(value.selectedNodeRealpath.toLowerCase(), operator.tools.node.toLowerCase());
  assert.deepEqual(value.calibratedTools, known.toolchain);
  assert.deepEqual(value.declaredDependencies, known.dependencies.slice(0, -1));
  assert.deepEqual(value.sdkExpected, sdk(calibration, operator));
  assert.deepEqual(value.sdkDeclared, value.sdkExpected);
  assert.equal(value.buildRawSha256, sha256(rawBuild));
  assert.deepEqual(value.buildRecord, originalPin(buildPath, rawBuild));
  return value;
}
export function bindWindowsHostObservations({ source, operator, tools, read, readOriginal, companions }) {
  const root = absolute(operator.root), calibrationDir = absolute(operator.calibrationDirectory);
  const evidence = win32.join(win32.dirname(calibrationDir), 'windows-parent-affinity-observation');
  const helperPath = win32.join(evidence, 'build', 'windows-parent-affinity-v1.exe');
  const buildPath = win32.join(evidence, 'affinity-build.json');
  const helperBytes = read('windows-parent-affinity-observation/build/windows-parent-affinity-v1.exe', 64 * 1024 * 1024);
  assert(helperBytes.length >= 64);
  const pe = helperBytes.readUInt32LE(60);
  assert(pe >= 64 && pe <= 2 * 1024 * 1024 - 8 && pe + 8 <= helperBytes.length, 'bounded original helper PE header');
  assert(helperBytes.subarray(pe, pe + 4).equals(Buffer.from([80, 69, 0, 0])) &&
    helperBytes.readUInt16LE(pe + 4) === 0x8664, 'original helper PE AMD64 identity');
  const buildRaw = read('windows-parent-affinity-observation/affinity-build.json', MAX_DOCUMENT);
  const calibrationRaw = read('windows-job-metrics-calibration/toolchain-before-build.json', MAX_DOCUMENT);
  const build = producerDocument(buildRaw), calibration = producerDocument(calibrationRaw);
  exact(calibration, ['compiler', 'linker', 'node', 'compilerFileVersion', 'linkerFileVersion', 'includeSearch', 'libSearch',
    'sdkDirectory', 'sdkVersion', 'vcToolsDirectory', 'vcToolsVersion', 'selectedDirectHeaders', 'selectedStaticLibraries', 'selectionScope'], 'original calibration');
  const sdkExpected = sdk(calibration, operator);
  const originals = new Map([[helperPath, helperBytes], [buildPath, buildRaw],
    [win32.join(calibrationDir, 'toolchain-before-build.json'), calibrationRaw]]);
  const toolchain = {};
  for (const [role, key] of [['compiler', 'cc'], ['linker', 'linker'], ['node', 'node']]) {
    toolchain[role] = projection(tools.buildBefore[key]); pin(toolchain[role]);
    assert.equal(toolchain[role].path, operator.tools[key]);
    assert.deepEqual(projection(tools.buildAfter[key]), toolchain[role]);
    assert.deepEqual(calibration[role], toolchain[role]);
  }
  const sourcePins = FILES.map(name => {
    const logical = `${DIRECTORY}/${name}`, raw = readOriginal(logical, 2 * 1024 * 1024);
    const path = win32.join(root, ...logical.split('/')); originals.set(path, raw);
    return originalPin(path, raw);
  });
  const workflowPath = win32.join(root, ...WORKFLOW.split('/'));
  const workflowBytes = readOriginal(WORKFLOW, 65536); originals.set(workflowPath, workflowBytes);
  const expected = { checkout: { head: source.commit, tree: source.tree }, sourceBefore: sourcePins,
    workflow: originalPin(workflowPath, workflowBytes), toolchain,
    calibratedToolchain: originalPin(win32.join(calibrationDir, 'toolchain-before-build.json'), calibrationRaw),
    helper: originalPin(helperPath, helperBytes) };
  const decoded = [], contexts = []; let parentCreated = null;
  for (const [index, phase] of ['before', 'after'].entries()) {
    const prefix = `m418-hosted-observations/capture/numeric-host-${phase}-affinity`;
    const json = name => producerDocument(read(name, MAX_DOCUMENT));
    assert(read(`${prefix}-build.raw.json`, MAX_DOCUMENT).equals(buildRaw), 'same original native build record');
    const before = json(`${prefix}-materials-before.json`), after = json(`${prefix}-materials-after.json`);
    assert.deepEqual(after, before, 'all actual material claims stable around native observation');
    materials(before, expected, operator, calibration, buildRaw, buildPath, originals);
    if (index) assert.deepEqual(before, contexts[0], 'all material claims stable across both phases');
    validateAffinityBuild(build, { ...before, expected: before.expected, sdkExpected,
      declaredDependencies: [...calibration.selectedDirectHeaders, ...calibration.selectedStaticLibraries],
      calibratedTools: toolchain, buildRawSha256: sha256(buildRaw) });
    const pid = companions[index].collectorNodePid;
    const command = json(`${prefix}.command.json`);
    assert.deepEqual(command, { executable: helperPath, args: ['--parent-pid', String(pid)], cwd: root, timeoutMs: 10000 });
    const result = json(`${prefix}.result.json`);
    exact(result, ['exitCode', 'signal', 'error', 'elapsedMs', 'streams', ...Object.keys(COMPANION_FLAGS)], 'original native process result');
    for (const [key, value] of Object.entries(COMPANION_FLAGS)) assert.equal(result[key], value);
    exact(result.streams, ['stdout', 'stderr'], 'both original native streams');
    const stdout = read(`${prefix}.stdout.raw`, RECORD_BYTES), stderr = read(`${prefix}.stderr.raw`, 0);
    for (const [name, raw] of [['stdout', stdout], ['stderr', stderr]])
      assert.deepEqual(result.streams[name], { bytes: raw.length, sha256: sha256(raw) });
    const fresh = decodeObservedParentAffinity({ stdout, stderr, exitCode: result.exitCode, signal: result.signal,
      error: result.error, elapsedMs: result.elapsedMs }, { parentPid: pid, parentCreated });
    parentCreated = fresh.parent.created;
    const claimed = companions[index].native;
    exact(claimed, ['decoded', 'buildRecord', 'capture'], 'retained native companion');
    assert.deepEqual(claimed.decoded, fresh, 'retained decoded projection equals ORIGINAL re-decoded C record');
    assert.deepEqual(claimed.buildRecord, before.buildRecord);
    assert.deepEqual(claimed.capture, { command, stdoutSha256: sha256(stdout), stderrSha256: sha256(stderr) });
    decoded.push(fresh); contexts.push(before);
  }
  return { decoded, parent: decoded[0].parent, toolInputs: { helper: expected.helper, sourceFiles: sourcePins,
    workflow: expected.workflow, calibration: expected.calibratedToolchain },
    materialScope: 'Original material claims and source/helper bytes bound; SDK/tool payload bytes are not retained here' };
}
