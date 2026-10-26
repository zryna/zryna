// Windows-only private controls; observations are not baseline/source/host certificates.
import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, isAbsolute, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { fingerprint, readBounded, createJson } from './files.mjs';
import { verify } from './verify.mjs';
assert.equal(process.platform, 'win32', 'actual Windows required; no Linux cfg0 qualification');
assert.equal(process.arch, 'x64'); assert.equal(process.versions.node, '22.22.1', 'pinned calibration Node');
const args = process.argv.slice(2);
assert(args.length === 6 && args[0] === '--helper' && args[2] === '--fixture' && args[4] === '--out');
const [helper, fixture, output] = [args[1], args[3], args[5]];
for (const path of [helper, fixture, output]) assert(isAbsolute(path), 'explicit absolute trusted local paths');
const here = dirname(fileURLToPath(import.meta.url));
const expected = JSON.parse(readBounded(join(here, 'expected.json'), 64 * 1024));
const pins = JSON.parse(readBounded(join(here, 'source-pins.json'), 64 * 1024));
assert.equal(expected.format, 'zryna.windows-job-calibration-expected.v1'); assert.equal(expected.cases.length, 8);
assert.equal(pins.format, 'zryna.windows-job-calibration-source-pins.v1');
assert(Array.isArray(pins.files) && pins.files.length === 29 && new Set(pins.files.map(pin => pin.path)).size === 29);
function sources() {
  return pins.files.map(pin => {
    assert(pin.path.split('/').every(part => /^[A-Za-z0-9_.-]+$/u.test(part) && part !== '.' && part !== '..'), 'fixed relative source path');
    const identity = fingerprint(join(dirname(here), pin.path), 2 * 1024 * 1024);
    assert.equal(identity.bytes, pin.bytes); assert.equal(identity.sha256, pin.sha256, `reviewed source pin: ${pin.path}`);
    return { path: pin.path, ...identity };
  });
}
const before = { node: process.version, sourcePins: fingerprint(join(here, 'source-pins.json'), 64 * 1024), helper: fingerprint(helper, 32 * 1024 * 1024), fixture: fingerprint(fixture, 32 * 1024 * 1024), sources: sources() };
mkdirSync(output, { recursive: false, mode: 0o700 });
createJson(join(output, 'inputs.json'), { format: 'zryna.windows-job-calibration-inputs.v1', helper, fixture, before,
  sourceAuthenticated: false, compilerBuildAuthenticated: false, baselineApproved: false, thresholds: null, closureStatus: 'blocked' });
const results = [];
for (const [index, item] of expected.cases.entries()) {
  const directory = join(output, item.name);
  const argv = [fixture, 'supervise', helper, directory, item.name];
  const result = spawnSync(argv[0], argv.slice(1), { shell: false, windowsHide: true, encoding: null,
    timeout: item.helperTimeoutMs + 20000, maxBuffer: 64 * 1024 });
  // Retain actual process outcome/streams BEFORE any sample/owner assertion; no retries.
  const stdout = result.stdout ?? Buffer.alloc(0), stderr = result.stderr ?? Buffer.alloc(0);
  writeFileSync(join(output, `${item.name}.owner.stdout.raw`), stdout, { flag: 'wx', mode: 0o600 });
  writeFileSync(join(output, `${item.name}.owner.stderr.raw`), stderr, { flag: 'wx', mode: 0o600 });
  const outcome = { argv, status: result.status, signal: result.signal, error: result.error?.message ?? null,
    stdoutBytes: stdout.length, stderrBytes: stderr.length };
  createJson(join(output, `${item.name}.process.json`), outcome);
  try { const checked = verify(directory, item, index, result); results.push({ name: item.name, status: 'passed', ...checked }); }
  catch (error) { results.push({ name: item.name, status: 'failed', reason: String(error), outcome }); }
  // Material drift is a failed packet, not an opportunity to reseal/retry a case.
  try {
    assert.deepEqual(fingerprint(helper, 32 * 1024 * 1024), before.helper);
    assert.deepEqual(fingerprint(fixture, 32 * 1024 * 1024), before.fixture); assert.deepEqual(sources(), before.sources);
    assert.deepEqual(fingerprint(join(here, 'source-pins.json'), 64 * 1024), before.sourcePins);
  } catch (error) { results.push({ name: `${item.name}-material-drift`, status: 'failed', reason: String(error) }); break; }
}
const passed = results.length === 8 && results.every(row => row.status === 'passed');
createJson(join(output, 'calibration.json'), { format: 'zryna.windows-job-calibration-observation.v1',
  status: passed ? 'controls-passed' : 'controls-failed', results, sourceAuthenticated: false, proofQualified: false,
  baselineApproved: false, thresholds: null, closureStatus: 'blocked', scope: 'fixed trusted C controls only; not actual workload performance' });
console.log(JSON.stringify({ status: passed ? 'controls-passed' : 'controls-failed', cases: results.length,
  passed: results.filter(row => row.status === 'passed').length, output }));
process.exitCode = passed ? 0 : 1;
