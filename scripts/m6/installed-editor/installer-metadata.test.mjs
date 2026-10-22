// Synthetic filesystem/parser controls only; never an editor or publisher receipt.
import assert from 'node:assert/strict';
import { mkdir, mkdtemp, realpath, writeFile, rm, link, symlink } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { tmpdir } from 'node:os';
import test from 'node:test';
import { descriptor } from './files.mjs';
import { sha, verifyInstalled } from './materials.mjs';
import { bindInstaller, deriveInstallerIdentity, installedPayload } from './installer-metadata.mjs';
function zip(rows) {
  const locals = [], central = [];
  let offset = 0;
  for (const [name, body] of rows) {
    const encoded = Buffer.from(name), local = Buffer.alloc(30), entry = Buffer.alloc(46);
    local.writeUInt32LE(0x04034b50, 0); local.writeUInt16LE(20, 4);
    local.writeUInt32LE(body.length, 18); local.writeUInt32LE(body.length, 22);
    local.writeUInt16LE(encoded.length, 26);
    entry.writeUInt32LE(0x02014b50, 0); entry.writeUInt16LE(20, 6);
    entry.writeUInt32LE(body.length, 20); entry.writeUInt32LE(body.length, 24);
    entry.writeUInt16LE(encoded.length, 28); entry.writeUInt32LE(offset, 42);
    locals.push(local, encoded, body); central.push(entry, encoded);
    offset += local.length + encoded.length + body.length;
  }
  const directory = Buffer.concat(central), end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0); end.writeUInt16LE(rows.length, 8);
  end.writeUInt16LE(rows.length, 10); end.writeUInt32LE(directory.length, 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...locals, directory, end]);
}
const serialize = (original, fields) => Buffer.from(JSON.stringify({ ...original, __metadata: fields }, null, '\t'));
async function fixture(t, original = { name: 'synthetic-only', version: '0', nested: { keep: ['all', 'values'] } }) {
  const root = await mkdtemp(join(await realpath(tmpdir()), 'm6-installer-metadata-unit-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const installed = join(root, 'installed'); await mkdir(installed);
  const bodies = { 'package.json': Buffer.from(JSON.stringify(original, null, 2) + '\n') };
  for (let i = 0; i < 11; i++) bodies[`src/file-${i}.cjs`] = Buffer.from(`synthetic body ${i}`);
  const manifest = Buffer.from('<synthetic-only-not-an-official-vsix/>');
  const bytes = zip([['[Content_Types].xml', Buffer.from('<synthetic/>')],
    ['extension.vsixmanifest', manifest],
    ...Object.entries(bodies).map(([path, body]) => ['extension/' + path, body])]);
  const source = join(root, 'source.vsix'); await writeFile(source, bytes);
  const payload = Object.fromEntries(Object.entries(bodies).map(([path, body]) =>
    [path, { bytes: body.length, sha256: sha(body) }]));
  const identity = deriveInstallerIdentity(bytes, payload, await descriptor(source));
  const window = { before: 1000, after: 2000 };
  const fields = { installedTimestamp: 1500, targetPlatform: 'undefined',
    size: Object.values(payload).reduce((sum, row) => sum + row.bytes, manifest.length) };
  const raw = serialize(original, fields);
  const installer = bindInstaller(identity, payload, window, raw);
  for (const [path, body] of Object.entries({ ...bodies, 'package.json': raw, '.vsixmanifest': manifest })) {
    await mkdir(dirname(join(installed, path)), { recursive: true });
    await writeFile(join(installed, path), body);
  }
  return { root, installed, source, original, bodies, manifest, bytes, payload, identity, window, fields, raw, installer };
}
test('explicit source-bound installer preserves eleven bodies and exact original manifest', async t => {
  const f = await fixture(t), expected = installedPayload(f.payload, f.installer);
  assert.equal(Object.keys(expected).length, 13);
  for (const [path, value] of Object.entries(f.payload)) {
    if (path !== 'package.json') assert.deepEqual(expected[path], value);
  }
  assert.deepEqual(expected['.vsixmanifest'], { bytes: f.manifest.length, sha256: sha(f.manifest) });
  assert.deepEqual(expected['package.json'], { bytes: f.raw.length, sha256: sha(f.raw) });
  assert.notEqual(expected['package.json'].sha256, f.payload['package.json'].sha256);
  assert.equal((await verifyInstalled(f.installed, f.payload, undefined, f.installer)).entries
    .filter(row => row.kind === 'file').length, 13);
  await assert.rejects(verifyInstalled(f.installed, f.payload));
  assert.throws(() => { f.installer.window.after = 9000; });
});
test('unknown namespace, manifest substitution and every original body stay independently checked', async t => {
  const changes = [
    f => writeFile(join(f.installed, 'extra'), 'unknown'),
    f => mkdir(join(f.installed, 'unknown-directory')),
    f => writeFile(join(f.installed, '.vsixmanifest'), 'substituted manifest'),
    f => writeFile(join(f.installed, 'src/file-7.cjs'), 'substituted original source'),
    f => writeFile(join(f.installed, 'package.json'), Buffer.concat([f.raw, Buffer.from('\n')])),
    f => rm(join(f.installed, '.vsixmanifest')),
  ];
  for (const change of changes) {
    const f = await fixture(t);
    await verifyInstalled(f.installed, f.payload, undefined, f.installer);
    await change(f);
    await assert.rejects(verifyInstalled(f.installed, f.payload, undefined, f.installer));
  }
});
test('metadata extras, substitutions, duplicates and alternate serialization are never stripped', async t => {
  const f = await fixture(t);
  const packages = [Buffer.concat([f.raw, Buffer.from('\n')]),
    Buffer.from(JSON.stringify({ ...f.original, __metadata: f.fields }, null, 2)),
    serialize({ ...f.original, nested: { keep: ['changed'] } }, f.fields),
    serialize({ added: true, ...f.original }, f.fields),
    serialize(f.original, { ...f.fields, extra: false }),
    serialize(f.original, { ...f.fields, targetPlatform: 'win32-x64' }),
    serialize(f.original, { ...f.fields, size: f.raw.length + f.manifest.length }),
    serialize(f.original, { ...f.fields, installedTimestamp: 999 }),
    serialize(f.original, { ...f.fields, installedTimestamp: 2001 }),
    serialize(f.original, { ...f.fields, installedTimestamp: 1500.5 }),
    serialize(f.original, { ...f.fields, installedTimestamp: '1500' }),
    serialize(f.original, { size: f.fields.size, targetPlatform: 'undefined', installedTimestamp: 1500 }),
    Buffer.from(f.raw.toString().replace('"installedTimestamp": 1500',
      '"installedTimestamp": 1500, "installedTimestamp": 1500'))];
  for (const raw of packages) {
    bindInstaller(f.identity, f.payload, f.window, f.raw);
    assert.throws(() => bindInstaller(f.identity, f.payload, f.window, raw));
  }
});
test('wallclock window is closed, bounded and never replaced by later verification time', async t => {
  const f = await fixture(t);
  for (const window of [{ before: 2000, after: 1000 }, { before: -1, after: 2000 },
    { before: 1000.5, after: 2000 }, { before: 1000, after: Infinity },
    { before: 1000, after: 602000 }, { before: 1000, after: 2000, grace: 1 }]) {
    bindInstaller(f.identity, f.payload, f.window, f.raw);
    assert.throws(() => bindInstaller(f.identity, f.payload, window, f.raw));
  }
  const changed = structuredClone(f.installer); changed.installedTimestamp = 2001;
  assert.throws(() => installedPayload(f.payload, changed));
  const unknown = structuredClone(f.installer); unknown.extra = true;
  assert.throws(() => installedPayload(f.payload, unknown));
});
test('original source identity and expected payload cannot be substituted by installed hashes', async t => {
  const f = await fixture(t);
  const wrongExpected = structuredClone(f.payload);
  wrongExpected['src/file-0.cjs'].sha256 = '0'.repeat(64);
  assert.throws(() => bindInstaller(f.identity, wrongExpected, f.window, f.raw));
  const wrongIdentity = structuredClone(f.identity); wrongIdentity.manifest.extra = 1;
  assert.throws(() => bindInstaller(wrongIdentity, f.payload, f.window, f.raw));
  await verifyInstalled(f.installed, f.payload, undefined, f.installer);
  await writeFile(f.source, Buffer.alloc(f.bytes.length, 0));
  await assert.rejects(verifyInstalled(f.installed, f.payload, undefined, f.installer));
});
test('retained strict file and alias guards still apply to installer-aware verification', async t => {
  const f = await fixture(t), file = join(f.installed, 'src/file-0.cjs');
  await verifyInstalled(f.installed, f.payload, undefined, f.installer);
  await link(file, join(f.root, 'same-body-hardlink'));
  await assert.rejects(verifyInstalled(f.installed, f.payload, undefined, f.installer));
  const other = await fixture(t), alias = join(other.root, 'alias');
  await symlink(other.installed, alias, process.platform === 'win32' ? 'junction' : 'dir');
  await assert.rejects(verifyInstalled(alias, other.payload, undefined, other.installer));
});

test('authenticated original __metadata cannot be silently overwritten', async t => {
  await assert.rejects(fixture(t, { name: 'synthetic', __metadata: { existing: true } }));
});
