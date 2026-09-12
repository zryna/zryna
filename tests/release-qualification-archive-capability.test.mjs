import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { encodeZip } from '../scripts/distribution/archive-zip.mjs';
import { sha256 } from '../scripts/distribution/canonical.mjs';
import { createQualificationArchiveCapability } from '../scripts/distribution-release/qualification-archive-capability.mjs';

const BLOCK = 512;

function tarHeader(path, mode, data, type = 48) {
  const header = Buffer.alloc(BLOCK);
  const octal = (offset, width, value) => {
    header.write(`${value.toString(8).padStart(width - 1, '0')}\0`, offset, width, 'ascii');
  };
  header.write(path, 0, 100, 'ascii');
  octal(100, 8, mode);
  octal(108, 8, 0);
  octal(116, 8, 0);
  octal(124, 12, data.length);
  octal(136, 12, 1);
  header.fill(32, 148, 156);
  header[156] = type;
  header.write('ustar\0', 257, 'ascii');
  header.write('00', 263, 'ascii');
  const checksum = header.reduce((sum, byte) => sum + byte, 0);
  header.write(`${checksum.toString(8).padStart(6, '0')}\0 `, 148, 8, 'ascii');
  return header;
}

function tar(path, mode, data, type = 48) {
  const padding = (BLOCK - data.length % BLOCK) % BLOCK;
  return Buffer.concat([tarHeader(path, mode, data, type), data, Buffer.alloc(padding + 2 * BLOCK)]);
}

function root(t) {
  const value = mkdtempSync(join(tmpdir(), 'zryna-qualification-capability-'));
  t.after(() => rmSync(value, { recursive: true, force: true }));
  return value;
}

function member(sourcePath, path, mode, data) {
  return { sourcePath, path, type: 'ordinary', mode, size: data.length, sha256: sha256(data) };
}

test('reads one ordinary member only after complete ZIP header and digest validation', async (t) => {
  const workingRoot = root(t);
  const data = Buffer.from('node executable');
  const sourcePath = 'node-v22.22.1-win-x64/node.exe';
  const archive = encodeZip('node-v22.22.1-win-x64', [
    { path: 'LICENSE', mode: 0o644, data: Buffer.from('license') },
    { path: 'node.exe', mode: 0o755, data },
  ]);
  const capability = createQualificationArchiveCapability({
    target: 'x86_64-pc-windows-msvc', nativeTools: [], workingRoot,
    system: { platform: 'win32', inspectFile: (path) => {
      assert.equal(path, workingRoot);
      return { isDirectory: () => true, isSymbolicLink: () => false };
    } },
  });
  const selected = await capability.readOrdinaryMember({
    format: 'zip', container: archive, containerSha256: sha256(archive),
    expected: member(sourcePath, 'runtime/node/node.exe', 0o755, data),
  });
  assert.deepEqual(selected, data);
  const changed = Buffer.from(archive);
  changed[changed.length - 23] ^= 1;
  await assert.rejects(() => capability.readOrdinaryMember({
    format: 'zip', container: changed, containerSha256: sha256(changed),
    expected: member(sourcePath, 'runtime/node/node.exe', 0o755, data),
  }));
});

test('expands XZ through one fixed observed executable and validates a tar ordinary member', async (t) => {
  const workingRoot = root(t);
  const xzPath = join(workingRoot, 'xz');
  const xzBytes = Buffer.from('observed xz executable');
  writeFileSync(xzPath, xzBytes);
  const data = Buffer.from('linux node executable');
  const sourcePath = 'node-v22.22.1-linux-x64/bin/node';
  const expanded = tar(sourcePath, 0o755, data);
  const archive = Buffer.from('authenticated xz archive');
  let calls = 0;
  const capability = createQualificationArchiveCapability({
    target: 'x86_64-unknown-linux-gnu', workingRoot,
    nativeTools: [{ name: 'xz', version: 'xz 5.6.1', origin: 'ubuntu-24.04:/usr/bin/xz',
      size: xzBytes.length, sha256: sha256(xzBytes), observationEvidenceSha256: 'a'.repeat(64) }],
    spawn(path, args, options) {
      calls += 1;
      assert.equal(path, xzPath);
      assert.deepEqual(args, ['-dc', '--single-stream']);
      assert.deepEqual(options.env, {});
      assert.equal(options.shell, false);
      return { status: 0, signal: null, stdout: expanded, stderr: Buffer.alloc(0) };
    },
    system: { platform: 'linux', xzPath,
      inspectFile(path) {
        if (path === workingRoot) {
          return { isDirectory: () => true, isSymbolicLink: () => false };
        }
        return { isFile: () => true, isSymbolicLink: () => false, size: xzBytes.length };
      },
      readFile: () => xzBytes,
    },
  });
  const container = await capability.expandXz({
    archive, expected: { size: expanded.length, sha256: sha256(expanded) },
  });
  const selected = await capability.readOrdinaryMember({
    format: 'tar', container, containerSha256: sha256(container),
    expected: member(sourcePath, 'runtime/node/bin/node', 0o755, data),
  });
  assert.equal(calls, 1);
  assert.deepEqual(selected, data);

  const link = tar(sourcePath, 0o755, data, 50);
  await assert.rejects(() => capability.readOrdinaryMember({
    format: 'tar', container: link, containerSha256: sha256(link),
    expected: member(sourcePath, 'runtime/node/bin/node', 0o755, data),
  }), /non-ordinary/);
});
