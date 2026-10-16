import { gunzipSync } from 'node:zlib';
import { bytes, exactKeys, requireValue, sha256 } from '../../distribution/canonical.mjs';
import { captureTarGzipMembers } from '../../distribution/npm-materials.mjs';
import { captureCompilerCacheTree } from './material-cache-files.mjs';

// Called only AFTER the established complete gzip/tar parser authenticates all framing/types/paths.
function authenticatedInventory(archive) {
  const tar = gunzipSync(archive, { maxOutputLength: 64 * 1024 * 1024 });
  const result = new Map();
  let offset = 0;
  let longName = null;
  function string(header, start, width) {
    return header.subarray(start, start + width).toString('ascii').split('\0')[0];
  }
  while (offset + 512 <= tar.length) {
    const header = tar.subarray(offset, offset + 512);
    if (header.every(byte => byte === 0)) break;
    const size = Number.parseInt(string(header, 124, 12).trim(), 8);
    const data = tar.subarray(offset + 512, offset + 512 + size);
    if (header[156] === 76) longName = data.subarray(0, -1).toString('ascii');
    else {
      const prefix = string(header, 345, 155);
      const name = string(header, 0, 100);
      const path = longName ?? (prefix ? `${prefix}/${name}` : name);
      result.set(path.slice(path.indexOf('/') + 1), {
        mode: Number.parseInt(string(header, 100, 8).trim(), 8) & 0o777,
        bytes: size, sha256: sha256(data),
      });
      longName = null;
    }
    offset += 512 + Math.ceil(size / 512) * 512;
  }
  return result;
}

export function validateCompilerCrateCache({ identity, archive, cacheRoot }, expectedSha256) {
  requireValue(typeof identity === 'string' && /^[A-Za-z0-9_-]+-[0-9][A-Za-z0-9+._-]*$/.test(identity)
    && Buffer.isBuffer(archive) && archive.length > 0 && archive.length <= 64 * 1024 * 1024
    && sha256(archive) === expectedSha256, 'compiler original crate identity');
  const tree = captureCompilerCacheTree(cacheRoot);
  const captured = tree.files;
  const checksum = captured.get('.cargo-checksum.json');
  const marker = captured.get('.cargo-ok');
  captured.delete('.cargo-checksum.json');
  captured.delete('.cargo-ok');
  requireValue(!marker || marker.data.equals(Buffer.from('{"v":1}')), 'compiler Cargo cache marker');
  requireValue(captured.size > 0 && captured.size <= 4096, 'compiler cache source inventory');
  const files = [...captured.values()].sort((a, b) => a.path < b.path ? -1 : 1);
  const manifest = captured.get('Cargo.toml');
  requireValue(manifest && manifest.data.length > 0, 'compiler crate manifest missing');
  // This established parser authenticates ALL framing/types/paths before inventory decoding.
  captureTarGzipMembers(archive, { root: identity, size: archive.length, sha256: expectedSha256,
    files: [{ sourcePath: 'Cargo.toml', path: 'source/Cargo.toml',
      mode: manifest.mode, size: manifest.data.length, sha256: sha256(manifest.data) }] });
  const inventory = authenticatedInventory(archive);
  requireValue(inventory.size === files.length && files.every(file => {
    const expected = inventory.get(file.path);
    return expected && (file.mode & ~expected.mode) === 0 && expected.bytes === file.data.length
      && expected.sha256 === sha256(file.data);
  }), 'compiler original archive complete cache correspondence');
  const directories = new Set();
  for (const path of inventory.keys()) {
    const segments = path.split('/');
    for (let index = 1; index < segments.length; index++) {
      directories.add(segments.slice(0, index).join('/'));
    }
  }
  requireValue(bytes([...directories].sort()).equals(bytes(tree.directories)),
    'compiler original archive complete cache directories');
  const hashes = Object.fromEntries(files.map(file => [file.path, sha256(file.data)]));
  if (checksum) {
    requireValue(checksum.data.length <= 2 * 1024 * 1024, 'compiler cache checksum byte budget');
    const parsed = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(checksum.data));
    exactKeys(parsed, ['files', 'package']);
    requireValue(parsed.package === expectedSha256 && bytes(parsed.files).equals(bytes(hashes)),
      'compiler cache checksum matches original source inventory');
  }
  return { identity, crateSha256: expectedSha256, sourceFiles: files.length,
    sourceInventorySha256: sha256(bytes(files.map(file => ({ path: file.path,
      bytes: file.data.length, sha256: hashes[file.path], mode: file.mode })))),
    checksumPresent: Boolean(checksum) };
}
