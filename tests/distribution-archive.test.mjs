import test from 'node:test';
import { validateArchiveRuntime } from '../scripts/distribution/runtime.mjs';
import assert from 'node:assert/strict';
import { bytes, orderedPaths, parseCanonical, portablePath } from '../scripts/distribution/canonical.mjs';
import { encodeTar, decodeTar } from '../scripts/distribution/archive-tar.mjs';
import { crc32, encodeZip, decodeZip } from '../scripts/distribution/archive-zip.mjs';

const root = 'zryna-0.2.0-x86_64-unknown-linux-gnu';
const files = [
  { path: 'LICENSE', mode: 0o644, data: Buffer.from('license\n') },
  { path: 'bin/zryna', mode: 0o755, data: Buffer.from('compiler') },
  { path: 'lib/zryna/bootstrap/node_modules/@typescript/old/lib/typescript.js',
    mode: 0o644, data: Buffer.from('provider') },
];

test('archive runtime requires the exact upstream compressor build on a qualified builder', () => {
  const runtime = { node: '22.22.1', zlib: '1.3.1-e00f703', platform: 'linux', architecture: 'x64' };
  validateArchiveRuntime(runtime);
  validateArchiveRuntime({ ...runtime, platform: 'win32' });
  for (const change of [{ node: '24.19.0' }, { zlib: '1.3.1' },
    { architecture: 'arm64' }, { platform: 'darwin' }]) {
    assert.throws(() => validateArchiveRuntime({ ...runtime, ...change }), /archive/);
  }
});

test('canonical record rejects duplicate keys, non-UTF8 and excessive nesting', () => {
  assert.deepEqual(parseCanonical(bytes({ b: 2, a: 1 })), { a: 1, b: 2 });
  for (const input of [Buffer.from('{"a":1,"a":1}\n'), Buffer.from([255]),
    Buffer.from(`${'['.repeat(13)}0${']'.repeat(13)}\n`), Buffer.from('{ "a":1}\n')]) {
    assert.throws(() => parseCanonical(input));
  }
});

test('portable paths reject traversal, platform aliases and prefix collisions', () => {
  for (const path of ['../a', '/a', 'C:/a', 'a\\b', 'a:b', 'a.', 'a ', 'NUL.txt', 'COM1',
    'a//b', 'é', 'a/'.repeat(12) + 'b', 'a'.repeat(201)]) assert.throws(() => portablePath(path));
  portablePath('a'.repeat(200));
  portablePath(Array(12).fill('a').join('/'));
  for (const paths of [['a', 'a'], ['A/x', 'a/y'], ['A', 'a'], ['a', 'a/b'], ['b', 'a']]) {
    assert.throws(() => orderedPaths(paths));
  }
});

test('ZIP CRC uses the standard independent check vector', () => {
  assert.equal(crc32(Buffer.from('123456789')), 0xcbf43926);
});

for (const kind of ['tar', 'zip']) {
  const encode = kind === 'tar' ? value => encodeTar(root, value, 1234567890)
    : value => encodeZip(root, value);
  const decode = kind === 'tar' ? value => decodeTar(value, root, 1234567890)
    : value => decodeZip(value, root, ['bin/zryna']);
  test(`${kind} is deterministic and retains exact file bytes and modes`, async () => {
    const archive = await encode(files);
    assert.deepEqual(await decode(archive), files);
    assert.deepEqual(await encode(files), archive);
  });
  test(`${kind} rejects truncation, appended data and an unexpected root`, async () => {
    const archive = await encode(files);
    await assert.rejects(async () => decode(archive.subarray(0, archive.length - 1)));
    await assert.rejects(async () => decode(Buffer.concat([archive, Buffer.from('hidden')])));
    const other = kind === 'tar' ? await encodeTar('other', files, 1234567890) : encodeZip('other', files);
    await assert.rejects(async () => decode(other));
  });
}

test('ZIP rejects a central-directory/local-header disagreement and executable mode tampering', () => {
  const original = encodeZip(root, files);
  const signature = Buffer.from([0x50, 0x4b, 0x01, 0x02]);
  const central = original.indexOf(signature);
  for (const field of [42, 38, 16]) {
    const changed = Buffer.from(original);
    changed[central + field] ^= 1;
    assert.throws(() => decodeZip(changed, root, ['bin/zryna']));
  }
});
