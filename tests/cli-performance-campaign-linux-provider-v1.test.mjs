// Pure original-byte controls. No acquisition, ELF execution, host or campaign qualification.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { PACKAGE, GNU_TIME, PROVIDER_SHA256, timeExecutable } from '../scripts/stability-gates/cli-performance-linux-provider.mjs';
import { acquisitionInvocation } from '../scripts/stability-gates/cli-performance-linux-acquire-provider.mjs';
import { reconcileNumericCampaignLinuxProvider, inspectNumericCampaignLinuxProvider }
  from '../scripts/stability-gates/cli-performance-campaign-linux-provider-v1.mjs';
import { replayNumericLinuxProviderOriginals, reconcileLinuxProviderAcquisition, reconcileLinuxProviderLoaderListings }
  from '../scripts/stability-gates/cli-performance-campaign-linux-provider-originals-v1.mjs';
import { archiveFixture } from './cli-performance-archive-fixture.mjs';
const root = '/tmp/m418-linux-provider-AbC123', prefix = root.slice(5) + '/';
const encode = value => Buffer.from(canonical(value));
const admission = readFileSync(new URL('../scripts/stability-gates/provider-inputs/linux-gnu-time-debian-trixie-v2.json', import.meta.url));
function acquisition() {
  const record = encode({ format: 'zryna.gnu-time-acquisition.v1', call: acquisitionInvocation(root),
    status: 'exact-package-hash-verified', partialPackageRetained: true, packageBytes: PACKAGE.bytes,
    packageSha256: PACKAGE.sha256, sourceAuthenticated: false, performanceQualified: false,
    baselineAuthority: null, thresholds: null });
  return { record, process: encode({ startedNs: '1', endedNs: '31000000001', status: 0,
    signal: null, error: null, stdoutBytes: 3, stderrBytes: 0, stdoutTruncated: false, stderrTruncated: false }),
  stdout: Buffer.from('200'), stderr: Buffer.alloc(0) };
}
function file(path, pin = {}, mode = 0o100755) {
  const size = pin.bytes ?? 123;
  return { path, bytes: size, sha256: pin.sha256 ?? '1'.repeat(64),
    identity: { dev: 1, ino: 2, size, mode, nlink: 1, uid: 1000, gid: 1000, mtimeMs: 1.25, ctimeMs: 2.5 } };
}
function receipt(records = acquisition()) {
  const loader = file('/usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2'), libc = file('/usr/lib/x86_64-linux-gnu/libc.so.6');
  return { format: 'zryna.gnu-time-runtime-receipt.v1', providerRoot: root, providerRecordSha256: PROVIDER_SHA256,
    acquisition: { curl: file('/usr/bin/curl'), version: 'observed curl version\n',
      receipt: file(`${root}/acquisition.json`, { bytes: records.record.length, sha256: sha256(records.record) }, 0o100600) },
    host: { platform: 'linux', arch: 'x64', osRelease: file('/usr/lib/os-release', {}, 0o100644) },
    package: file(`${root}/time.deb`, PACKAGE, 0o100600), tool: { file: file(timeExecutable(root).path, GNU_TIME), version: 'time (GNU Time) UNKNOWN\n' },
    runtime: { dpkgDeb: file('/usr/bin/dpkg-deb'), dpkgQuery: file('/usr/bin/dpkg-query'), loader, libc,
      sharedProviders: [structuredClone(loader), structuredClone(libc)], versions: { dpkgDeb: 'observed\n', libc: 'observed\n', loader: 'observed\n' } },
    provenance: { packageAcquisition: 'fixed-url-exact-hash-this-host-no-fresh-gpg', hostPackages: 'observed packages\n',
      historicalAnchor: 'historical-only-a719cdffdae1f744c92b2d93f65103a1e4cfc3429d2fc55cdb85fe3ffbf8470a' },
    sourceAuthenticated: false, performanceQualified: false, baselineAuthority: null, thresholds: null };
}
function fixture(change = () => {}) {
  const records = acquisition(), observed = receipt(records), raw = encode(observed);
  const values = new Map([['runtime-receipt.json', raw], ['acquisition.json', records.record],
    ['acquisition-process.json', records.process], ['acquisition.stdout', records.stdout], ['acquisition.stderr', records.stderr],
    ['time.deb', Buffer.alloc(PACKAGE.bytes)]]);
  change(values);
  const bytes = archiveFixture([...values].map(([name, value]) => [prefix + name, value]));
  return { bytes, input: { artifact: { bytes: bytes.length, sha256: sha256(bytes) }, admission,
    capture: encode({ host: { platform: 'linux-x86_64' }, providerRoot: root, providerRecordSha256: PROVIDER_SHA256,
      providerReceiptSha256: sha256(raw), tools: { time: timeExecutable(root) } }), receipt: raw,
    binding: encode({ providerRoot: root, providerRecordSha256: PROVIDER_SHA256, providerReceiptSha256: sha256(raw),
      executable: timeExecutable(root), authority: 'binding-only-no-source-or-baseline-authority' }) } };
}
test('original acquisition exact deadline is accepted, first extra nanosecond and changed URL fail', () => {
  const originals = acquisition(), r = receipt(originals);
  assert.equal(reconcileLinuxProviderAcquisition(root, r, originals).acquisitionRecordSha256, sha256(originals.record));
  for (const mutate of [p => { p.endedNs = '31000000002'; }, p => { p.endedNs = '0'; },
    p => { p.stdoutTruncated = true; }, p => { p.stdoutBytes = 4; }, p => { p.error = 'timeout'; }]) {
    const changed = JSON.parse(originals.process); mutate(changed);
    assert.throws(() => reconcileLinuxProviderAcquisition(root, r, { ...originals, process: encode(changed) }));
  }
  const record = JSON.parse(originals.record); record.call.args[record.call.args.length - 1] = 'https://example.test/time.deb';
  const changed = { ...originals, record: encode(record) };
  assert.throws(() => reconcileLinuxProviderAcquisition(root, receipt(changed), changed));
  assert.equal(reconcileLinuxProviderAcquisition(root, r, originals).acquisitionRecordSha256, sha256(originals.record));
});
test('HTTP, stderr, malformed native clocks and source authority cannot replace original acquisition success', () => {
  const originals = acquisition();
  for (const changed of [{ ...originals, stdout: Buffer.from('302') }, { ...originals, stderr: Buffer.from('warning') },
    { ...originals, process: encode({ ...JSON.parse(originals.process), startedNs: '-1' }) }])
    assert.throws(() => reconcileLinuxProviderAcquisition(root, receipt(originals), changed));
  const record = JSON.parse(originals.record); record.sourceAuthenticated = true;
  const changed = { ...originals, record: encode(record) };
  assert.throws(() => reconcileLinuxProviderAcquisition(root, receipt(changed), changed));
});
test('three loader observations cover the declared physical providers without claiming host alias bytes', () => {
  const r = receipt(), listing = Buffer.from('\tlibc.so.6 => /lib/x86_64-linux-gnu/libc.so.6 (0xabc)\n\t/lib64/ld-linux-x86-64.so.2 (0xdef)\n');
  const lists = { dpkgDeb: listing, dpkgQuery: listing, time: listing };
  const result = reconcileLinuxProviderLoaderListings(r, lists);
  assert.equal(result.length, 3); assert.equal(result[0].dependencies[0].declaredPath, r.runtime.libc.path);
  for (const changed of [{ ...lists, time: Buffer.from('libc.so.6 => not found\n') },
    { ...lists, dpkgDeb: Buffer.from('libc.so.6 => /tmp/libc.so.6 (0xabc)\n') }])
    assert.throws(() => reconcileLinuxProviderLoaderListings(r, changed));
  const changed = structuredClone(r); changed.runtime.sharedProviders.push(file('/usr/lib/x86_64-linux-gnu/libz.so.1.3'));
  assert.throws(() => reconcileLinuxProviderLoaderListings(changed, lists));
});
test('observed versioned library names project uniquely and conflicting or missing receipt pins reject', () => {
  const r = receipt(); r.runtime.sharedProviders.push(file('/usr/lib/x86_64-linux-gnu/libz.so.1.3'));
  const listing = Buffer.from('libz.so.1 => /lib/x86_64-linux-gnu/libz.so.1 (0xabc)\nlibc.so.6 => /lib/x86_64-linux-gnu/libc.so.6 (0xdef)\n/lib64/ld-linux-x86-64.so.2 (0x123)\n');
  const lists = { dpkgDeb: listing, dpkgQuery: listing, time: listing };
  assert.equal(reconcileLinuxProviderLoaderListings(r, lists)[0].dependencies
    .find(row => row.observedAlias === '/lib/x86_64-linux-gnu/libz.so.1').declaredPath,
    '/usr/lib/x86_64-linux-gnu/libz.so.1.3');
  const ambiguous = structuredClone(r); ambiguous.runtime.sharedProviders.push(file('/usr/lib/x86_64-linux-gnu/libz.so.1.4'));
  assert.throws(() => reconcileLinuxProviderLoaderListings(ambiguous, lists), /unique declared/u);
  const inconsistent = structuredClone(r); inconsistent.runtime.loader.sha256 = '2'.repeat(64);
  assert.throws(() => reconcileLinuxProviderLoaderListings(inconsistent, lists));
});
test('matching receipt and acquisition reach the real fixed-package hash gate; synthetic bytes cannot pass', () => {
  const f = fixture();
  assert.throws(() => replayNumericLinuxProviderOriginals(f.input, f.bytes), /exact fixed package hash/u);
  for (const change of [v => v.delete('runtime-receipt.json'), v => v.set('runtime-receipt.json', Buffer.from('{}\n')),
    v => v.set('acquisition.stdout', Buffer.from('302')), v => v.delete('acquisition-process.json')]) {
    const changed = fixture(change); assert.throws(() => replayNumericLinuxProviderOriginals(changed.input, changed.bytes));
  }
});
test('provider ZIP digest, root confinement and capture/binding/admission originals are mandatory', () => {
  const f = fixture();
  for (const input of [{ ...f.input, artifact: { ...f.input.artifact, sha256: '0'.repeat(64) } },
    { ...f.input, binding: encode({}) }, { ...f.input, admission: encode({}) }])
    assert.throws(() => replayNumericLinuxProviderOriginals(input, f.bytes));
  const bytes = archiveFixture([['foreign-provider/runtime-receipt.json', f.input.receipt]]);
  assert.throws(() => replayNumericLinuxProviderOriginals({ ...f.input,
    artifact: { bytes: bytes.length, sha256: sha256(bytes) } }, bytes), /one exact selected provider root/u);
});
test('fixed live owner rejects copied keys, override arguments and oversized non-Buffer ZIP inputs', () => {
  assert.equal(reconcileNumericCampaignLinuxProvider.length, 4); assert.equal(inspectNumericCampaignLinuxProvider.length, 4);
  assert.throws(() => reconcileNumericCampaignLinuxProvider({}, {}, {}, Buffer.alloc(22), () => {}), /fixed provider owner/u);
  for (const bytes of [null, new Uint8Array(22), Buffer.alloc(21), Buffer.alloc(1024 * 1024 + 1)])
    assert.throws(() => reconcileNumericCampaignLinuxProvider({}, {}, {}, bytes), /bounded original Linux provider ZIP/u);
  assert.throws(() => reconcileNumericCampaignLinuxProvider({}, {}, {}, Buffer.alloc(22)), /own archive key/u);
  for (const key of [{}, Object.freeze({}), { providerOriginalsVerified: true }])
    assert.throws(() => inspectNumericCampaignLinuxProvider({}, {}, {}, key), /own provider key/u);
});
test('pure originals worker cannot accept caller readers, missing originals or extra root selectors', () => {
  const f = fixture();
  assert.throws(() => replayNumericLinuxProviderOriginals(f.input, f.bytes, () => {}), /fixed pure original replay/u);
  assert.throws(() => replayNumericLinuxProviderOriginals({ ...f.input, reader: () => {} }, f.bytes));
  for (const key of ['capture', 'binding', 'admission', 'receipt']) {
    const input = { ...f.input }; delete input[key]; assert.throws(() => replayNumericLinuxProviderOriginals(input, f.bytes));
  }
});
