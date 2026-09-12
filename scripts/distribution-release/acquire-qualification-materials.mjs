import { spawnSync } from 'node:child_process';
import { isAbsolute } from 'node:path';
import { orderedPaths, sha256 } from '../distribution/canonical.mjs';

const MAX_GIT_FILE = 16 * 1024 * 1024;
const MAX_CRATE = 64 * 1024 * 1024;
const REQUEST_TIMEOUT = 30_000;
const WASMTIME_LICENSE =
  'https://raw.githubusercontent.com/bytecodealliance/wasmtime/7bac2c2775808aaec5d4aa5627a5e447b51102cf/LICENSE';
const SOURCE_FILES = Object.freeze([
  ['LICENSE', 'LICENSE'],
  ['NOTICE', 'NOTICE'],
  ['adapters/typescript-6/src/limits-v3.mjs', 'lib/zryna/bootstrap/limits-v3.mjs'],
  ['adapters/typescript-6/src/limits-v4.mjs', 'lib/zryna/bootstrap/limits-v4.mjs'],
  ['adapters/typescript-6/src/worker-v3.mjs', 'lib/zryna/bootstrap/worker-v3.mjs'],
  ['adapters/typescript-6/src/worker-v4.mjs', 'lib/zryna/bootstrap/worker-v4.mjs'],
  ['adapters/typescript-6/src/worker.mjs', 'lib/zryna/bootstrap/worker.mjs'],
]);

function reject(message) {
  throw new Error(`R406-QUALIFICATION-ACQUISITION: ${message}`);
}

function exactUrl(value, hostname) {
  let url;
  try {
    url = new URL(value);
  } catch {
    reject('material URL is malformed');
  }
  if (url.protocol !== 'https:' || url.hostname !== hostname || url.port !== ''
    || url.username !== '' || url.password !== '' || url.search !== '' || url.hash !== '') {
    reject('material URL authority differs');
  }
  return url.href;
}

export async function fetchQualificationResource(descriptor, {
  fetchImpl = fetch, requestTimeoutMs = REQUEST_TIMEOUT,
} = {}) {
  const { url, maximum, size, digest } = descriptor;
  if (!Number.isSafeInteger(maximum) || maximum < 1 || maximum > 512 * 1024 * 1024
    || (size !== undefined && (!Number.isSafeInteger(size) || size < 1 || size > maximum))
    || (digest !== undefined && !/^[0-9a-f]{64}$/.test(digest))
    || !Number.isInteger(requestTimeoutMs) || requestTimeoutMs < 1
    || requestTimeoutMs > REQUEST_TIMEOUT) reject('material resource descriptor differs');
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), requestTimeoutMs);
  let response;
  try {
    response = await fetchImpl(url, {
      method: 'GET', redirect: 'error', signal: controller.signal,
      headers: { accept: 'application/octet-stream', 'accept-encoding': 'identity' },
    });
    if (!response || response.status !== 200 || response.url !== url || !response.body
      || ![null, 'identity'].includes(response.headers.get('content-encoding'))) {
      reject('material response identity differs');
    }
    const declared = response.headers.get('content-length');
    if (declared !== null && (!/^(0|[1-9][0-9]*)$/.test(declared)
      || Number(declared) < 1 || Number(declared) > maximum
      || (size !== undefined && Number(declared) !== size))) {
      reject('material response length differs');
    }
    const chunks = [];
    let length = 0;
    const reader = response.body.getReader();
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      if (!(value instanceof Uint8Array) || value.length < 1) reject('material response chunk differs');
      length += value.length;
      if (length > maximum || (size !== undefined && length > size)) {
        await reader.cancel();
        reject('material response exceeds its byte bound');
      }
      chunks.push(Buffer.from(value));
    }
    const bytes = Buffer.concat(chunks, length);
    if (bytes.length < 1 || (size !== undefined && bytes.length !== size)
      || (digest !== undefined && sha256(bytes) !== digest)) {
      reject('material response bytes differ');
    }
    return bytes;
  } catch (error) {
    if (error instanceof Error && error.message.startsWith('R406-QUALIFICATION-ACQUISITION:')) {
      throw error;
    }
    reject('material request failed');
  } finally {
    clearTimeout(timeout);
  }
}

function runGit(spawn, args, cwd, maximum = MAX_GIT_FILE) {
  const result = spawn('git', args, {
    cwd, encoding: null, maxBuffer: maximum + 1, shell: false, windowsHide: true,
  });
  if (result.error || result.status !== 0 || !Buffer.isBuffer(result.stdout)
    || result.stdout.length > maximum) reject(`git ${args[0]} failed or exceeded its bound`);
  return result.stdout;
}

function sourceBlob(spawn, sourceRoot, commit, sourcePath) {
  const entry = runGit(spawn, ['ls-tree', '-z', '--full-tree', commit, '--', sourcePath], sourceRoot);
  const suffix = Buffer.from(`\t${sourcePath}\0`);
  if (!entry.subarray(-suffix.length).equals(suffix) || entry.indexOf(0) !== entry.length - 1) {
    reject('source material tree entry is missing or ambiguous');
  }
  const match = /^(100644) (blob) ([0-9a-f]{40})$/
    .exec(entry.subarray(0, entry.length - suffix.length).toString('ascii'));
  if (!match) reject('source material must be one ordinary Git blob');
  const sizeBytes = runGit(spawn, ['cat-file', '-s', match[3]], sourceRoot, 64);
  const sizeText = sizeBytes.toString('ascii').trimEnd();
  if (!/^[1-9][0-9]*$/.test(sizeText) || Number(sizeText) > MAX_GIT_FILE) {
    reject('source material size differs');
  }
  const data = runGit(spawn, ['cat-file', 'blob', match[3]], sourceRoot, Number(sizeText));
  if (data.length !== Number(sizeText)) reject('source material bytes differ');
  return data;
}

async function defaultAdapters() {
  const [npm, node, rustCapture, rust, materials] = await Promise.all([
    import('../distribution/npm-materials.mjs'),
    import('../distribution/node-materials.mjs'),
    import('../distribution/rust-capture.mjs'),
    import('../distribution/rust-materials.mjs'),
    import('../distribution/materials.mjs'),
  ]);
  return {
    typeScriptMaterialDescriptors: npm.typeScriptMaterialDescriptors,
    captureTypeScriptMaterials: npm.captureTypeScriptMaterials,
    captureNodeMaterials: node.captureNodeMaterials,
    captureRustMaterials: rustCapture.captureRustMaterials,
    rustMaterials: rust.rustMaterials,
    nodeTarget: (target) => materials.NODE_TARGETS[target],
  };
}

function upstreamLicenseDescriptor(records) {
  const values = records.flatMap(({ files }) => files)
    .filter(({ origin }) => origin === WASMTIME_LICENSE)
    .map(({ size, sha256: digest }) => `${size}:${digest}`);
  const unique = [...new Set(values)];
  if (unique.length !== 1) reject('Wasmtime license descriptor is missing or ambiguous');
  const [size, digest] = unique[0].split(':');
  return { url: WASMTIME_LICENSE, maximum: Number(size), size: Number(size), digest };
}

export async function acquireQualificationMaterials({
  sourceRoot, sourceCommit, target, archiveCapability,
  fetchImpl = fetch, spawn = spawnSync, requestTimeoutMs = REQUEST_TIMEOUT, adapters,
}) {
  if (!isAbsolute(sourceRoot ?? '') || !/^[0-9a-f]{40}$/.test(sourceCommit ?? '')
    || !['x86_64-unknown-linux-gnu', 'x86_64-pc-windows-msvc'].includes(target)) {
    reject('exact source root, commit, and target are required');
  }
  const implementation = adapters ?? await defaultAdapters();
  for (const name of ['typeScriptMaterialDescriptors', 'captureTypeScriptMaterials',
    'captureNodeMaterials', 'captureRustMaterials', 'rustMaterials']) {
    if (typeof implementation[name] !== 'function') reject(`material adapter ${name} is missing`);
  }
  const request = (descriptor) => fetchQualificationResource(descriptor,
    { fetchImpl, requestTimeoutMs });
  const npm = implementation.typeScriptMaterialDescriptors();
  const keysUrl = exactUrl(npm.keys.url, 'registry.npmjs.org');
  const keys = await request({ url: keysUrl, maximum: npm.keys.size,
    size: npm.keys.size, digest: npm.keys.sha256 });
  const npmInputs = [];
  for (const entry of npm.packages) {
    const metadataUrl = exactUrl(entry.metadata.url, 'registry.npmjs.org');
    const archiveUrl = exactUrl(entry.tarball.url, 'registry.npmjs.org');
    npmInputs.push({
      metadata: await request({ url: metadataUrl, maximum: entry.metadata.size,
        size: entry.metadata.size, digest: entry.metadata.sha256 }),
      keys,
      archive: await request({ url: archiveUrl, maximum: entry.tarball.size,
        size: entry.tarball.size, digest: entry.tarball.sha256 }),
    });
  }
  if (typeof implementation.nodeTarget !== 'function') reject('material adapter nodeTarget is missing');
  const nodeRecord = implementation.nodeTarget(target);
  if (!nodeRecord) reject('Node material target differs');
  const nodeUrl = exactUrl(
    `https://nodejs.org/dist/v22.22.1/${nodeRecord.archive}`,
    'nodejs.org',
  );
  const nodeArchive = await request({ url: nodeUrl, maximum: nodeRecord.archiveSize,
    size: nodeRecord.archiveSize, digest: nodeRecord.archiveSha256 });
  const rustRecords = implementation.rustMaterials(target);
  const rustCaptures = [];
  for (const record of rustRecords) {
    const identity = `${record.name}-${record.version}`;
    if (!/^[a-z0-9_-]+-[0-9]+\.[0-9]+\.[0-9]+(?:[A-Za-z0-9.+-]*)$/.test(identity)) {
      reject('Rust material identity differs');
    }
    const url = exactUrl(
      `https://static.crates.io/crates/${record.name}/${identity}.crate`,
      'static.crates.io',
    );
    rustCaptures.push({ identity, archive: await request({
      url, maximum: MAX_CRATE, digest: record.crateSha256,
    }) });
  }
  const upstreamLicense = await request(upstreamLicenseDescriptor(rustRecords));
  const files = [
    ...SOURCE_FILES.map(([sourcePath, path]) => ({
      path, mode: 0o644, data: sourceBlob(spawn, sourceRoot, sourceCommit, sourcePath),
    })),
    ...await implementation.captureNodeMaterials(target, nodeArchive, archiveCapability),
    ...implementation.captureTypeScriptMaterials(npmInputs),
    ...implementation.captureRustMaterials(target, rustCaptures, upstreamLicense),
  ].sort((left, right) => left.path < right.path ? -1 : left.path > right.path ? 1 : 0);
  orderedPaths(files.map(({ path }) => path));
  if (files.some(({ mode, data }) => ![0o644, 0o755].includes(mode)
    || !Buffer.isBuffer(data) || data.length < 1)) reject('captured material bytes differ');
  return { capturedMaterials: files, rustCaptures };
}
