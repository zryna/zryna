import { createHash, createPublicKey, verify } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { inflateRawSync } from 'node:zlib';
import { exactKeys, orderedPaths, parseCanonical, portablePath, requireValue, sha256 }
  from './canonical.mjs';

const RECORD_BYTES = readFileSync(new URL('./materials-typescript-v1.json', import.meta.url));
const RECORD_SHA256 = 'f43639bd9decf44d95a094caad029a572851e4399c889da6071d4425203da3f6';
const RECORD = parseCanonical(RECORD_BYTES);
const BLOCK = 512;
const MAX_METADATA = 256 * 1024;
const MAX_KEYS = 64 * 1024;
const MAX_ARCHIVE = 64 * 1024 * 1024;
const MAX_EXPANDED = 64 * 1024 * 1024;
const MAX_FILES = 512;
const MAX_TAR_ENTRIES = 4096;

function digest(algorithm, input, encoding = 'hex') {
  return createHash(algorithm).update(input).digest(encoding);
}

function base64(value, label) {
  requireValue(typeof value === 'string' && /^[A-Za-z0-9+/]+={0,2}$/.test(value), label);
  const decoded = Buffer.from(value, 'base64');
  requireValue(decoded.length >= 1 && decoded.toString('base64') === value, label);
  return decoded;
}

function descriptorMatches(descriptor, input, maximum, label) {
  requireValue(Buffer.isBuffer(input) && input.length >= 1 && input.length <= maximum
    && descriptor.size === input.length && descriptor.sha256 === sha256(input), `${label} bytes`);
}

function officialUrl(value) {
  let url;
  try {
    url = new URL(value);
  } catch {
    requireValue(false, 'npm descriptor URL');
  }
  requireValue(url.protocol === 'https:' && url.hostname === 'registry.npmjs.org'
    && url.port === '' && url.username === '' && url.password === ''
    && url.search === '' && url.hash === '', 'npm descriptor URL');
}

function validateDescriptor(descriptor, keysDescriptor) {
  exactKeys(descriptor, ['name', 'version', 'metadata', 'tarball', 'signature', 'files']);
  exactKeys(keysDescriptor, ['url', 'size', 'sha256']);
  exactKeys(descriptor.metadata, ['url', 'size', 'sha256']);
  exactKeys(descriptor.tarball, [
    'url', 'size', 'sha256', 'integrity', 'sha1', 'fileCount', 'unpackedSize',
  ]);
  exactKeys(descriptor.signature, ['keyid', 'sig']);
  officialUrl(keysDescriptor.url);
  requireValue(keysDescriptor.url === 'https://registry.npmjs.org/-/npm/v1/keys'
    && Number.isSafeInteger(keysDescriptor.size) && keysDescriptor.size >= 1
    && keysDescriptor.size <= MAX_KEYS && /^[0-9a-f]{64}$/.test(keysDescriptor.sha256),
  'npm keys descriptor');
  requireValue(/^(@[a-z0-9._-]+\/)?[a-z0-9._-]+$/.test(descriptor.name)
    && /^[0-9]+\.[0-9]+\.[0-9]+$/.test(descriptor.version), 'npm package identity');
  const encodedName = descriptor.name.replace('/', '%2F');
  const baseName = descriptor.name.split('/').at(-1);
  requireValue(descriptor.metadata.url
    === `https://registry.npmjs.org/${encodedName}/${descriptor.version}`
    && descriptor.tarball.url
    === `https://registry.npmjs.org/${descriptor.name}/-/${baseName}-${descriptor.version}.tgz`,
  'npm package URLs');
  for (const [value, maximum] of [[descriptor.metadata, MAX_METADATA],
    [descriptor.tarball, MAX_ARCHIVE]]) {
    officialUrl(value.url);
    requireValue(Number.isSafeInteger(value.size) && value.size >= 1 && value.size <= maximum
      && /^[0-9a-f]{64}$/.test(value.sha256), 'npm artifact descriptor');
  }
  requireValue(/^[0-9a-f]{40}$/.test(descriptor.tarball.sha1)
    && /^sha512-[A-Za-z0-9+/]{86}==$/.test(descriptor.tarball.integrity)
    && Number.isSafeInteger(descriptor.tarball.fileCount) && descriptor.tarball.fileCount >= 1
    && descriptor.tarball.fileCount <= MAX_FILES
    && Number.isSafeInteger(descriptor.tarball.unpackedSize)
    && descriptor.tarball.unpackedSize >= 1 && descriptor.tarball.unpackedSize <= MAX_EXPANDED,
  'npm tarball descriptor');
  requireValue(/^SHA256:[A-Za-z0-9+/]{43}$/.test(descriptor.signature.keyid)
    && /^[A-Za-z0-9+/]+={0,2}$/.test(descriptor.signature.sig), 'npm signature descriptor');
  requireValue(Array.isArray(descriptor.files) && descriptor.files.length >= 1
    && descriptor.files.length <= descriptor.tarball.fileCount, 'npm selected file descriptors');
  let previous = '';
  for (const file of descriptor.files) {
    exactKeys(file, ['sourcePath', 'path', 'mode', 'size', 'sha256']);
    requireValue(file.sourcePath > previous && file.sourcePath.startsWith('package/')
      && [0o644, 0o755].includes(file.mode) && Number.isSafeInteger(file.size) && file.size >= 1
      && file.size <= MAX_EXPANDED && /^[0-9a-f]{64}$/.test(file.sha256),
    'npm selected file descriptor');
    previous = file.sourcePath;
  }
  orderedPaths(descriptor.files.map(file => file.path).sort());
}

function jsonBytes(input, label) {
  try {
    return JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(input));
  } catch {
    requireValue(false, `${label} JSON`);
  }
}

function crc32(input) {
  let value = 0xffffffff;
  for (const byte of input) {
    value ^= byte;
    for (let bit = 0; bit < 8; bit++) value = (value >>> 1) ^ (0xedb88320 & -(value & 1));
  }
  return (value ^ 0xffffffff) >>> 0;
}

function gunzipSingle(input) {
  requireValue(input.length >= 18 && input[0] === 0x1f && input[1] === 0x8b
    && input[2] === 8 && (input[3] & 0xe0) === 0, 'npm gzip header');
  const flags = input[3];
  let offset = 10;
  if (flags & 4) {
    requireValue(offset + 2 <= input.length - 8, 'npm gzip extra header');
    const size = input.readUInt16LE(offset);
    offset += 2;
    requireValue(size <= 4096 && offset + size <= input.length - 8, 'npm gzip extra header');
    offset += size;
  }
  for (const flag of [8, 16]) {
    if (flags & flag) {
      const end = input.indexOf(0, offset);
      requireValue(end >= offset && end - offset <= 1024 && end < input.length - 8,
        'npm gzip text header');
      offset = end + 1;
    }
  }
  if (flags & 2) {
    requireValue(offset + 2 <= input.length - 8
      && input.readUInt16LE(offset) === (crc32(input.subarray(0, offset)) & 0xffff),
    'npm gzip header checksum');
    offset += 2;
  }
  const compressed = input.subarray(offset, input.length - 8);
  let result;
  try {
    result = inflateRawSync(compressed, { info: true, maxOutputLength: MAX_EXPANDED });
  } catch {
    requireValue(false, 'npm gzip payload');
  }
  const payload = result.buffer;
  requireValue(result.engine.bytesWritten === compressed.length
    && input.readUInt32LE(input.length - 8) === crc32(payload)
    && input.readUInt32LE(input.length - 4) === payload.length, 'npm gzip framing');
  return payload;
}

function ascii(header, offset, width, label) {
  const field = header.subarray(offset, offset + width);
  const end = field.indexOf(0);
  const value = field.subarray(0, end < 0 ? width : end);
  requireValue(value.every(byte => byte >= 32 && byte <= 126), `npm tar ${label}`);
  return value.toString('ascii');
}

function octal(header, offset, width, label) {
  const field = ascii(header, offset, width, label).trim();
  requireValue(/^[0-7]+$/.test(field), `npm tar ${label}`);
  return Number.parseInt(field, 8);
}

function headerChecksum(header) {
  let sum = 0;
  for (let index = 0; index < header.length; index++) {
    sum += index >= 148 && index < 156 ? 32 : header[index];
  }
  return sum;
}

function parseTar(input, descriptor, root = 'package', allowedModes = new Set([0o644, 0o755])) {
  requireValue(/^[A-Za-z0-9@][A-Za-z0-9@+._-]*$/.test(root), 'npm tar root');
  requireValue(input.length % BLOCK === 0, 'npm tar block framing');
  const files = [];
  const paths = new Set();
  let offset = 0;
  let unpacked = 0;
  let longName = null;
  let entries = 0;
  while (offset + BLOCK <= input.length) {
    const header = input.subarray(offset, offset + BLOCK);
    if (header.every(byte => byte === 0)) break;
    requireValue(['ustar', 'ustar '].includes(ascii(header, 257, 6, 'magic'))
      && headerChecksum(header) === octal(header, 148, 8, 'checksum'), 'npm tar header');
    const size = octal(header, 124, 12, 'size');
    requireValue(size <= MAX_EXPANDED && offset + BLOCK + size <= input.length
      && entries++ < MAX_TAR_ENTRIES, 'npm tar member');
    const data = input.subarray(offset + BLOCK, offset + BLOCK + size);
    const next = offset + BLOCK + Math.ceil(size / BLOCK) * BLOCK;
    requireValue(input.subarray(offset + BLOCK + size, next).every(byte => byte === 0),
      'npm tar padding');
    if (header[156] === 76) {
      requireValue(longName === null && size >= 2 && size <= 257 && data.at(-1) === 0
        && header.subarray(157, 257).every(byte => byte === 0)
        && data.subarray(0, -1).every(byte => byte >= 32 && byte <= 126),
      'npm tar long name');
      longName = data.subarray(0, -1).toString('ascii');
      offset = next;
      continue;
    }
    requireValue((header[156] === 0 || header[156] === 48)
      && header.subarray(157, 257).every(byte => byte === 0), 'npm tar ordinary file');
    const name = ascii(header, 0, 100, 'name');
    const prefix = ascii(header, 345, 155, 'prefix');
    const path = longName ?? (prefix ? `${prefix}/${name}` : name);
    longName = null;
    const segments = path.split('/');
    const folded = path.toLowerCase();
    requireValue(path.length <= 256 && segments[0] === root && segments.length <= 16
      && segments.slice(1).every(segment => /^[A-Za-z0-9@+._-]+$/.test(segment)
        && !/[. ]$/.test(segment)
        && !/^(con|prn|aux|nul|com[0-9]|lpt[0-9])(?:\.|$)/i.test(segment))
      && !paths.has(folded)
      && [...paths].every(existing => !existing.startsWith(`${folded}/`)
        && !folded.startsWith(`${existing}/`)), 'npm tar path');
    paths.add(folded);
    const rawMode = octal(header, 100, 8, 'mode');
    const mode = rawMode & 0o777;
    requireValue(size >= 0 && size <= MAX_EXPANDED
      && (rawMode === mode || rawMode === 0o100000 + mode)
      && (allowedModes === null || allowedModes.has(mode))
      && offset + BLOCK + size <= input.length, 'npm tar member');
    unpacked += size;
    requireValue(unpacked <= MAX_EXPANDED && files.length < MAX_TAR_ENTRIES,
      'npm tar expansion');
    files.push({ path, mode, data });
    offset = next;
  }
  requireValue((descriptor.fileCount === undefined || files.length === descriptor.fileCount)
    && (descriptor.unpackedSize === undefined || unpacked === descriptor.unpackedSize)
    && longName === null
    && offset + 2 * BLOCK <= input.length
    && input.subarray(offset).every(byte => byte === 0), 'npm tar closure');
  return files;
}

function validateRegistry(descriptor, keysDescriptor, metadata, keys, archive) {
  validateDescriptor(descriptor, keysDescriptor);
  descriptorMatches(descriptor.metadata, metadata, MAX_METADATA, 'npm metadata');
  descriptorMatches(keysDescriptor, keys, MAX_KEYS, 'npm registry keys');
  descriptorMatches(descriptor.tarball, archive, MAX_ARCHIVE, 'npm tarball');
  const document = jsonBytes(metadata, 'npm metadata');
  const keyDocument = jsonBytes(keys, 'npm keys');
  requireValue(document.name === descriptor.name && document.version === descriptor.version
    && document.dist?.tarball === descriptor.tarball.url
    && document.dist?.integrity === descriptor.tarball.integrity
    && document.dist?.shasum === descriptor.tarball.sha1
    && document.dist?.fileCount === descriptor.tarball.fileCount
    && document.dist?.unpackedSize === descriptor.tarball.unpackedSize
    && document.dist?.signatures?.length === 1
    && document.dist.signatures[0].keyid === descriptor.signature.keyid
    && document.dist.signatures[0].sig === descriptor.signature.sig,
  'npm metadata identity');
  requireValue(digest('sha512', archive, 'base64') === descriptor.tarball.integrity.slice(7)
    && digest('sha1', archive) === descriptor.tarball.sha1, 'npm tarball integrity');
  const matches = keyDocument.keys?.filter(key => key.keyid === descriptor.signature.keyid) ?? [];
  requireValue(matches.length === 1 && matches[0].expires === null
    && matches[0].keytype === 'ecdsa-sha2-nistp256'
    && matches[0].scheme === 'ecdsa-sha2-nistp256', 'npm signing key');
  let key;
  try {
    key = createPublicKey({ key: base64(matches[0].key, 'npm signing key encoding'),
      format: 'der', type: 'spki' });
  } catch {
    requireValue(false, 'npm signing key encoding');
  }
  const message = Buffer.from(`${descriptor.name}@${descriptor.version}:${descriptor.tarball.integrity}`);
  requireValue(verify('sha256', message, key, base64(descriptor.signature.sig, 'npm signature encoding')),
    'npm registry signature');
}

export function captureNpmPackage(descriptor, input, keysDescriptor = RECORD.keys) {
  exactKeys(input, ['metadata', 'keys', 'archive']);
  const { metadata, keys, archive } = input;
  validateRegistry(descriptor, keysDescriptor, metadata, keys, archive);
  const members = new Map(parseTar(gunzipSingle(archive), descriptor.tarball)
    .map(file => [file.path, file]));
  return descriptor.files.map(expected => {
    const selected = members.get(expected.sourcePath);
    requireValue(selected?.data.length === expected.size && sha256(selected.data) === expected.sha256,
      'npm selected member');
    return { path: expected.path, mode: expected.mode, data: Buffer.from(selected.data) };
  });
}

// This pure boundary is for already authenticated crate archives. It performs complete bounded
// gzip/tar framing and ordinary-member admission without fetching or extracting to a filesystem.
export function captureTarGzipMembers(archive, descriptor) {
  exactKeys(descriptor, ['root', 'size', 'sha256', 'files']);
  requireValue(Buffer.isBuffer(archive) && archive.length === descriptor.size
    && archive.length <= MAX_ARCHIVE && sha256(archive) === descriptor.sha256
    && /^[A-Za-z0-9@][A-Za-z0-9@+._-]*$/.test(descriptor.root)
    && Array.isArray(descriptor.files) && descriptor.files.length >= 1
    && descriptor.files.length <= MAX_FILES, 'upstream tar descriptor');
  const sourcePaths = new Set();
  for (const expected of descriptor.files) {
    exactKeys(expected, ['sourcePath', 'path', 'mode', 'size', 'sha256']);
    portablePath(`${descriptor.root}/${expected.sourcePath}`);
    requireValue(!sourcePaths.has(expected.sourcePath)
      && Number.isSafeInteger(expected.mode) && expected.mode >= 0 && expected.mode <= 0o777
      && Number.isSafeInteger(expected.size) && expected.size >= 1
      && expected.size <= MAX_EXPANDED && /^[0-9a-f]{64}$/.test(expected.sha256),
    'upstream selected descriptor');
    sourcePaths.add(expected.sourcePath);
  }
  orderedPaths(descriptor.files.map(file => file.path));
  const members = new Map(parseTar(gunzipSingle(archive), {}, descriptor.root, null)
    .map(file => [file.path, file]));
  return descriptor.files.map(expected => {
    const selected = members.get(`${descriptor.root}/${expected.sourcePath}`);
    requireValue(selected?.data.length === expected.size
      && sha256(selected.data) === expected.sha256, 'upstream selected member');
    return { path: expected.path, mode: expected.mode, data: Buffer.from(selected.data) };
  });
}

export function typeScriptMaterialDescriptors() {
  return structuredClone(RECORD);
}

export function captureTypeScriptMaterials(inputs) {
  requireValue(Array.isArray(inputs) && inputs.length === RECORD.packages.length, 'npm capture set');
  const files = RECORD.packages.flatMap((descriptor, index) =>
    captureNpmPackage(descriptor, inputs[index]));
  files.sort((left, right) => left.path < right.path ? -1 : left.path > right.path ? 1 : 0);
  orderedPaths(files.map(file => file.path));
  return files;
}

exactKeys(RECORD, ['format', 'keys', 'packages']);
requireValue(sha256(RECORD_BYTES) === RECORD_SHA256
  && RECORD.format === 'zryna.typescript-distribution-materials.v1'
  && RECORD.packages.length === 2, 'TypeScript material recipe identity');
