import assert from 'node:assert/strict';
import path from 'node:path';
import { mkdirSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { digest, inventory, parseCanonical, portablePath, readStableFile } from './registry-files.mjs';
import { MAX_ARTIFACT, MAX_LOG, MAX_RECEIPT } from './evidence-input.mjs';
import { actualFile } from './observe-materials.mjs';
import { installedByteCases, validateInstalledByteSource } from './installed-byte-boundary.mjs';
import { installedNames, positive } from './installed-cases.mjs';

const sourceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
export function retain(root, relative, bytes) {
  portablePath(relative);
  mkdirSync(path.dirname(path.join(root, relative)), { recursive: true, mode: 0o700 });
  writeFileSync(path.join(root, relative), bytes, { flag: 'wx', mode: 0o600 });
  return { path: relative, bytes: bytes.length, sha256: digest(bytes) };
}
function copy(root, relative, source, maximum = MAX_ARTIFACT) {
  return retain(root, relative, readStableFile(source, maximum, 'retained actual output'));
}
export function browserAttachments(root) {
  const directory = path.join(root, 'browser');
  const selected = inventory(root, 'browser').filter(name => /\.(json|log)$/.test(name) && !name.includes('/config/') && !name.includes('/data/'))
    .map(name => ({ ...actualFile(path.join(root, name), MAX_LOG), path: name }));
  const report = parseCanonical(readStableFile(path.join(directory, 'conformance.json'), MAX_RECEIPT));
  const pin = parseCanonical(readStableFile(path.join(sourceRoot, 'tests/scalar-host/browser-pin.json'), 64 * 1024));
  const cache = path.join(sourceRoot, '.zryna/cache', `scalar-browser-${report.host}-${pin.browserVersion}`);
  selected.push(copy(root, 'browser/archive.zip', path.join(cache, 'archive.zip')));
  return selected.sort((a, b) => a.path.localeCompare(b.path, 'en'));
}
export function installedAttachments(root) {
  const directory = path.join(root, 'installed');
  const report = parseCanonical(readStableFile(path.join(directory, 'qualification.json'), MAX_RECEIPT));
  const candidate = parseCanonical(readStableFile(path.join(directory, 'candidate.json'), MAX_RECEIPT));
  const invocations = parseCanonical(readStableFile(path.join(directory, 'invocations.json'), MAX_RECEIPT));
  assert.deepEqual(invocations.map(item => item.name), installedNames, 'all actual installed invocation identities before retention');
  assert.deepEqual(invocations.filter(item => item.manifestSha256).map(item => item.name).sort(), [...positive.keys()].sort(),
    'exact nine positive installed observations before retention');
  const descriptors = ['qualification.json', 'candidate.json', 'invocations.json', 'architecture-receipt.json', candidate.archive.filename]
    .map(name => ({ ...actualFile(path.join(directory, name), MAX_ARTIFACT), path: `installed/${name}` }));
  for (const row of invocations.filter(item => item.manifestSha256)) {
    const bundle = path.join(report.project, '.zryna/out', `${row.name}.wasi-command-run`);
    const manifestFile = path.join(bundle, 'zryna-wasi-command-manifest-v1.json');
    const manifestBytes = readStableFile(manifestFile, 16384);
    const manifest = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(manifestBytes));
    portablePath(manifest.component.path);
    descriptors.push(retain(root, `installed/${row.name}/manifest.json`, manifestBytes));
    descriptors.push(copy(root, `installed/${row.name}/component.wasm`, path.join(bundle, manifest.component.path)));
  }
  for (const row of invocations.filter(item => installedByteCases[item.name])) {
    const fixture = installedByteCases[row.name];
    const bytes = readStableFile(path.join(report.project, fixture.file), 256 * 1024);
    validateInstalledByteSource(row, bytes, sourceRoot);
    descriptors.push(retain(root, `installed/boundary-source/${fixture.file}`, bytes));
  }
  return descriptors.sort((a, b) => a.path.localeCompare(b.path, 'en'));
}
