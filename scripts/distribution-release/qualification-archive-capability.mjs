import { spawnSync } from 'node:child_process';
import { lstatSync, readFileSync } from 'node:fs';
import { isAbsolute } from 'node:path';
import { inflateRawSync } from 'node:zlib';
import { crc32 } from '../distribution/archive-zip.mjs';
import { exactKeys, sha256 } from '../distribution/canonical.mjs';

const BLOCK = 512;
const MAX_TOOL = 64 * 1024 * 1024;
const MAX_CONTAINER = 512 * 1024 * 1024;
const XZ_PATH = '/usr/bin/xz';

function reject(message) {
  throw new Error(`R406-QUALIFICATION-ARCHIVE-CAPABILITY: ${message}`);
}

function expectedMember(value) {
  exactKeys(value, ['sourcePath', 'path', 'type', 'mode', 'size', 'sha256']);
  if (value.type !== 'ordinary' || !/^[ -~]{1,256}$/.test(value.sourcePath)
    || value.sourcePath.startsWith('/') || value.sourcePath.includes('\\')
    || value.sourcePath.split('/').some((part) => part === '' || part === '.' || part === '..')
    || ![0o644, 0o755].includes(value.mode) || !Number.isSafeInteger(value.size)
    || value.size < 1 || value.size > MAX_CONTAINER || !/^[0-9a-f]{64}$/.test(value.sha256)) {
    reject('ordinary member descriptor differs');
  }
  return value;
}

function tarAscii(header, offset, width) {
  const field = header.subarray(offset, offset + width);
  const end = field.indexOf(0);
  const value = field.subarray(0, end < 0 ? width : end);
  if (!value.every((byte) => byte >= 32 && byte <= 126)) reject('tar text field differs');
  return value.toString('ascii');
}

function tarOctal(header, offset, width) {
  const value = tarAscii(header, offset, width).trim();
  if (!/^[0-7]+$/.test(value)) reject('tar numeric field differs');
  return Number.parseInt(value, 8);
}

function tarChecksum(header) {
  let sum = 0;
  for (let index = 0; index < header.length; index += 1) {
    sum += index >= 148 && index < 156 ? 32 : header[index];
  }
  return sum;
}

function readTarMember(container, expected) {
  if (container.length % BLOCK !== 0) reject('tar block framing differs');
  let offset = 0;
  let selected;
  let closed = false;
  let pathOverride = false;
  while (offset + BLOCK <= container.length) {
    const header = container.subarray(offset, offset + BLOCK);
    if (header.every((byte) => byte === 0)) {
      if (offset + 2 * BLOCK > container.length
        || !container.subarray(offset).every((byte) => byte === 0)) reject('tar trailer differs');
      closed = true;
      break;
    }
    const checksum = tarOctal(header, 148, 8);
    if (checksum !== tarChecksum(header)
      || header.subarray(257, 262).toString('ascii') !== 'ustar') {
      reject('tar header checksum or format differs');
    }
    const name = tarAscii(header, 0, 100);
    const prefix = tarAscii(header, 345, 155);
    const path = prefix ? `${prefix}/${name}` : name;
    const size = tarOctal(header, 124, 12);
    const mode = tarOctal(header, 100, 8);
    const type = header[156];
    const end = offset + BLOCK + size;
    if (size > MAX_CONTAINER || end > container.length) reject('tar member framing differs');
    if (path === expected.sourcePath) {
      if (selected || pathOverride || ![0, 48].includes(type)
        || ![expected.mode, 0o100000 + expected.mode].includes(mode)
        || !header.subarray(157, 257).every((byte) => byte === 0)
        || size !== expected.size) {
        reject('selected tar member is missing, ambiguous, or non-ordinary');
      }
      selected = Buffer.from(container.subarray(offset + BLOCK, end));
    }
    pathOverride = [76, 120].includes(type);
    offset += BLOCK + Math.ceil(size / BLOCK) * BLOCK;
  }
  if (!closed || !selected || selected.length !== expected.size
    || sha256(selected) !== expected.sha256) {
    reject('selected tar member bytes differ');
  }
  return selected;
}

function zipName(bytes) {
  if (bytes.length < 1 || bytes.length > 256
    || !bytes.every((byte) => byte >= 32 && byte <= 126)) reject('ZIP member name differs');
  return bytes.toString('ascii');
}

function readZipMember(container, expected) {
  if (container.length < 22 || container.readUInt32LE(container.length - 22) !== 0x06054b50
    || container.readUInt16LE(container.length - 2) !== 0) reject('ZIP end record differs');
  const eocd = container.length - 22;
  const disk = container.readUInt16LE(eocd + 4);
  const centralDisk = container.readUInt16LE(eocd + 6);
  const diskEntries = container.readUInt16LE(eocd + 8);
  const entries = container.readUInt16LE(eocd + 10);
  const centralSize = container.readUInt32LE(eocd + 12);
  const centralOffset = container.readUInt32LE(eocd + 16);
  if (disk !== 0 || centralDisk !== 0 || entries < 1 || diskEntries !== entries
    || centralOffset + centralSize !== eocd) reject('ZIP central directory framing differs');
  let offset = centralOffset;
  let selected;
  for (let index = 0; index < entries; index += 1) {
    if (offset + 46 > eocd || container.readUInt32LE(offset) !== 0x02014b50) {
      reject('ZIP central entry differs');
    }
    const creator = container.readUInt16LE(offset + 4) >>> 8;
    const flags = container.readUInt16LE(offset + 8);
    const method = container.readUInt16LE(offset + 10);
    const checksum = container.readUInt32LE(offset + 16);
    const compressedSize = container.readUInt32LE(offset + 20);
    const size = container.readUInt32LE(offset + 24);
    const nameLength = container.readUInt16LE(offset + 28);
    const extraLength = container.readUInt16LE(offset + 30);
    const commentLength = container.readUInt16LE(offset + 32);
    const external = container.readUInt32LE(offset + 38);
    const localOffset = container.readUInt32LE(offset + 42);
    const next = offset + 46 + nameLength + extraLength + commentLength;
    if (next > eocd || compressedSize > MAX_CONTAINER || size > MAX_CONTAINER) {
      reject('ZIP central entry bounds differ');
    }
    const nameBytes = container.subarray(offset + 46, offset + 46 + nameLength);
    const name = zipName(nameBytes);
    if (name === expected.sourcePath) {
      const unixType = (external >>> 16) & 0xf000;
      const unixMode = (external >>> 16) & 0o777;
      const dosDirectory = (external & 0x10) !== 0;
      if (selected || name.endsWith('/') || dosDirectory || ![0, 8].includes(method)
        || (flags & ~0x808) !== 0 || size !== expected.size
        || (creator === 3 && (unixType !== 0x8000 || unixMode !== expected.mode))) {
        reject('selected ZIP member is missing, ambiguous, or non-ordinary');
      }
      if (localOffset + 30 > centralOffset || container.readUInt32LE(localOffset) !== 0x04034b50) {
        reject('selected ZIP local header differs');
      }
      const localFlags = container.readUInt16LE(localOffset + 6);
      const localMethod = container.readUInt16LE(localOffset + 8);
      const localChecksum = container.readUInt32LE(localOffset + 14);
      const localCompressed = container.readUInt32LE(localOffset + 18);
      const localSize = container.readUInt32LE(localOffset + 22);
      const localNameLength = container.readUInt16LE(localOffset + 26);
      const localExtraLength = container.readUInt16LE(localOffset + 28);
      const dataOffset = localOffset + 30 + localNameLength + localExtraLength;
      if (localFlags !== flags || localMethod !== method
        || !container.subarray(localOffset + 30, localOffset + 30 + localNameLength).equals(nameBytes)
        || dataOffset + compressedSize > centralOffset
        || ((flags & 0x8) === 0 && (localChecksum !== checksum
          || localCompressed !== compressedSize || localSize !== size))) {
        reject('selected ZIP local and central headers differ');
      }
      const compressed = container.subarray(dataOffset, dataOffset + compressedSize);
      try {
        if (method === 0) {
          selected = Buffer.from(compressed);
        } else {
          const result = inflateRawSync(compressed, { info: true, maxOutputLength: expected.size });
          if (result.engine.bytesWritten !== compressed.length) {
            reject('selected ZIP deflate framing differs');
          }
          selected = result.buffer;
        }
      } catch {
        reject('selected ZIP deflate stream differs');
      }
      if (selected.length !== size || crc32(selected) !== checksum) {
        reject('selected ZIP checksum differs');
      }
    }
    offset = next;
  }
  if (offset !== eocd || !selected || selected.length !== expected.size
    || sha256(selected) !== expected.sha256) reject('selected ZIP member bytes differ');
  return selected;
}

function authenticateXz(tool, path, inspectFile, readFile) {
  if (!tool) reject('XZ native tool observation differs');
  exactKeys(tool, ['name', 'version', 'origin', 'size', 'sha256', 'observationEvidenceSha256']);
  if (!isAbsolute(path) || tool.name !== 'xz' || !/^xz [ -~]{1,156}$/.test(tool.version)
    || tool.origin !== 'ubuntu-24.04:/usr/bin/xz'
    || !Number.isSafeInteger(tool.size) || tool.size < 1 || tool.size > MAX_TOOL
    || !/^[0-9a-f]{64}$/.test(tool.sha256)
    || !/^[0-9a-f]{64}$/.test(tool.observationEvidenceSha256)) {
    reject('XZ native tool observation differs');
  }
  const metadata = inspectFile(path);
  if (!metadata.isFile() || metadata.isSymbolicLink() || metadata.size !== tool.size) {
    reject('XZ executable file identity differs');
  }
  const bytes = readFile(path);
  if (bytes.length !== tool.size || sha256(bytes) !== tool.sha256) {
    reject('XZ executable bytes differ');
  }
}

export function createQualificationArchiveCapability({
  target, nativeTools, workingRoot, spawn = spawnSync,
  system = { platform: process.platform, xzPath: XZ_PATH,
    inspectFile: lstatSync, readFile: readFileSync },
}) {
  const expectedPlatform = target === 'x86_64-unknown-linux-gnu' ? 'linux' : 'win32';
  if (system.platform !== expectedPlatform || !isAbsolute(workingRoot ?? '')) {
    reject('archive capability target or controlled working root differs');
  }
  const working = system.inspectFile(workingRoot);
  if (!working.isDirectory() || working.isSymbolicLink()) {
    reject('archive capability working root differs');
  }
  if (!Array.isArray(nativeTools)) reject('native tool observations differ');
  const xz = target === 'x86_64-unknown-linux-gnu'
    ? nativeTools.filter(({ name }) => name === 'xz') : [];
  if (xz.length > 1 || (target === 'x86_64-unknown-linux-gnu' && xz.length !== 1)) {
    reject('exactly one XZ native tool observation is required');
  }
  return Object.freeze({
    async expandXz({ archive, expected }) {
      if (target !== 'x86_64-unknown-linux-gnu' || !Buffer.isBuffer(archive)
        || archive.length < 1 || archive.length > MAX_CONTAINER) reject('XZ input differs');
      exactKeys(expected, ['size', 'sha256']);
      if (!Number.isSafeInteger(expected.size) || expected.size < 1 || expected.size > MAX_CONTAINER
        || !/^[0-9a-f]{64}$/.test(expected.sha256)) reject('XZ output descriptor differs');
      authenticateXz(xz[0], system.xzPath, system.inspectFile, system.readFile);
      const result = spawn(system.xzPath, ['-dc', '--single-stream'], {
        cwd: workingRoot, env: {}, input: archive, encoding: null, shell: false,
        windowsHide: true, timeout: 60_000, maxBuffer: expected.size + 1,
      });
      authenticateXz(xz[0], system.xzPath, system.inspectFile, system.readFile);
      if (result.error || result.status !== 0 || result.signal !== null
        || !Buffer.isBuffer(result.stdout) || result.stdout.length !== expected.size
        || sha256(result.stdout) !== expected.sha256 || !Buffer.isBuffer(result.stderr)
        || result.stderr.length !== 0) reject('XZ command result differs');
      return Buffer.from(result.stdout);
    },
    async readOrdinaryMember({ format, container, containerSha256, expected }) {
      expectedMember(expected);
      if (!['tar', 'zip'].includes(format) || !Buffer.isBuffer(container)
        || container.length < 1 || container.length > MAX_CONTAINER
        || !/^[0-9a-f]{64}$/.test(containerSha256)
        || sha256(container) !== containerSha256) reject('archive container identity differs');
      return format === 'tar'
        ? readTarMember(container, expected) : readZipMember(container, expected);
    },
  });
}
