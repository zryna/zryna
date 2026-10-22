import { gzipSync } from 'node:zlib';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { REGISTRY } from './material-lock.mjs';

export const CRATE_SHA = 'f2032f911046de80f0a198e0901378627c33f59ea0ac00e363d481118bd70a53';
export function graphFixture() {
  const sourceRoot = '/synthetic-source';
  const app = { id: 'app', name: 'zryna-playground-compiler', version: '0.1.0', source: null,
    manifest_path: `${sourceRoot}/apps/zryna-playground-compiler/Cargo.toml`,
    targets: [{ name: 'zryna-playground-compiler', kind: ['bin'] }] };
  const crate = { id: 'crate', name: 'autocfg', version: '1.5.1', source: REGISTRY,
    manifest_path: '/synthetic-cache/autocfg-1.5.1/Cargo.toml' };
  const metadata = { version: 1, workspace_root: sourceRoot, workspace_members: ['app'],
    packages: [app, crate], resolve: { root: null, nodes: [
      { id: 'app', features: [], deps: [{ pkg: 'crate', dep_kinds: [{ kind: null, target: null }] }] },
      { id: 'crate', features: [], deps: [] },
    ] } };
  const cargoLock = Buffer.from(`version = 4\n\n[[package]]\nname = "zryna-playground-compiler"\n`
    + `version = "0.1.0"\n\n[[package]]\nname = "autocfg"\nversion = "1.5.1"\n`
    + `source = "${REGISTRY}"\nchecksum = "${CRATE_SHA}"\n`);
  return { metadata, sourceRoot, cargoLock,
    approved: new Map([['autocfg\0' + '1.5.1', { name: 'autocfg', version: '1.5.1', crateSha256: CRATE_SHA }]]) };
}

export function syntheticArchive(root, files, type = 48) {
  const chunks = [];
  for (const file of files) {
    const header = Buffer.alloc(512);
    header.write(`${root}/${file.path}`);
    for (const [offset, width, value] of [[100, 8, file.mode], [108, 8, 0], [116, 8, 0],
      [124, 12, file.data.length], [136, 12, 1700000000]]) {
      header.write(`${value.toString(8).padStart(width - 1, '0')}\0`, offset, width, 'ascii');
    }
    header.fill(32, 148, 156); header[156] = type;
    header.write('ustar\0', 257, 'ascii'); header.write('00', 263);
    const sum = header.reduce((a, b) => a + b, 0);
    header.write(`${sum.toString(8).padStart(6, '0')}\0 `, 148, 8, 'ascii');
    chunks.push(header, file.data, Buffer.alloc((512 - file.data.length % 512) % 512));
  }
  chunks.push(Buffer.alloc(1024));
  return gzipSync(Buffer.concat(chunks));
}

export function writeSyntheticCache(root) {
  const files = [{ path: 'Cargo.toml', mode: 0o644, data: Buffer.from('[package]\nname="fixture"\n') },
    { path: 'src/lib.rs', mode: 0o644, data: Buffer.from('// inert fixture, never compiled\n') }];
  for (const file of files) {
    mkdirSync(dirname(join(root, file.path)), { recursive: true });
    writeFileSync(join(root, file.path), file.data, { mode: file.mode });
  }
  writeFileSync(join(root, '.cargo-ok'), '{"v":1}', { mode: 0o644 });
  return files;
}
