import { canonical, canonicalBounded, parseCanonical, sha256 } from './canonical.mjs';
import {
  validateReleaseQualificationInput,
  validateReleaseQualificationInputText,
} from './validate-release-qualification-input.mjs';

const LIMITS = Object.freeze({ files: 512, file: 268435456, total: 536870912 });
const PATH = /^[A-Za-z0-9@][A-Za-z0-9@+._-]*(?:\/[A-Za-z0-9@][A-Za-z0-9@+._-]*){0,11}$/;

function reject(message) {
  throw new Error(`R406-QUALIFICATION-CORE: ${message}`);
}

function artifact(logicalPath, data) {
  return { logicalPath, size: data.length, sha256: sha256(data) };
}

function compareDescriptor(descriptor, file) {
  return descriptor.path === file.path && descriptor.mode === file.mode
    && descriptor.size === file.data.length && descriptor.sha256 === sha256(file.data);
}

function validateFiles(files, maximum = LIMITS.files) {
  if (!Array.isArray(files) || files.length < 1 || files.length > maximum) {
    reject('qualification file count differs');
  }
  let previous = '';
  let total = 0;
  const folded = new Set();
  for (const file of files) {
    const key = file?.path?.toLowerCase?.() ?? '';
    const prefixes = key.split('/').slice(0, -1)
      .map((_, index, segments) => segments.slice(0, index + 1).join('/'));
    if (!PATH.test(file?.path ?? '') || file.path <= previous || folded.has(key)
      || prefixes.some((prefix) => folded.has(prefix))
      || [...folded].some((path) => path.startsWith(`${key}/`))
      || ![0o644, 0o755].includes(file.mode) || !Buffer.isBuffer(file.data)
      || file.data.length < 1 || file.data.length > LIMITS.file) {
      reject('qualification file path, order, mode, or bytes differ');
    }
    previous = file.path;
    folded.add(key);
    total += file.data.length;
    if (total > LIMITS.total) reject('qualification expanded bytes exceed their bound');
  }
  return files;
}

function receiptFile(path, bytes, descriptor) {
  if (!Buffer.isBuffer(bytes) || descriptor.logicalPath !== path
    || canonical(artifact(path, bytes)) !== canonical({
      logicalPath: descriptor.logicalPath, size: descriptor.size, sha256: descriptor.sha256,
    })) reject(`${path} bytes differ from the qualification input`);
  return { path, mode: 0o644, data: bytes };
}

export function prepareQualification(input, capturedMaterials, receipts) {
  validateReleaseQualificationInput(input);
  validateFiles(capturedMaterials, 508);
  if (capturedMaterials.length !== input.materials.files.length
    || capturedMaterials.some((file, index) => !compareDescriptor(input.materials.files[index], file))
    || capturedMaterials.some(({ path }) => path.startsWith('qualification/'))
    || capturedMaterials.some(({ path }) => path === 'bin/zryna' || path === 'zryna.exe')) {
    reject('captured materials differ from the closed qualification descriptors');
  }
  const binding = Buffer.from(`${canonicalBounded(input)}\n`);
  const payload = [
    ...capturedMaterials,
    receiptFile('qualification/source.json', receipts.source, input.sourceReceipt),
    receiptFile('qualification/architecture.json', receipts.architecture,
      input.architectureReceipt),
    receiptFile('qualification/gates.json', receipts.gates, input.gateReceipt),
    { path: 'qualification/input.json', mode: 0o644, data: binding },
  ].sort((left, right) => left.path < right.path ? -1 : left.path > right.path ? 1 : 0);
  validateFiles(payload);
  return { binding, embeddingDigest: sha256(binding), payload };
}

function inventoryFile(file) {
  return { path: file.path, mode: file.mode, size: file.data.length, sha256: sha256(file.data) };
}

function checksumBytes(files) {
  return Buffer.from(files.map((file) => `${sha256(file.data)}  ${file.path}\n`).join(''));
}

function targetPaths(target) {
  return target === 'x86_64-pc-windows-msvc'
    ? { cli: 'zryna.exe', extension: 'zip' }
    : { cli: 'bin/zryna', extension: 'tar.gz' };
}

async function defaultPrimitives() {
  const [{ requireArchiveRuntime }, { encodeTar, decodeTar }, { encodeZip, decodeZip }, binary] =
    await Promise.all([
      import('../distribution/runtime.mjs'),
      import('../distribution/archive-tar.mjs'),
      import('../distribution/archive-zip.mjs'),
      import('../distribution/binary-identity.mjs'),
    ]);
  return { requireArchiveRuntime, encodeTar, decodeTar, encodeZip, decodeZip,
    verifyCompiledIdentity: binary.verifyCompiledIdentity };
}

export async function assembleQualification(binding, { payload, cli }, suppliedPrimitives) {
  if (!Buffer.isBuffer(binding) || !Buffer.isBuffer(cli) || cli.length < 64) {
    reject('qualification binding or compiled CLI bytes differ');
  }
  const input = validateReleaseQualificationInputText(binding.toString('utf8'));
  validateFiles(payload);
  const bindingFile = payload.find(({ path }) => path === 'qualification/input.json');
  if (!bindingFile?.data.equals(binding)) reject('qualification payload binding differs');
  const primitives = suppliedPrimitives ?? await defaultPrimitives();
  primitives.requireArchiveRuntime();
  primitives.verifyCompiledIdentity(cli, sha256(binding), input.target.triple);
  const paths = targetPaths(input.target.triple);
  const files = [...payload, { path: paths.cli, mode: 0o755, data: cli }]
    .sort((left, right) => left.path < right.path ? -1 : left.path > right.path ? 1 : 0);
  validateFiles(files);
  const inventory = Buffer.from(`${canonicalBounded({
    format: 'zryna.release-qualification-inventory.v1',
    status: 'qualification-only',
    productionAdmission: 'forbidden',
    files: files.map(inventoryFile),
  })}\n`);
  files.push({ path: 'qualification/inventory.json', mode: 0o644, data: inventory });
  files.sort((left, right) => left.path < right.path ? -1 : left.path > right.path ? 1 : 0);
  const checksums = checksumBytes(files);
  files.push({ path: 'qualification/checksums.sha256', mode: 0o644, data: checksums });
  files.sort((left, right) => left.path < right.path ? -1 : left.path > right.path ? 1 : 0);
  validateFiles(files);
  const archive = input.archive.format === 'zip'
    ? primitives.encodeZip(input.archive.root, files)
    : await primitives.encodeTar(input.archive.root, files, input.source.sourceDateEpoch);
  if (!Buffer.isBuffer(archive) || archive.length < 1 || archive.length > LIMITS.total) {
    reject('qualification archive bytes exceed their bound');
  }
  return {
    archive,
    filename: `${input.archive.root}.${paths.extension}`,
    files,
    inventory,
    checksums,
  };
}

export async function verifyQualification(archive, expected, suppliedPrimitives) {
  if (!Buffer.isBuffer(archive) || archive.length !== expected.size
    || sha256(archive) !== expected.sha256) reject('qualification archive descriptor differs');
  const input = validateReleaseQualificationInputText(expected.binding.toString('utf8'));
  const paths = targetPaths(input.target.triple);
  if (expected.filename !== `${input.archive.root}.${paths.extension}`) {
    reject('qualification archive filename differs');
  }
  const primitives = suppliedPrimitives ?? await defaultPrimitives();
  primitives.requireArchiveRuntime();
  const files = input.archive.format === 'zip'
    ? primitives.decodeZip(archive, input.archive.root, [paths.cli])
    : await primitives.decodeTar(archive, input.archive.root, input.source.sourceDateEpoch);
  validateFiles(files);
  const get = (path) => {
    const matches = files.filter((file) => file.path === path);
    if (matches.length !== 1) reject(`${path} is missing or ambiguous`);
    return matches[0];
  };
  if (!get('qualification/input.json').data.equals(expected.binding)) {
    reject('decoded qualification binding differs');
  }
  const inventory = parseCanonical(get('qualification/inventory.json').data.toString('utf8'));
  if (inventory.format !== 'zryna.release-qualification-inventory.v1'
    || inventory.status !== 'qualification-only' || inventory.productionAdmission !== 'forbidden') {
    reject('qualification inventory identity differs');
  }
  const indexed = files.filter(({ path }) => ![
    'qualification/inventory.json', 'qualification/checksums.sha256',
  ].includes(path));
  if (canonical(inventory.files) !== canonical(indexed.map(inventoryFile))
    || !get('qualification/checksums.sha256').data.equals(checksumBytes(
      files.filter(({ path }) => path !== 'qualification/checksums.sha256'),
    ))) reject('qualification inventory or checksum coverage differs');
  primitives.verifyCompiledIdentity(get(paths.cli).data, sha256(expected.binding), input.target.triple);
  return { files, binding: expected.binding, inventory, archiveSha256: sha256(archive) };
}
