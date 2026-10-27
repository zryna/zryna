// Fixed acquisition controller. IO seam is for pure controls; operator inputs cannot choose a command.
import assert from 'node:assert/strict';
import { posix } from 'node:path';
import { canonical, sha256 } from './input.mjs';
import { DOWNLOAD_URL, PACKAGE, providerRoot } from './cli-performance-linux-provider.mjs';
export const ACQUISITION_PARENT_MS = 31000;
const RAW_LIMIT = 16384;
export function acquisitionInvocation(root) {
  providerRoot(root);
  return { file: '/usr/bin/curl', timeoutMs: ACQUISITION_PARENT_MS,
    args: ['--disable', '--fail', '--silent', '--show-error', '--proto', '=https', '--proto-redir', '=https',
      '--max-redirs', '0', '--max-time', '30', '--connect-timeout', '10', '--max-filesize', '131072',
      '--output', posix.join(root, 'time.deb'), '--write-out', '%{http_code}', DOWNLOAD_URL] };
}
export function validatePackageDownload(bytes) {
  assert(Buffer.isBuffer(bytes) && bytes.length <= 128 * 1024, 'bounded package download');
  assert.equal(bytes.length, PACKAGE.bytes, 'exact fixed package size before extraction');
  assert.equal(sha256(bytes), PACKAGE.sha256, 'exact fixed package hash before extraction');
}
function deadline(startedNs, endedNs) {
  assert(typeof startedNs === 'bigint' && typeof endedNs === 'bigint' && endedNs >= startedNs,
    'monotonic acquisition clock');
  assert(endedNs - startedNs <= BigInt(ACQUISITION_PARENT_MS) * 1000000n, 'total acquisition deadline');
}
export function checkAcquisitionOutcome({ startedNs, endedNs, result }) {
  deadline(startedNs, endedNs);
  assert.equal(result.error, null, 'no acquisition timeout/spawn/overflow failure');
  assert.equal(result.signal, null); assert.equal(result.status, 0, 'curl TLS/HTTP acquisition success');
  assert(Buffer.isBuffer(result.stdout) && Buffer.isBuffer(result.stderr));
  assert(result.stdout.length + result.stderr.length <= RAW_LIMIT, 'bounded curl reporting');
  assert(result.stdout.equals(Buffer.from('200')), 'fixed official URL must return unredirected 200');
  assert.equal(result.stderr.length, 0, 'no unclassified acquisition warnings');
}
export function collectAcquisition({ root, io }) {
  const call = acquisitionInvocation(root), startedNs = io.nowNs();
  const record = { format: 'zryna.gnu-time-acquisition.v1', call,
    status: 'invalid-or-incomplete', partialPackageRetained: true,
    sourceAuthenticated: false, performanceQualified: false, baselineAuthority: null, thresholds: null };
  try {
    let result;
    try { result = io.execute(call); }
    catch (error) { result = { status: null, signal: null, error: String(error.message).slice(0, 4096),
      stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) }; }
    const endedNs = io.nowNs();
    const raw = { status: result.status, signal: result.signal, error: result.error,
      startedNs: String(startedNs), endedNs: String(endedNs), stdoutBytes: result.stdout.length,
      stderrBytes: result.stderr.length, stdoutTruncated: result.stdout.length > RAW_LIMIT,
      stderrTruncated: result.stderr.length > RAW_LIMIT };
    // The original private time.deb is never discarded on failure. Outer reporting keeps bounded prefixes.
    try {
      io.retain('acquisition.stdout', result.stdout.subarray(0, RAW_LIMIT));
      io.retain('acquisition.stderr', result.stderr.subarray(0, RAW_LIMIT));
      io.retain('acquisition-process.json', Buffer.from(canonical(raw)));
    } finally { io.check(); } // including timeout/overflow/fault paths
    checkAcquisitionOutcome({ startedNs, endedNs, result });
    const bytes = io.readPackage();
    deadline(startedNs, io.nowNs()); // successful EOF and bounded file read share the original deadline
    validatePackageDownload(bytes);
    deadline(startedNs, io.nowNs()); // include exact package hashing in the same completion budget
    record.status = 'exact-package-hash-verified'; record.packageBytes = bytes.length;
    record.packageSha256 = sha256(bytes);
    return bytes;
  } catch (error) { record.failure = String(error.message).slice(0, 4096); throw error; }
  finally { io.retain('acquisition.json', Buffer.from(canonical(record))); }
}
