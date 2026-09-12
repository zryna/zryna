import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import test from 'node:test';
import { gzipSync } from 'node:zlib';
import { captureTarGzipMembers } from '../scripts/distribution/npm-materials.mjs';
import { nodeExpandedArchive, nodeMaterialMembers }
  from '../scripts/distribution/node-materials.mjs';
import { rustMaterials } from '../scripts/distribution/rust-materials.mjs';

const BLOCK = 512;

function sha256(input) {
  return createHash('sha256').update(input).digest('hex');
}

function octal(header, offset, width, value) {
  header.write(`${value.toString(8).padStart(width - 1, '0')}\0`, offset, width, 'ascii');
}

function header(path, mode, size, type = 48, link = '') {
  const value = Buffer.alloc(BLOCK);
  value.write(path, 0, 100, 'ascii');
  octal(value, 100, 8, mode);
  octal(value, 108, 8, 0);
  octal(value, 116, 8, 0);
  octal(value, 124, 12, size);
  octal(value, 136, 12, 1);
  value.fill(32, 148, 156);
  value[156] = type;
  value.write(link, 157, 100, 'ascii');
  value.write('ustar ', 257, 6, 'ascii');
  value.write('00', 263, 2, 'ascii');
  const checksum = value.reduce((sum, byte) => sum + byte, 0);
  value.write(`${checksum.toString(8).padStart(6, '0')}\0 `, 148, 8, 'ascii');
  return value;
}

function entry(path, data, type = 48, link = '') {
  const padding = (BLOCK - data.length % BLOCK) % BLOCK;
  return [header(path, 0o644, data.length, type, link), data, Buffer.alloc(padding)];
}

function namedGzip(input) {
  const gzip = gzipSync(input, { level: 9 });
  const prefix = Buffer.from(gzip.subarray(0, 10));
  prefix[3] |= 8;
  return Buffer.concat([prefix, Buffer.from('fixture.crate\0'), gzip.subarray(10)]);
}

function fixture({ corruptPadding = false, type = 48, trailing = false } = {}) {
  const root = 'example-1.0.0';
  const sourcePath = `licenses/${'long-'.repeat(18)}LICENSE`;
  const archivePath = `${root}/${sourcePath}`;
  const data = Buffer.from('license\n');
  const longName = Buffer.from(`${archivePath}\0`);
  const raw = Buffer.concat([
    ...entry('././@LongLink', longName, 76),
    ...entry('placeholder', data, type, type === 49 || type === 50 ? `${root}/target` : ''),
    ...entry(`${root}/EMPTY`, Buffer.alloc(0)),
    Buffer.alloc(2 * BLOCK),
  ]);
  if (corruptPadding) raw[raw.indexOf(data) + data.length] = 1;
  let archive = namedGzip(raw);
  if (trailing) archive = Buffer.concat([archive, Buffer.from([0])]);
  return { archive, descriptor: {
    root, size: archive.length, sha256: sha256(archive), files: [{ sourcePath,
      path: 'licenses/example-LICENSE', mode: 0o644, size: data.length,
      sha256: sha256(data) }],
  }, data };
}

test('captures an ordinary crate member through optional gzip and GNU long-name framing', () => {
  const value = fixture();
  assert.deepEqual(captureTarGzipMembers(value.archive, value.descriptor), [{
    path: 'licenses/example-LICENSE', mode: 0o644, data: value.data,
  }]);
});

test('rejects linked crate members even when their bytes match', () => {
  for (const type of [49, 50]) {
    const value = fixture({ type });
    assert.throws(() => captureTarGzipMembers(value.archive, value.descriptor),
      /npm tar ordinary file/);
  }
});

test('rejects trailing bytes after a complete crate gzip stream', () => {
  const value = fixture({ trailing: true });
  assert.throws(() => captureTarGzipMembers(value.archive, value.descriptor), /npm gzip/);
});

test('rejects nonzero crate tar padding', () => {
  const value = fixture({ corruptPadding: true });
  assert.throws(() => captureTarGzipMembers(value.archive, value.descriptor),
    /npm tar padding/);
});

test('publishes exact Node member and expanded Linux tar requirements', () => {
  assert.deepEqual(nodeExpandedArchive('x86_64-unknown-linux-gnu'), {
    size: 203161600,
    sha256: 'd9bd21210a6aaf6d02d35f739d02bb05ecda8c63deb1abb0150c288c3263fb48',
  });
  assert.equal(nodeExpandedArchive('x86_64-pc-windows-msvc'), null);
  for (const target of ['x86_64-unknown-linux-gnu', 'x86_64-pc-windows-msvc']) {
    const members = nodeMaterialMembers(target);
    assert.equal(members.length, 2);
    assert.ok(members.every(member => member.type === 'ordinary'));
  }
});

test('pins the complete target-specific Rust capture sets and Wasmtime license fanout', () => {
  const linux = rustMaterials('x86_64-unknown-linux-gnu');
  const windows = rustMaterials('x86_64-pc-windows-msvc');
  assert.equal(linux.length, 143);
  assert.equal(windows.length, 151);
  const upstream = 'https://raw.githubusercontent.com/bytecodealliance/wasmtime/'
    + '7bac2c2775808aaec5d4aa5627a5e447b51102cf/LICENSE';
  const destinations = new Set([...linux, ...windows].flatMap(record => record.files)
    .filter(file => file.origin === upstream).map(file => file.path));
  assert.equal(destinations.size, 13);
});
