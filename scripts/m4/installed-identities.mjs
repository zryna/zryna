import assert from 'node:assert/strict';
import path from 'node:path';
import { digest, readStableFile } from './registry-files.mjs';

const world = 'zryna:capability-profiles/command@0.1.0';
function uint32(number) {
  const bytes = Buffer.alloc(4); bytes.writeUInt32LE(number); return bytes;
}
function uint64(number) {
  const bytes = Buffer.alloc(8); bytes.writeBigUInt64LE(BigInt(number)); return bytes;
}
function frame(text) {
  const bytes = Buffer.from(text); return Buffer.concat([uint32(bytes.length), bytes]);
}
function witIdentity(registry, sourceRoot, component) {
  const wit = JSON.parse(readStableFile(path.join(sourceRoot, 'tests/wit-capability-profiles-v1.json'), 64 * 1024));
  const profile = wit.profiles.find(entry => entry.id === 'command');
  const explicit = [...profile.imports].sort();
  const resolved = [...explicit, 'wasi:io/error@0.2.12', 'wasi:io/poll@0.2.12', 'wasi:io/streams@0.2.12'].sort();
  const packages = [...wit.wasi.packages, 'wasi:io@0.2.12', wit.wit.package].sort();
  assert.deepEqual(component.packages, packages, 'exact WIT package closure');
  assert.deepEqual(component.explicitImports, explicit, 'specified imports remain distinct from effective host grants');
  assert.deepEqual(component.resolvedImports, resolved, 'exact parser-resolved type dependency closure');
  assert.deepEqual(component.exports, profile.exports);
  const sources = [...registry.scope.witClosure].sort((a, b) => a.logicalPath < b.logicalPath ? -1 : 1);
  assert.equal(sources.length, 34);
  const bound = [Buffer.from('zryna.command-wit-binding.v1\0'), frame(world), uint32(sources.length)];
  const closure = [];
  for (const source of sources) {
    const bytes = readStableFile(path.join(sourceRoot, source.path), 256 * 1024);
    assert.equal(digest(bytes), source.sha256);
    bound.push(frame(source.logicalPath), Buffer.from(source.sha256, 'hex'));
    const name = Buffer.from(source.logicalPath);
    closure.push(uint64(name.length), name, uint64(bytes.length), bytes);
  }
  for (const names of [explicit, resolved, profile.exports]) {
    bound.push(uint32(names.length), ...names.map(frame));
  }
  assert.equal(component.worldSha256, digest(Buffer.concat(bound)), 'exact WIT world binding');
  assert.equal(component.witClosureDigest, digest(Buffer.concat(closure)), 'exact WIT source closure bytes');
}
export function validateInstalledIdentity(manifest, registry, sourceRoot) {
  const required = manifest.source.requirements;
  witIdentity(registry, sourceRoot, manifest.component);
  const program = [Buffer.from('zryna.command-program-binding.v1\0'), frame('command-h1-v1'),
    ...[manifest.source.sha256, manifest.component.languageSha256, manifest.component.storageSha256,
      manifest.component.linear32Sha256, manifest.component.linuxX8664Sha256, manifest.component.worldSha256]
      .map(value => Buffer.from(value, 'hex')),
    ...[256, 0, 65536, 65536, 15728640, 15728640, 16777216].map(uint32),
    Buffer.from([required.length ? 1 : 0]), ...(required.length ? [frame('MODE')] : [])];
  assert.equal(manifest.source.programBinding, digest(Buffer.concat(program)), 'producer program binding observation');
}
