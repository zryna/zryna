// Fixed Windows live qualification only; no workload, command selector or host admission.
import assert from 'node:assert/strict';
import { lstatSync, readFileSync, realpathSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execute } from '../process.mjs';
import { pinFile } from '../release-build-input.mjs';
import { canonical, exact } from '../input.mjs';
import { decodeObservedParentAffinity, RECORD_BYTES } from './decode.mjs';
assert.equal(process.argv.length, 3, 'only one fixed evidence directory argument');
assert.equal(process.platform, 'win32'); assert.equal(process.arch, 'x64'); assert.equal(process.version, 'v22.22.1');
const output = realpathSync(resolve(process.argv[2]));
assert(lstatSync(output).isDirectory() && !lstatSync(output).isSymbolicLink());
const here = dirname(fileURLToPath(import.meta.url)), root = resolve(here, '../../..');
const helper = join(output, 'build/windows-parent-affinity-v1.exe');
const build = JSON.parse(readFileSync(join(output, 'affinity-build.json'), 'utf8'));
const observedPin = path => { const value = pinFile(path); return { path: value.path, bytes: value.bytes, sha256: value.sha256 }; };
const save = (name, value) => writeFileSync(join(output, name), Buffer.isBuffer(value) ? value : canonical(value), { flag: 'wx', mode: 0o600 });
assert.equal(build.format, 'zryna.windows-parent-affinity-build.v1');
for (const key of ['sourceAuthenticated', 'proofQualified', 'performanceQualified', 'hostAdmitted']) assert.equal(build[key], false);
assert.equal(build.closureStatus, 'blocked'); assert.equal(build.thresholds, null);
assert.match(build.checkout.head, /^[a-f0-9]{40}$/u); assert.match(build.checkout.tree, /^[a-f0-9]{40}$/u);
assert.equal(build.checkout.head, process.env.EXPECTED_SOURCE_REVISION);
const sourceNames = ['probe.c', 'build-msvc.cmd', 'decode.mjs', 'decode.test.mjs', 'hosted.ps1', 'live-controls.mjs'];
assert.deepEqual(build.sourceBefore.map(row => row.path.toLowerCase()), sourceNames.map(name => join(here, name).toLowerCase()));
assert.equal(build.helper.path.toLowerCase(), helper.toLowerCase());
assert.equal(build.workflow.path.toLowerCase(), join(root, '.github/workflows/windows-job-metrics.yml').toLowerCase());
assert.equal(build.toolchain.node.path.toLowerCase(), process.execPath.toLowerCase());
const bindings = [...build.sourceBefore, build.workflow, ...Object.values(build.toolchain), ...build.dependencies,
  build.calibratedToolchain, build.helper];
const verify = name => {
  const actual = bindings.map(row => observedPin(row.path));
  save(name + '.bindings.json', actual);
  for (const [index, row] of bindings.entries()) {
    exact(row, ['path', 'bytes', 'sha256'], 'selected actual source/tool');
    assert.deepEqual(actual[index], row, 'source, binary and calibrated tools unchanged');
  }
};
const flags = { sourceAuthenticated: false, proofQualified: false, performanceQualified: false, hostAdmitted: false,
  closureStatus: 'blocked', baselineAuthority: null, thresholdAuthority: null, thresholds: null };
const report = { format: 'zryna.windows-parent-affinity-live-controls.v1', status: 'incomplete', parentPid: process.pid,
  source: build.checkout, results: [], ...flags };
async function probe(name, parentPid) {
  verify(name + '-before');
  const args = ['--parent-pid', String(parentPid)], command = { executable: helper, args, cwd: root, timeoutMs: 10000 };
  save(name + '.command.json', command);
  const result = await execute(helper, args, { root, timeoutMs: 10000 });
  save(name + '.stdout.raw', result.stdout); save(name + '.stderr.raw', result.stderr);
  const { stdout, stderr, ...termination } = result;
  save(name + '.result.json', { ...termination, stdoutBytes: stdout.length, stderrBytes: stderr.length });
  verify(name + '-after'); // Raw results precede every decoding, success assertion and postflight check.
  return result;
}
function refused(result, first, wrongPid) {
  assert.equal(result.exitCode, 1); assert.equal(result.signal, null); assert.equal(result.error, null);
  assert(Buffer.isBuffer(result.stderr) && result.stderr.length === 0);
  assert(Number.isFinite(result.elapsedMs) && result.elapsedMs >= 0 && result.elapsedMs <= 10000);
  const raw = result.stdout;
  assert(Buffer.isBuffer(raw) && raw.length > 0 && raw.length <= RECORD_BYTES && raw.every(byte => byte < 128));
  const row = JSON.parse(raw.toString('ascii')); exact(row, Object.keys(first.originalRecord), 'closed original refusal');
  assert.deepEqual(Object.keys(row), Object.keys(first.originalRecord), 'same fixed native key order');
  assert(raw.equals(Buffer.from(JSON.stringify(row) + '\n', 'ascii')), 'complete fixed ASCII refusal wire');
  assert.equal(row.format, 'zryna.windows-parent-affinity.v1'); assert.equal(row.status, 'unavailable');
  assert.equal(row.stage, 2); assert.equal(row.error, 13); // Toolhelp self-entry disagrees: ERROR_INVALID_DATA.
  assert.equal(row.parentPid, wrongPid); assert(Number.isSafeInteger(row.childPid) && row.childPid > 0 && row.childPid <= 0xffffffff);
  assert.notEqual(row.childPid, process.pid);
  assert(typeof row.childCreated === 'string' && /^[1-9][0-9]{0,19}$/u.test(row.childCreated));
  assert(BigInt(row.childCreated) > BigInt(first.parent.created) && BigInt(row.childCreated) <= 0xffffffffffffffffn);
  assert.equal(row.snapshotClosed, true);
  for (const key of ['parentLiveBefore', 'parentLiveAfter', 'snapshotsMatch', 'parentHandleClosed']) assert.equal(row[key], false);
  for (const key of ['parentCreated', 'parentCreatedAfter', 'processMask', 'systemMask', 'afterProcessMask', 'afterSystemMask']) assert.equal(row[key], '0');
  for (const key of ['cleanupError', 'groupCount', 'groupId', 'activeProcessorCount', 'maskBitCount',
    'afterGroupCount', 'afterGroupId', 'afterActiveProcessorCount']) assert.equal(row[key], 0);
  assert(Number.isSafeInteger(row.elapsedMs) && row.elapsedMs >= 0 && row.elapsedMs < 10000);
  assert.throws(() => decodeObservedParentAffinity(result, { parentPid: process.pid, parentCreated: first.parent.created }));
  return row;
}
try {
  const before = decodeObservedParentAffinity(await probe('observed-before', process.pid), { parentPid: process.pid, parentCreated: null });
  save('observed-before.projection.json', before);
  report.results.push({ name: 'observed-before', status: 'observed-unqualified', observation: before });
  const after = decodeObservedParentAffinity(await probe('observed-after', process.pid), { parentPid: process.pid, parentCreated: before.parent.created });
  save('observed-after.projection.json', after);
  assert.deepEqual(after.parent, before.parent, 'same actual creating Node parent at both phases');
  report.results.push({ name: 'observed-after', status: 'observed-unqualified', observation: after });
  const wrongPid = process.pid === 1 ? 2 : 1;
  const wrong = refused(await probe('wrong-parent', wrongPid), before, wrongPid);
  save('wrong-parent.refusal.json', wrong);
  report.results.push({ name: 'wrong-parent', status: 'refused-before-parent-open', originalRecord: wrong });
  verify('final'); report.status = 'three-controls-observed-unqualified';
} catch (error) {
  report.failure = { name: error.name, message: String(error.message).slice(0, 4096) };
  throw error;
} finally {
  save('live-controls.json', report);
}
