import assert from 'node:assert/strict';
import { createHash, generateKeyPairSync, sign } from 'node:crypto';
import test from 'node:test';
import { gzipSync } from 'node:zlib';
import { captureNpmPackage } from '../scripts/distribution/npm-materials.mjs';

const BLOCK = 512;

function hash(algorithm, input, encoding = 'hex') {
  return createHash(algorithm).update(input).digest(encoding);
}

function octal(header, offset, width, value) {
  header.write(`${value.toString(8).padStart(width - 1, '0')}\0`, offset, width, 'ascii');
}

function tarHeader(path, mode, data, type = 48) {
  const header = Buffer.alloc(BLOCK);
  header.write(path, 0, 100, 'ascii');
  octal(header, 100, 8, mode);
  octal(header, 108, 8, 0);
  octal(header, 116, 8, 0);
  octal(header, 124, 12, data.length);
  octal(header, 136, 12, 1);
  header.fill(32, 148, 156);
  header[156] = type;
  if (type === 50) header.write('package/target', 157, 100, 'ascii');
  header.write('ustar\0', 257, 'ascii');
  header.write('00', 263, 'ascii');
  const checksum = header.reduce((sum, byte) => sum + byte, 0);
  header.write(`${checksum.toString(8).padStart(6, '0')}\0 `, 148, 8, 'ascii');
  return header;
}

function tar(files, type = 48) {
  const chunks = [];
  for (const file of files) {
    chunks.push(tarHeader(file.sourcePath, file.mode, file.data, type), file.data);
    const padding = (BLOCK - file.data.length % BLOCK) % BLOCK;
    if (padding) chunks.push(Buffer.alloc(padding));
  }
  chunks.push(Buffer.alloc(2 * BLOCK));
  return Buffer.concat(chunks);
}

function fixture({ type = 48, alterArchive } = {}) {
  const selected = [
    { sourcePath: 'package/LICENSE.txt', path: 'licenses/example-LICENSE.txt', mode: 0o644,
      data: Buffer.from('license\n') },
    { sourcePath: 'package/lib/example.js', path: 'lib/example.js', mode: 0o644,
      data: Buffer.from('export {};\n') },
  ];
  const unpackedSize = selected.reduce((sum, file) => sum + file.data.length, 0);
  let archive = gzipSync(tar(selected, type), { level: 9 });
  if (alterArchive) archive = alterArchive(archive);
  const integrity = `sha512-${hash('sha512', archive, 'base64')}`;
  const { privateKey, publicKey } = generateKeyPairSync('ec', { namedCurve: 'P-256' });
  const keyBytes = publicKey.export({ format: 'der', type: 'spki' });
  const keyid = `SHA256:${hash('sha256', keyBytes, 'base64').replace(/=$/, '')}`;
  const signature = sign('sha256', Buffer.from(`example@1.2.3:${integrity}`), privateKey)
    .toString('base64');
  const keys = Buffer.from(JSON.stringify({ keys: [{ expires: null, keyid,
    keytype: 'ecdsa-sha2-nistp256', scheme: 'ecdsa-sha2-nistp256',
    key: keyBytes.toString('base64') }] }));
  const metadataValue = { name: 'example', version: '1.2.3', dist: {
    tarball: 'https://registry.npmjs.org/example/-/example-1.2.3.tgz', integrity,
    shasum: hash('sha1', archive), fileCount: selected.length, unpackedSize,
    signatures: [{ keyid, sig: signature }],
  } };
  const metadata = Buffer.from(JSON.stringify(metadataValue));
  const descriptor = {
    name: 'example', version: '1.2.3',
    metadata: { url: 'https://registry.npmjs.org/example/1.2.3', size: metadata.length,
      sha256: hash('sha256', metadata) },
    tarball: { url: metadataValue.dist.tarball, size: archive.length,
      sha256: hash('sha256', archive), integrity, sha1: metadataValue.dist.shasum,
      fileCount: selected.length, unpackedSize },
    signature: { keyid, sig: signature },
    files: selected.map(file => ({ sourcePath: file.sourcePath, path: file.path, mode: file.mode,
      size: file.data.length, sha256: hash('sha256', file.data) })),
  };
  const keysDescriptor = { url: 'https://registry.npmjs.org/-/npm/v1/keys', size: keys.length,
    sha256: hash('sha256', keys) };
  return { descriptor, input: { metadata, keys, archive }, keysDescriptor, selected };
}

test('captures only authenticated selected npm members', () => {
  const value = fixture();
  const captured = captureNpmPackage(value.descriptor, value.input, value.keysDescriptor);
  assert.deepEqual(captured, value.selected.map(file => ({
    path: file.path, mode: file.mode, data: file.data,
  })));
});

test('rejects a link even when metadata, integrity, and signature authenticate its archive', () => {
  const value = fixture({ type: 50 });
  assert.throws(() => captureNpmPackage(value.descriptor, value.input, value.keysDescriptor),
    /npm tar ordinary file/);
});

test('rejects trailing gzip ambiguity after authenticating the altered bytes', () => {
  const value = fixture({ alterArchive: archive => Buffer.concat([archive, Buffer.from([0])]) });
  assert.throws(() => captureNpmPackage(value.descriptor, value.input, value.keysDescriptor),
    /npm gzip/);
});

test('rejects metadata with a different registry signature', () => {
  const value = fixture();
  const metadata = JSON.parse(value.input.metadata);
  metadata.dist.signatures[0].sig = metadata.dist.signatures[0].sig.replace(/^./, 'A');
  value.input.metadata = Buffer.from(JSON.stringify(metadata));
  value.descriptor.metadata = { ...value.descriptor.metadata, size: value.input.metadata.length,
    sha256: hash('sha256', value.input.metadata) };
  assert.throws(() => captureNpmPackage(value.descriptor, value.input, value.keysDescriptor),
    /npm metadata identity/);
});
