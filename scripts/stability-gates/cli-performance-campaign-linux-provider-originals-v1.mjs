// Pure retained-byte consistency. No platform, source, host or live owner capability is issued.
import assert from 'node:assert/strict';
import { TextDecoder } from 'node:util';
import { exact, parse, sha256 } from './input.mjs';
import { openPerformanceArchive, listPerformanceArchive, readPerformanceArchiveEntry, closePerformanceArchive }
  from './cli-performance-archive.mjs';
import { GNU_TIME, PACKAGE, PROVIDER_SHA256, providerRoot, timeExecutable, validateProviderBinding,
  validateProviderRecord, validateRuntimeReceipt } from './cli-performance-linux-provider.mjs';
import { acquisitionInvocation, checkAcquisitionOutcome, validatePackageDownload }
  from './cli-performance-linux-acquire-provider.mjs';
import { parseLoaderListing } from './cli-performance-linux-provision-provider.mjs';
import { u64 } from './windows-job-metrics-v1/calibration/protocol.mjs';
const decoder = new TextDecoder('utf-8', { fatal: true });
function original(bytes, maximum) {
  assert(Buffer.isBuffer(bytes) && bytes.length > 0 && bytes.length <= maximum, 'bounded original provider record');
  const copy = Buffer.alloc(bytes.length); copy.set(bytes); return copy;
}
export function reconcileLinuxProviderAcquisition(root, receipt, originals) {
  assert.equal(arguments.length, 3); providerRoot(root);
  exact(originals, ['record', 'process', 'stdout', 'stderr'], 'original acquisition records');
  const bytes = original(originals.record, 16384), record = parse(bytes);
  assert.equal(bytes.length, receipt.acquisition.receipt.bytes);
  assert.equal(sha256(bytes), receipt.acquisition.receipt.sha256);
  assert.deepEqual(record, { format: 'zryna.gnu-time-acquisition.v1', call: acquisitionInvocation(root),
    status: 'exact-package-hash-verified', partialPackageRetained: true, packageBytes: PACKAGE.bytes,
    packageSha256: PACKAGE.sha256, sourceAuthenticated: false, performanceQualified: false,
    baselineAuthority: null, thresholds: null });
  const raw = parse(original(originals.process, 16384));
  exact(raw, ['status', 'signal', 'error', 'startedNs', 'endedNs', 'stdoutBytes', 'stderrBytes',
    'stdoutTruncated', 'stderrTruncated'], 'original acquisition process');
  for (const name of ['startedNs', 'endedNs']) { assert.equal(typeof raw[name], 'string'); u64(raw[name]); }
  const stdout = originals.stdout, stderr = originals.stderr;
  assert(Buffer.isBuffer(stdout) && Buffer.isBuffer(stderr) && stdout.length + stderr.length <= 16384);
  assert.equal(raw.stdoutBytes, stdout.length); assert.equal(raw.stderrBytes, stderr.length);
  assert.equal(raw.stdoutTruncated, false); assert.equal(raw.stderrTruncated, false);
  checkAcquisitionOutcome({ startedNs: BigInt(raw.startedNs), endedNs: BigInt(raw.endedNs),
    result: { status: raw.status, signal: raw.signal, error: raw.error, stdout, stderr } });
  return { acquisitionRecordSha256: sha256(bytes), acquisitionProcessSha256: sha256(originals.process) };
}
export function reconcileLinuxProviderLoaderListings(receipt, listings) {
  assert.equal(arguments.length, 2);
  exact(listings, ['dpkgDeb', 'dpkgQuery', 'time'], 'three retained loader listings');
  const providers = receipt.runtime.sharedProviders;
  for (const key of ['loader', 'libc'])
    assert.deepEqual(providers.find(pin => pin.path === receipt.runtime[key].path), receipt.runtime[key]);
  const used = new Set(), rows = [];
  for (const [tool, bytes] of Object.entries(listings)) {
    const raw = original(bytes, 16384), aliases = parseLoaderListing(decoder.decode(raw));
    const dependencies = aliases.map(alias => {
      const normalized = alias === '/lib64/ld-linux-x86-64.so.2' ? receipt.runtime.loader.path
        : alias.replace(/^\/lib\//u, '/usr/lib/');
      const matches = providers.filter(pin => pin.path === normalized || pin.path.startsWith(`${normalized}.`));
      assert.equal(matches.length, 1, 'unique declared canonical/versioned dependency projection');
      used.add(matches[0].path); return { observedAlias: alias, declaredPath: matches[0].path };
    });
    assert(dependencies.some(row => row.declaredPath === receipt.runtime.loader.path));
    assert(dependencies.some(row => row.declaredPath === receipt.runtime.libc.path));
    rows.push({ tool, bytes: raw.length, sha256: sha256(raw), dependencies });
  }
  assert.deepEqual([...used].sort(), providers.map(pin => pin.path), 'all declared shared providers observed');
  return rows; // Name projection only: host symlinks and host library bytes are not in these ZIPs.
}
export function replayNumericLinuxProviderOriginals(input, bytes) {
  assert.equal(arguments.length, 2, 'fixed pure original replay; never a live provider key');
  exact(input, ['artifact', 'capture', 'binding', 'admission', 'receipt'], 'original provider inputs');
  assert(Buffer.isBuffer(bytes) && bytes.length >= 22 && bytes.length <= 1024 * 1024);
  const capture = parse(original(input.capture, 262144));
  assert.equal(capture.host.platform, 'linux-x86_64'); validateProviderBinding(capture);
  const root = providerRoot(capture.providerRoot), prefix = root.slice('/tmp/'.length) + '/';
  const bindingBytes = original(input.binding, 16384), admission = original(input.admission, 16384);
  const primaryReceipt = original(input.receipt, 65536);
  validateProviderRecord(admission);
  assert.deepEqual(parse(bindingBytes), { providerRoot: root, providerRecordSha256: PROVIDER_SHA256,
    providerReceiptSha256: capture.providerReceiptSha256, executable: timeExecutable(root),
    authority: 'binding-only-no-source-or-baseline-authority' });
  const image = Buffer.alloc(bytes.length); image.set(bytes);
  const key = openPerformanceArchive(image, input.artifact, 'linux-provider');
  try {
    const entries = listPerformanceArchive(key);
    assert(entries.every(row => row.path.startsWith(prefix)), 'one exact selected provider root only');
    const load = (name, maximum) => {
      const entry = entries.find(row => row.path === prefix + name);
      assert(entry && entry.bytes <= maximum, `bounded original provider member ${name}`);
      return readPerformanceArchiveEntry(key, prefix + name);
    };
    const receiptBytes = load('runtime-receipt.json', 65536);
    assert(receiptBytes.equals(primaryReceipt), 'original receipt equals primary round retained receipt');
    assert.equal(sha256(receiptBytes), capture.providerReceiptSha256);
    const receipt = validateRuntimeReceipt(receiptBytes, root);
    const acquisition = reconcileLinuxProviderAcquisition(root, receipt, { record: load('acquisition.json', 16384),
      process: load('acquisition-process.json', 16384), stdout: load('acquisition.stdout', 16384),
      stderr: load('acquisition.stderr', 16384) });
    validatePackageDownload(load('time.deb', 128 * 1024));
    const tool = load('extracted/usr/bin/time', GNU_TIME.bytes);
    assert.equal(tool.length, GNU_TIME.bytes); assert.equal(sha256(tool), GNU_TIME.sha256);
    assert.equal(receipt.package.identity.uid, receipt.tool.file.identity.uid);
    assert.equal(receipt.package.identity.uid, receipt.acquisition.receipt.identity.uid);
    const loaderListings = reconcileLinuxProviderLoaderListings(receipt, {
      dpkgDeb: load('dpkg-deb.loader.stdout.raw', 16384), dpkgQuery: load('dpkg-query.loader.stdout.raw', 16384),
      time: load('time.loader.stdout.raw', 16384) });
    return { format: 'zryna.cli-performance-linux-provider-originals.v1', host: 'linux-x86_64',
      artifact: structuredClone(input.artifact), providerRoot: root, providerRecordSha256: PROVIDER_SHA256,
      providerReceiptSha256: sha256(receiptBytes), providerOriginalsVerified: true,
      originalBindings: { capture: sha256(input.capture), binding: sha256(bindingBytes), admission: sha256(admission),
        receipt: sha256(receiptBytes), ...acquisition }, package: { ...PACKAGE }, executable: timeExecutable(root),
      observedRuntime: receipt, loaderListings, retainedEntries: entries,
      sourceAuthenticated: false, rawAuthenticated: false, hostAuthenticated: false, proofQualified: false,
      performanceQualified: false, baselineAdmitted: false, hostAdmitted: false, baselineAuthority: null,
      thresholdAuthority: null, thresholds: null, closureStatus: 'blocked',
      pending: ['Host os-release/curl/dpkg/loader/libc/shared-library bytes and symlink resolution are not independently retained',
        'Actual acquisition/runtime execution, provider/tool provenance, host/class/method and numeric table/anchor admission'] };
  } finally { closePerformanceArchive(key); }
}
