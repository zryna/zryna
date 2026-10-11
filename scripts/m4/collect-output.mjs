import assert from 'node:assert/strict';
import path from 'node:path';
import { closeSync, constants, fstatSync, lstatSync, mkdirSync, openSync, realpathSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { digest, inventory, parseCanonical, portablePath, readStableFile } from './registry-files.mjs';
import { MAX_ARTIFACT, MAX_LOG, MAX_RECEIPT, exact, retained } from './evidence-input.mjs';
import { validateM4Evidence } from './evidence.mjs';
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

export function retainedOutputFiles(receipt, receiptBytes) {
  assert(receiptBytes.length <= MAX_RECEIPT, 'original receipt byte bound');
  assert.deepEqual(parseCanonical(receiptBytes, 'original receipt'), receipt, 'original receipt value identity');
  const files = [...receipt.tools.map(tool => tool.versionLog),
    ...Object.values(receipt.sourceState).flatMap(phase => Object.values(phase).flatMap(item => [item.stdout, item.stderr])),
    ...receipt.results.flatMap(result => [result.stdout, result.stderr, ...result.attachments]),
    { path: 'receipt.json', bytes: receiptBytes.length, sha256: digest(receiptBytes) }];
  assert(files.length <= 256, 'retained ZIP entry count including receipt');
  const seen = new Set();
  let total = 0;
  for (const file of files) {
    exact(file, ['path', 'bytes', 'sha256'], 'staged descriptor');
    portablePath(file.path);
    assert(Number.isSafeInteger(file.bytes) && file.bytes >= 0 && file.bytes <= MAX_ARTIFACT, 'staged file byte bound');
    assert.match(file.sha256, /^[a-f0-9]{64}$/);
    const key = file.path.toLowerCase();
    assert(!seen.has(key), 'staged duplicate or case collision');
    seen.add(key);
    total += file.bytes;
  }
  assert(total <= 768 * 1024 * 1024, 'staged aggregate byte bound');
  const zipOverhead = 22 + files.reduce((sum, file) => sum + 30 + 46 + 16 + 2 * Buffer.byteLength(file.path, 'utf8'), 0);
  assert(total + zipOverhead <= 768 * 1024 * 1024, 'staged ZIP byte bound including supported record overhead');
  const directorySpellings = new Map();
  for (const file of files) {
    const parts = file.path.split('/');
    for (let index = 1; index < parts.length; index++) {
      const prefix = parts.slice(0, index).join('/');
      const key = prefix.toLowerCase();
      assert(!seen.has(key), 'staged parent file conflict');
      assert(!directorySpellings.has(key) || directorySpellings.get(key) === prefix, 'staged directory prefix case alias');
      directorySpellings.set(key, prefix);
    }
  }
  return files;
}
function directoryChain(target) {
  const entries = [];
  let current = path.parse(target).root;
  for (const part of ['', ...target.slice(current.length).split(path.sep).filter(Boolean)]) {
    if (part) current = path.join(current, part);
    const stat = lstatSync(current, { bigint: true });
    assert(stat.isDirectory() && !stat.isSymbolicLink(), 'staged ancestor must be a non-linked directory');
    assert.equal(realpathSync(current), current, 'staged ancestor must be canonical');
    entries.push({ path: current, dev: stat.dev, ino: stat.ino });
  }
  return entries;
}
function sameIdentity(actual, expected) {
  assert.equal(actual.dev, expected.dev, 'staged device identity changed');
  assert.equal(actual.ino, expected.ino, 'staged file identity changed');
}

// Linux mutations use open parent handles. Windows retains mandatory reparse,
// canonical path and handle identity checks where Node lacks POSIX directory flags.
export class ClosedEvidenceStage {
  constructor(root) {
    assert(path.isAbsolute(root) && path.resolve(root) === root, 'canonical absolute evidence root');
    this.root = root;
    this.target = `${root}-retained`;
    this.chain = directoryChain(root);
    this.directories = new Map();
    this.handles = [];
    try { lstatSync(this.target); assert.fail('retained stage already exists'); }
    catch (error) { if (error.code !== 'ENOENT') throw error; }
    if (process.platform !== 'win32') {
      assert(process.platform === 'linux' && constants.O_DIRECTORY && constants.O_NOFOLLOW,
        'Unix retained staging requires Linux anchored no-follow capabilities');
    }
    this.created = false;
  }
  unchanged() {
    for (const entry of [...this.chain, ...this.directories.values()]) {
      const stat = lstatSync(entry.path, { bigint: true });
      assert(stat.isDirectory() && !stat.isSymbolicLink(), 'staged ancestor changed shape');
      sameIdentity(stat, entry);
      assert.equal(realpathSync(entry.path), entry.path);
      if (entry.fd !== undefined) sameIdentity(fstatSync(entry.fd, { bigint: true }), entry);
    }
  }
  openDirectory(directory) {
    const stat = lstatSync(directory, { bigint: true });
    const entry = { path: directory, dev: stat.dev, ino: stat.ino };
    assert(stat.isDirectory() && !stat.isSymbolicLink());
    if (process.platform === 'linux') {
      entry.fd = openSync(directory, constants.O_RDONLY | constants.O_DIRECTORY | constants.O_NOFOLLOW);
      this.handles.push(entry.fd);
      sameIdentity(fstatSync(entry.fd, { bigint: true }), entry);
      assert.equal(realpathSync(`/proc/self/fd/${entry.fd}`), directory, 'anchored directory capability');
    }
    this.directories.set(directory, entry);
    this.unchanged();
    return entry;
  }
  child(parent, name) {
    return parent.fd === undefined ? path.join(parent.path, name) : `/proc/self/fd/${parent.fd}/${name}`;
  }
  create() {
    this.unchanged();
    const parent = this.openDirectory(path.dirname(this.root));
    mkdirSync(this.child(parent, path.basename(this.target)), { mode: 0o700 });
    this.unchanged();
    this.openDirectory(this.target);
    this.created = true;
    return this;
  }
  write(descriptor, bytes) {
    assert(this.created && Buffer.isBuffer(bytes));
    assert(bytes.length <= MAX_ARTIFACT, 'staged file byte bound');
    exact(descriptor, ['path', 'bytes', 'sha256'], 'staged descriptor');
    portablePath(descriptor.path);
    assert.equal(bytes.length, descriptor.bytes, 'staged bytes identity');
    assert.equal(digest(bytes), descriptor.sha256, 'staged digest identity');
    this.unchanged();
    const parts = descriptor.path.split('/');
    let parent = this.directories.get(this.target);
    for (const part of parts.slice(0, -1)) {
      const directory = path.join(parent.path, part);
      if (!this.directories.has(directory)) {
        mkdirSync(this.child(parent, part), { mode: 0o700 });
        this.unchanged();
        this.openDirectory(directory);
      }
      parent = this.directories.get(directory);
    }
    this.unchanged();
    const fd = openSync(this.child(parent, parts.at(-1)), constants.O_WRONLY | constants.O_CREAT
      | constants.O_EXCL | (constants.O_NOFOLLOW ?? 0), 0o600);
    try {
      const opened = fstatSync(fd, { bigint: true });
      assert(opened.isFile(), 'staged output must be regular');
      writeFileSync(fd, bytes);
      this.unchanged();
      const file = path.join(parent.path, parts.at(-1));
      const after = lstatSync(file, { bigint: true });
      assert(after.isFile() && !after.isSymbolicLink(), 'staged output became linked or nonregular');
      sameIdentity(after, opened);
      sameIdentity(fstatSync(fd, { bigint: true }), opened);
      assert.equal(after.size, BigInt(bytes.length));
      assert(readStableFile(file, MAX_ARTIFACT).equals(bytes), 'staged retained bytes differ');
    } finally { closeSync(fd); }
    this.unchanged();
  }
  close() {
    for (const fd of this.handles.reverse()) closeSync(fd);
    this.handles = [];
  }
}

export async function stageM4Evidence(receipt, options) {
  // No caller flag or alternate validator can skip the complete native evidence contract.
  const observed = structuredClone(receipt);
  const context = structuredClone(options);
  await validateM4Evidence(observed, context);
  const outside = path.relative(context.sourceRoot, context.root);
  assert(outside && (path.isAbsolute(outside) || outside.split(path.sep)[0] === '..'), 'retained stage must be outside checkout');
  const bytes = readStableFile(path.join(context.root, 'receipt.json'), MAX_RECEIPT, 'original receipt');
  const files = retainedOutputFiles(observed, bytes);
  // PowerShell supplies mixed separators on Windows; normalize only filesystem lookup,
  // while the retained original receipt keeps its exact observed evidenceDirectory string.
  const stage = new ClosedEvidenceStage(path.resolve(context.root));
  try {
    stage.create();
    for (const descriptor of files.slice(0, -1)) {
      stage.write(descriptor, retained(descriptor, context.root, MAX_ARTIFACT));
    }
    assert(readStableFile(path.join(context.root, 'receipt.json'), MAX_RECEIPT).equals(bytes), 'original receipt changed');
    stage.write(files.at(-1), bytes);
    return stage.target;
  } finally { stage.close(); }
}
