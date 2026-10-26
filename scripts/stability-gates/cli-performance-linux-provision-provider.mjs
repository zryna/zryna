// Explicit future operator entry: provision only the reviewed GNU time recipe. Never imported to run.
import assert from 'node:assert/strict';
import { chmodSync, lstatSync, mkdirSync, mkdtempSync, realpathSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';
import { canonical } from './input.mjs';
import { collectAcquisition } from './cli-performance-linux-acquire-provider.mjs';
export { validatePackageDownload } from './cli-performance-linux-acquire-provider.mjs';
import { GNU_TIME, PROVIDER_SHA256, RECEIPT_NAME, ROOT_PREFIX,
  fileIdentity, openLinuxTimeProvider, snapshotProviderFile, timeExecutable,
  validateRuntimeReceipt, validateUbuntuRelease } from './cli-performance-linux-provider.mjs';
export function parseLoaderListing(text) {
  assert(typeof text === 'string' && Buffer.byteLength(text) <= 16384);
  const paths = [];
  for (const line of text.trim().split('\n')) {
    if (/^\s*linux-vdso\.so\.1 \(0x[0-9a-f]+\)$/u.test(line)) continue;
    const match = /^\s*(?:[A-Za-z0-9_.+-]+ => )?(\/[A-Za-z0-9_./+-]+) \(0x[0-9a-f]+\)$/u.exec(line);
    assert(match, 'closed loader listing projection; no missing dependency');
    const path = match[1];
    assert.match(path, /^\/(?:usr\/)?lib\/x86_64-linux-gnu\/[A-Za-z0-9_.+-]+$|^\/lib64\/ld-linux-x86-64\.so\.2$/u);
    paths.push(path);
  }
  return [...new Set(paths)].sort();
}
// Root and all input/executable identities are checked around every provisioning subprocess.
function run(file, args, pins, root) {
  const beforeRoot = fileIdentity(lstatSync(root));
  for (const pin of pins) assert.deepEqual(snapshotProviderFile(pin.path).pin, pin);
  const result = spawnSync(file, args, { cwd: root, env: { PATH: '/usr/bin:/bin', LC_ALL: 'C', HOME: root },
    timeout: 30000, maxBuffer: 16384, encoding: 'utf8' });
  for (const pin of pins) assert.deepEqual(snapshotProviderFile(pin.path).pin, pin);
  const afterRoot = fileIdentity(lstatSync(root));
  for (const key of ['dev', 'ino', 'mode', 'nlink', 'uid', 'gid']) assert.equal(afterRoot[key], beforeRoot[key]);
  assert.equal(result.error, undefined, 'bounded subprocess completed');
  assert.equal(result.signal, null); assert.equal(result.status, 0);
  assert.equal(result.stderr, '', 'no unclassified provider warnings');
  return result.stdout;
}
export async function provisionLinuxTimeProvider({ checkoutRoot }) {
  assert.equal(process.platform, 'linux'); assert.equal(process.arch, 'x64');
  assert(typeof checkoutRoot === 'string' && checkoutRoot === resolve(checkoutRoot));
  assert.equal(realpathSync(checkoutRoot), checkoutRoot);
  const osRelease = snapshotProviderFile('/usr/lib/os-release', 16384);
  validateUbuntuRelease(osRelease.bytes);
  const tmp = lstatSync('/tmp');
  assert(tmp.isDirectory() && !tmp.isSymbolicLink() && tmp.uid === 0 && (tmp.mode & 0o1000), 'trusted sticky /tmp');
  const root = mkdtempSync(ROOT_PREFIX); chmodSync(root, 0o700);
  const rel = relative(checkoutRoot, root);
  assert(rel.startsWith('../') || rel === '..', 'fresh provider output must be outside checkout');
  // No partial failure is relabeled complete. Its fresh private root is retained for inspection.
  const archive = join(root, 'time.deb');
  writeFileSync(archive, Buffer.alloc(0), { flag: 'wx', mode: 0o600 });
  const outputIdentity = fileIdentity(lstatSync(archive));
  const curl = snapshotProviderFile('/usr/bin/curl').pin;
  const acquisitionPins = [osRelease.pin, curl];
  const curlVersion = run(curl.path, ['--version'], acquisitionPins, root);
  collectAcquisition({ root, io: { nowNs: () => process.hrtime.bigint(),
    check: () => {
      for (const pin of acquisitionPins) assert.deepEqual(snapshotProviderFile(pin.path).pin, pin);
      const current = fileIdentity(lstatSync(archive));
      for (const key of ['dev', 'ino', 'mode', 'nlink', 'uid', 'gid'])
        assert.equal(current[key], outputIdentity[key], 'same private acquisition output');
    },
    execute: call => {
      for (const pin of acquisitionPins) assert.deepEqual(snapshotProviderFile(pin.path).pin, pin);
      // Preserve managed proxy and CA variables for acquisition only; performance children use reviewed env.
      const result = spawnSync(call.file, call.args, { cwd: root, env: { ...process.env, LC_ALL: 'C' },
        timeout: call.timeoutMs, killSignal: 'SIGKILL', maxBuffer: 16384, encoding: null });
      return { status: result.status, signal: result.signal,
        error: result.error ? String(result.error.code ?? result.error.message) : null,
        stdout: result.stdout ?? Buffer.alloc(0), stderr: result.stderr ?? Buffer.alloc(0) };
    },
    readPackage: () => snapshotProviderFile(archive, 128 * 1024).bytes,
    retain: (name, bytes) => writeFileSync(join(root, name), bytes, { flag: 'wx', mode: 0o600 }),
  } });
  const acquisitionReceipt = snapshotProviderFile(join(root, 'acquisition.json'), 16384).pin;
  const packagePin = snapshotProviderFile(archive).pin;
  const dpkgDeb = snapshotProviderFile('/usr/bin/dpkg-deb').pin;
  const dpkgQuery = snapshotProviderFile('/usr/bin/dpkg-query').pin;
  const loader = snapshotProviderFile(realpathSync('/lib64/ld-linux-x86-64.so.2')).pin;
  const libc = snapshotProviderFile('/usr/lib/x86_64-linux-gnu/libc.so.6').pin;
  // Loader inspection is fixed to the two observed host executables and fixed extracted tool.
  const base = [osRelease.pin, packagePin, curl, acquisitionReceipt, dpkgDeb, dpkgQuery, loader, libc];
  const shared = new Map([[loader.path, loader], [libc.path, libc]]);
  for (const tool of [dpkgDeb, dpkgQuery]) {
    const listing = run(loader.path, ['--list', tool.path], [...base, ...shared.values()], root);
    for (const alias of parseLoaderListing(listing)) {
      const path = realpathSync(alias); shared.set(path, snapshotProviderFile(path).pin);
    }
  }
  const pins = () => [...base, ...shared.values()];
  const dpkgVersion = run(dpkgDeb.path, ['--version'], pins(), root);
  const loaderVersion = run(loader.path, ['--version'], pins(), root);
  const libcVersion = run(libc.path, [], pins(), root);
  const extracted = join(root, 'extracted'); mkdirSync(extracted, { mode: 0o700 });
  // Exact package bytes have already been bounded and hashed. No install or maintainer script operation.
  run(dpkgDeb.path, ['--extract', archive, extracted], pins(), root);
  const tool = snapshotProviderFile(timeExecutable(root).path).pin;
  assert.equal(tool.bytes, GNU_TIME.bytes); assert.equal(tool.sha256, GNU_TIME.sha256);
  const listing = run(loader.path, ['--list', tool.path], [...pins(), tool], root);
  for (const alias of parseLoaderListing(listing)) {
    const path = realpathSync(alias); shared.set(path, snapshotProviderFile(path).pin);
  }
  const version = run(tool.path, ['--version'], [...pins(), tool], root);
  // Host package ownership/version is observed provenance, never fresh upstream authentication.
  const hostPackages = run(dpkgQuery.path, ['--show', '--showformat=${binary:Package}\t${Version}\t${Architecture}\n',
    'dpkg', 'libc6:amd64', 'libbz2-1.0:amd64', 'liblzma5:amd64', 'libmd0:amd64', 'libzstd1:amd64', 'zlib1g:amd64'],
  [...pins(), tool], root);
  const receipt = { format: 'zryna.gnu-time-runtime-receipt.v1', providerRoot: root,
    providerRecordSha256: PROVIDER_SHA256, acquisition: { curl, version: curlVersion, receipt: acquisitionReceipt },
    host: { platform: process.platform, arch: process.arch,
      osRelease: osRelease.pin }, package: packagePin, tool: { file: tool, version },
    runtime: { dpkgDeb, dpkgQuery, loader, libc,
      sharedProviders: [...shared.values()].sort((a, b) => a.path.localeCompare(b.path, 'en')).map(pin => structuredClone(pin)),
      versions: { dpkgDeb: dpkgVersion, libc: libcVersion, loader: loaderVersion } },
    provenance: { packageAcquisition: 'fixed-url-exact-hash-this-host-no-fresh-gpg', hostPackages,
      historicalAnchor: 'historical-only-a719cdffdae1f744c92b2d93f65103a1e4cfc3429d2fc55cdb85fe3ffbf8470a' },
    sourceAuthenticated: false, performanceQualified: false, baselineAuthority: null, thresholds: null };
  const bytes = Buffer.from(canonical(receipt)); validateRuntimeReceipt(bytes, root);
  writeFileSync(join(root, RECEIPT_NAME), bytes, { flag: 'wx', mode: 0o600 });
  const provider = openLinuxTimeProvider({ providerRoot: root }); provider.check();
  return provider.binding;
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  assert.equal(process.argv.length, 3, 'exactly one canonical checkout root; no URL/hash/program/grant options');
  const binding = await provisionLinuxTimeProvider({ checkoutRoot: process.argv[2] });
  process.stdout.write(canonical(binding));
}
