import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { performance } from 'node:perf_hooks';
import { remaining } from './process.mjs';
import { lstat, readFile, readdir, readlink, realpath, writeFile } from 'node:fs/promises';
import { basename, join, relative, sep } from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { decodeTar } from '../../distribution/archive-tar.mjs';
import { decodeZip } from '../../distribution/archive-zip.mjs';
import { extractVerifiedProductionFiles } from '../../distribution-release/run-installed-acceptance.mjs';
import { vsixEntries } from '../../portable-setup/vsix.mjs';
import { descriptor, ordinaryRoot, capture, readBytes } from './files.mjs';
export { descriptor, ordinaryRoot } from './files.mjs';
import { EDITOR_ARCHIVES, EDITOR_VERSION } from './pins.mjs';

const require = createRequire(import.meta.url);
const { verifyInstallation } = require('../../../editors/vscode-zryna/src/installation.cjs');
export const sha = bytes => createHash('sha256').update(bytes).digest('hex');
export async function canonicalJson(file, limit = 262144, deadline = performance.now() + 30000) {
  const bytes = await readBytes(file, deadline, limit);
  const value = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
  assert(Buffer.from(`${JSON.stringify(value, null, 2)}\n`).equals(bytes), 'Noncanonical JSON.');
  return value;
}
export async function prepareSetup(handoff, expected, output, deadline) {
  await ordinaryRoot(handoff);
  const receipt = await canonicalJson(join(handoff, 'candidate-receipt.json'), 262144, deadline);
  const reproduction = await canonicalJson(join(handoff, 'reproduction.json'), 262144, deadline);
  const target = process.platform === 'win32' ? 'x86_64-pc-windows-msvc' : 'x86_64-unknown-linux-gnu';
  assert.equal(receipt.format, 'zryna.portable-setup-receipt.v1');
  assert.equal(receipt.status, 'review-candidate');
  assert.equal(receipt.productionAdmission, 'forbidden');
  assert.equal(receipt.sourceCommit, expected.commit);
  assert.equal(receipt.sourceTree, expected.tree);
  assert.equal(receipt.target, target);
  assert.equal(receipt.archive.sha256, expected.archive);
  assert.equal(receipt.manifestSha256, expected.manifest);
  assert.equal(receipt.serverSha256, expected.server);
  assert.equal(receipt.vsixSha256, expected.vsix);
  assert.equal(reproduction.sourceCommit, expected.commit);
  assert.equal(reproduction.target, target);
  for (const key of ['serverReplicas', 'vsixReplicas']) {
    assert.equal(reproduction[key].length, 2);
    assert.equal(reproduction[key][0], reproduction[key][1]);
    assert.equal(reproduction[key][0], key === 'serverReplicas' ? receipt.serverSha256 : receipt.vsixSha256);
  }
  const extension = process.platform === 'win32' ? '.zip' : '.tar.gz';
  const name = `zryna-setup-0.1.0-candidate.3-${target}`;
  assert.equal(receipt.archive.filename, name + extension);
  const archive = join(handoff, receipt.archive.filename);
  assert(Number.isSafeInteger(receipt.archive.size) && receipt.archive.size > 0
    && receipt.archive.size <= 768 * 1024 * 1024);
  const captured = await capture(archive, join(output, 'authenticated-setup.archive'),
    { bytes: receipt.archive.size, sha256: expected.archive }, deadline);
  const actual = captured.source;
  const bytes = await readFile(captured.captured.path);
  const files = process.platform === 'win32'
    ? decodeZip(bytes, name, ['bin/zryna-language-server.exe', 'compiler/bin/zryna.exe',
      'compiler/runtime/node/node.exe']) : await decodeTar(bytes, name, expected.epoch);
  const manifest = files.find(file => file.path === 'setup.json');
  assert(manifest && sha(manifest.data) === receipt.manifestSha256);
  const setup = join(output, name);
  extractVerifiedProductionFiles(setup, files);
  const config = verifyInstallation(setup, receipt.manifestSha256);
  assert.equal(config.manifest.sourceCommit, expected.commit);
  assert.equal(config.manifest.sourceTree, expected.tree);
  const vsix = join(setup, 'editor', 'zryna-0.5.0.vsix');
  assert.equal((await descriptor(vsix)).sha256, receipt.vsixSha256);
  const entries = vsixEntries(await readFile(vsix));
  const sourceFiles = ['CHANGELOG.md', 'LICENSE', 'README.md', 'package.json',
    'src/connection.cjs', 'src/extension.cjs', 'src/installation.cjs', 'src/run-command.cjs',
    'src/run-input.cjs', 'src/run-process.cjs', 'src/run-project.cjs', 'syntaxes/zryna.tmLanguage.json'];
  assert.equal(entries.length, sourceFiles.length + 2);
  const extensionRoot = fileURLToPath(new URL('../../../editors/vscode-zryna/', import.meta.url));
  for (const path of sourceFiles) {
    const packaged = { 'CHANGELOG.md': 'changelog.md', 'README.md': 'readme.md', LICENSE: 'LICENSE.txt' }[path] ?? path;
    const entry = entries.find(entry => entry.name === `extension/${packaged}`);
    assert(entry && entry.data.equals(await readFile(join(extensionRoot, path))),
      `VSIX source binding differs: ${path}`);
  }
  const payload = Object.fromEntries(entries
    .filter(entry => entry.name.startsWith('extension/'))
    .map(entry => [entry.name.slice(10), { bytes: entry.data.length, sha256: sha(entry.data) }]));
  assert.equal(Object.keys(payload).length, 12);
  await writeFile(join(output, 'setup-authentication.json'), `${JSON.stringify({ receipt,
    reproduction, archive: actual, protectedCapture: captured.captured, setup, payload }, null, 2)}\n`, { flag: 'wx' });
  return { setup, digest: receipt.manifestSha256, vsix, payload };
}
export async function inventory(root, deadline = performance.now() + 30000) {
  const entries = [];
  let total = 0;
  async function walk(directory) {
    for (const name of (await readdir(directory)).sort()) {
      remaining(deadline);
      const path = join(directory, name), logical = relative(root, path).split(sep).join('/');
      assert(entries.length < 25000 && logical.split('/').length <= 32);
      const stat = await lstat(path);
      const row = { path: logical, mode: stat.mode & 0o7777 };
      if (stat.isSymbolicLink()) {
        const target = await readlink(path), actual = await realpath(path);
        assert(actual === root || actual.startsWith(root + sep), 'Editor alias escapes root.');
        entries.push({ ...row, kind: 'link', target });
      } else if (stat.isDirectory()) {
        entries.push({ ...row, kind: 'directory' });
        await walk(path);
      } else {
        assert(stat.isFile() && stat.size <= 512 * 1024 * 1024);
        total += stat.size;
        assert(total <= 2 * 1024 * 1024 * 1024);
        const data = await descriptor(path, deadline);
        entries.push({ ...row, kind: 'file', bytes: data.bytes, sha256: data.sha256 });
      }
    }
  }
  await ordinaryRoot(root);
  await walk(root);
  return { entries, totalBytes: total, sha256: sha(Buffer.from(JSON.stringify(entries))) };
}
export async function verifyInstalled(directory, expected, deadline = performance.now() + 30000) {
  const actual = await inventory(directory, deadline);
  const directories = new Set();
  for (const name of Object.keys(expected)) {
    const parts = name.split('/');
    for (let length = 1; length < parts.length; length++) directories.add(parts.slice(0, length).join('/'));
  }
  assert.deepEqual(actual.entries.filter(entry => entry.kind === 'directory').map(entry => entry.path).sort(),
    [...directories].sort(), 'Installed VSIX directory namespace differs.');
  const files = actual.entries.filter(entry => entry.kind === 'file');
  assert.equal(files.length, Object.keys(expected).length, 'Installed VSIX complete file count differs.');
  assert(actual.entries.every(entry => entry.kind !== 'link'), 'Installed VSIX link forbidden.');
  for (const file of files) {
    assert(expected[file.path], 'Unexpected installed VSIX file.');
    assert.equal(file.bytes, expected[file.path].bytes);
    assert.equal(file.sha256, expected[file.path].sha256, `Installed VSIX body differs: ${file.path}`);
  }
  return actual;
}
export async function verifyEditorArchive(archive, deadline) {
  const pin = EDITOR_ARCHIVES[process.platform];
  assert(pin && process.arch === 'x64');
  const actual = await descriptor(archive, deadline, pin.bytes);
  assert.equal(actual.bytes, pin.bytes);
  assert.equal(actual.sha256, pin.sha256);
  return { ...actual, official: pin };
}
export async function verifyEditor(root, deadline) {
  const base = process.platform === 'win32' ? root : join(root, 'VSCode-linux-x64');
  const executable = join(base, process.platform === 'win32' ? 'Code.exe' : 'code');
  const product = JSON.parse(await readFile(join(base, 'resources/app/product.json'), 'utf8'));
  const packageData = JSON.parse(await readFile(join(base, 'resources/app/package.json'), 'utf8'));
  assert.equal(packageData.version, EDITOR_VERSION);
  assert.match(product.commit, /^[a-f0-9]{40}$/);
  return { executable, descriptor: await descriptor(executable, deadline), productCommit: product.commit,
    version: packageData.version, inventory: await inventory(root, deadline) };
}
export async function installedDirectory(extensions) {
  const rows = (await readdir(extensions)).filter(name => name.startsWith('zryna.zryna-'));
  assert.equal(rows.length, 1);
  const path = join(extensions, rows[0]);
  assert.equal(basename(path), 'zryna.zryna-0.5.0');
  return path;
}
