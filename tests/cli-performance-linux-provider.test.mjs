// Pure source-frozen units. No provider reader, child, network, compiler, or benchmark execution.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { canonical } from '../scripts/stability-gates/input.mjs';
import { GNU_TIME, PACKAGE, PROVIDER_SHA256, providerRoot, timeExecutable, validateObservedFile,
  validateProviderBinding, validateProviderRecord, validateRuntimeReceipt, validateUbuntuRelease }
  from '../scripts/stability-gates/cli-performance-linux-provider.mjs';
import { parseLoaderListing, validatePackageDownload }
  from '../scripts/stability-gates/cli-performance-linux-provision-provider.mjs';
const bytes = readFileSync(new URL('../scripts/stability-gates/provider-inputs/linux-gnu-time-debian-trixie-v2.json', import.meta.url));
const root = '/tmp/m418-linux-provider-AbC123';
const encode = value => Buffer.from(canonical(value));
function file(path, pin = {}, mode = 0o100755) {
  const size = pin.bytes ?? 123;
  return { path, bytes: size, sha256: pin.sha256 ?? '1'.repeat(64),
    identity: { dev: 1, ino: 2, size, mode, nlink: 1, uid: 1000, gid: 1000, mtimeMs: 1.25, ctimeMs: 2.5 } };
}
function receipt() {
  const loader = file('/usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2');
  const libc = file('/usr/lib/x86_64-linux-gnu/libc.so.6');
  return { format: 'zryna.gnu-time-runtime-receipt.v1', providerRoot: root, providerRecordSha256: PROVIDER_SHA256,
    acquisition: { curl: file('/usr/bin/curl'), version: 'observed curl version\n',
      receipt: file(`${root}/acquisition.json`, {}, 0o100600) },
    host: { platform: 'linux', arch: 'x64', osRelease: file('/usr/lib/os-release', {}, 0o100644) },
    package: file(`${root}/time.deb`, PACKAGE, 0o100600),
    tool: { file: file(timeExecutable(root).path, GNU_TIME), version: 'time (GNU Time) UNKNOWN\nactual output\n' },
    runtime: { dpkgDeb: file('/usr/bin/dpkg-deb'), dpkgQuery: file('/usr/bin/dpkg-query'), loader, libc,
      sharedProviders: [structuredClone(loader), structuredClone(libc)], versions: { dpkgDeb: 'observed dpkg version\n', libc: 'observed libc version\n',
        loader: 'observed loader version\n' } },
    provenance: { packageAcquisition: 'fixed-url-exact-hash-this-host-no-fresh-gpg',
      hostPackages: 'dpkg\tobserved version\tamd64\n',
      historicalAnchor: 'historical-only-a719cdffdae1f744c92b2d93f65103a1e4cfc3429d2fc55cdb85fe3ffbf8470a' },
    sourceAuthenticated: false, performanceQualified: false, baselineAuthority: null, thresholds: null };
}
function changedReceipt(mutate) {
  const value = receipt();
  validateRuntimeReceipt(encode(value), root); // baseline must reach and pass the actual validator
  mutate(value);
  const encoded = encode(value); // construction/canonical failures are not negative-validator passes
  assert.throws(() => validateRuntimeReceipt(encoded, root));
}
test('portable recipe fixes original package and binary bytes while historical GPG stays historical', () => {
  const record = validateProviderRecord(bytes);
  assert.equal(record.package.SHA256, PACKAGE.sha256);
  assert.equal(record.executable.sha256, GNU_TIME.sha256);
  assert.equal(record.historicalAnchor.currentHostVerification, false);
  assert.equal(record.package.Size, '50776'); assert.equal(record.executable.bytes, 27384);
  assert.equal(record.memoryMethod, 'gnu-time-wait4-maxrss-kib.v1');
  assert.equal(record.sourceAuthenticated, false); assert.equal(record.performanceQualified, false);
});
test('portable recipe rejects alternate URL hashes paths fake version or authority fields', () => {
  for (const mutate of [r => { r.downloadUrl = 'https://example.test/time.deb'; },
    r => { r.executable.sha256 = '0'.repeat(64); }, r => { r.executable.relativePath = 'other/time'; },
    r => { r.package.Version = '1.9'; }, r => { r.historicalAnchor.currentHostVerification = true; },
    r => { r.sourceAuthenticated = true; }, r => { r.performanceQualified = true; },
    r => { r.baselineAuthority = {}; }, r => { r.thresholds = { rss: 1 }; }, r => { r.extra = true; }]) {
    const value = JSON.parse(bytes); mutate(value); assert.throws(() => validateProviderRecord(encode(value)));
  }
});
test('portable provider root permits only fresh fixed-recipe instances and derives sole executable leaf', () => {
  assert.equal(providerRoot(root), root);
  assert.deepEqual(timeExecutable(root), { path: `${root}/extracted/usr/bin/time`, ...GNU_TIME });
  for (const value of ['/usr/bin', '/tmp/other-AbC123', `${root}/..`, `${root}/child`,
    '/tmp/m418-linux-provider-short', '/tmp/m418-linux-provider-ABC1234', '/tmp/m418-linux-provider-ABC_12'])
    assert.throws(() => providerRoot(value));
});
test('portable capture binding cannot replace tool bytes executable leaf recipe or historical receipt', () => {
  const fixture = () => ({ providerRoot: root, providerRecordSha256: PROVIDER_SHA256,
    providerReceiptSha256: '2'.repeat(64), tools: { time: timeExecutable(root) } });
  validateProviderBinding(fixture());
  for (const mutate of [c => { c.tools.time.path = '/usr/bin/time'; },
    c => { c.tools.time.sha256 = '0'.repeat(64); }, c => { c.tools.time.bytes += 1; },
    c => { c.providerRecordSha256 = '0'.repeat(64); }, c => { c.providerReceiptSha256 = 'bad'; },
    c => { c.providerOriginSha256 = 'a'.repeat(64); }, c => { c.providerRoot = '/tmp/alternate'; }]) {
    const value = fixture(); mutate(value); assert.throws(() => validateProviderBinding(value));
  }
});
test('portable receipt keeps fresh runtime and local package provenance separate from historical anchor', () => {
  const value = receipt(); assert.deepEqual(validateRuntimeReceipt(encode(value), root), value);
  assert.equal(value.tool.version.split('\n')[0], 'time (GNU Time) UNKNOWN');
  assert.equal(value.performanceQualified, false); assert.equal(value.sourceAuthenticated, false);
  assert.equal(value.baselineAuthority, null); assert.equal(value.thresholds, null);
});
test('portable receipt rejects omitted extra duplicate noncanonical or historical proof substitutions', () => {
  for (const mutate of [r => { delete r.runtime.libc; }, r => { r.runtime.extra = {}; },
    r => { r.provenance.currentHostGpgVerified = true; }, r => { r.provenance.historicalAnchor = 'fresh-gpg'; },
    r => { r.provenance.packageAcquisition = 'historical-origin-file'; },
    r => { r.tool.version = 'time (GNU Time) 1.9-0.2\n'; }, r => { r.sourceAuthenticated = true; },
    r => { r.performanceQualified = true; }, r => { r.thresholds = {}; }]) changedReceipt(mutate);
  for (const value of [Buffer.from('{"a":1,"a":2}\n'), Buffer.from(JSON.stringify(receipt(), null, 2)), Buffer.alloc(262145)])
    assert.throws(() => validateRuntimeReceipt(value, root));
});
test('portable receipt rejects wrong package binary size path mode link count and identity projection', () => {
  for (const mutate of [r => { r.package.sha256 = '0'.repeat(64); }, r => { r.package.bytes += 1; },
    r => { r.package.path = '/tmp/other.deb'; }, r => { r.package.identity.mode = 0o100644; },
    r => { r.tool.file.sha256 = '0'.repeat(64); }, r => { r.tool.file.path = '/usr/bin/time'; },
    r => { r.tool.file.identity.nlink = 2; }, r => { r.tool.file.identity.mode = 0o100777; },
    r => { r.tool.file.identity.mode = 0o120755; }, r => { r.tool.file.identity.size += 1; },
    r => { r.tool.file.identity.extra = 0; }, r => { r.tool.file.identity.dev = -0.5; }]) changedReceipt(mutate);
  assert.throws(() => validateObservedFile(file('/tmp/input', {}, 0o100666)));
});
test('portable runtime projection rejects alternate dpkg libc loader and unbounded duplicate shared providers', () => {
  for (const mutate of [r => { r.acquisition.curl.path = '/tmp/curl'; }, r => { r.acquisition.receipt.path = '/tmp/old-acquisition.json'; },
    r => { r.runtime.dpkgDeb.path = '/tmp/dpkg-deb'; },
    r => { r.runtime.dpkgQuery.path = '/tmp/dpkg-query'; }, r => { r.runtime.libc.path = '/tmp/libc.so.6'; },
    r => { r.runtime.loader.path = '/tmp/loader'; }, r => { r.runtime.sharedProviders = []; },
    r => { r.runtime.sharedProviders.push(structuredClone(r.runtime.libc)); }, r => { r.runtime.sharedProviders.reverse(); },
    r => { r.runtime.sharedProviders[0].path = '/tmp/library.so'; },
    r => { r.runtime.versions.libc = ''; }, r => { r.provenance.hostPackages = ''; }]) changedReceipt(mutate);
});
test('portable host admission rejects other distributions versions architecture and relabeled old host paths', () => {
  validateUbuntuRelease(Buffer.from('ID=ubuntu\nVERSION_ID="24.04"\n'));
  for (const text of ['ID=debian\nVERSION_ID="24.04"\n', 'ID=ubuntu\nVERSION_ID="22.04"\n'])
    assert.throws(() => validateUbuntuRelease(Buffer.from(text)));
  changedReceipt(r => { r.host.arch = 'arm64'; });
  changedReceipt(r => { r.host.osRelease.path = '/tmp/old-host-os-release'; });
  assert.throws(() => validateRuntimeReceipt(encode(receipt()), '/tmp/m418-linux-provider-DEF456'));
});
test('portable package admission bounds 128 KiB before extraction and rejects same-size wrong digest', () => {
  for (const value of [Buffer.alloc(0), Buffer.alloc(PACKAGE.bytes - 1), Buffer.alloc(PACKAGE.bytes),
    Buffer.alloc(128 * 1024 + 1)]) assert.throws(() => validatePackageDownload(value));
});
test('portable loader projection accepts explicit system closure and rejects missing alternate or ambiguous providers', () => {
  assert.deepEqual(parseLoaderListing('\tlinux-vdso.so.1 (0xabc)\n\tlibc.so.6 => /lib/x86_64-linux-gnu/libc.so.6 (0xdef)\n\t/lib64/ld-linux-x86-64.so.2 (0x123)\n'),
    ['/lib/x86_64-linux-gnu/libc.so.6', '/lib64/ld-linux-x86-64.so.2']);
  for (const text of ['libc.so.6 => not found\n', 'libc.so.6 => /tmp/libc.so.6 (0xabc)\n',
    'warning: preload active\n', 'libc.so.6 => /lib/x86_64-linux-gnu/../evil (0xabc)\n', ''])
    assert.throws(() => parseLoaderListing(text));
});

test('loader projection accepts the observed absolute system interpreter alias only', () => {
  const line = '\t/lib64/ld-linux-x86-64.so.2 => /usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2 (0x7fa878687000)\n';
  assert.deepEqual(parseLoaderListing(line), ['/usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2']);
  for (const text of [line.replace('/lib64/', '/tmp/'),
    line.replace(' => /usr/lib/', ' => /tmp/'),
    line.replace(' => /usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2', ' => /lib/x86_64-linux-gnu/libc.so.6')])
    assert.throws(() => parseLoaderListing(text));
});
