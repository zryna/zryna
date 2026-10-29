// Inert manual transport controls: no network, compiler, verifier, daemon or guest.
import assert from 'node:assert/strict';
import test from 'node:test';
import { gzipSync } from 'node:zlib';
import { dispatchData, ORIGINALS, selection } from '../scripts/stability-gates/lifecycle-private-io.mjs';
import { canonical } from '../scripts/stability-gates/input.mjs';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { relocate, restoreInputs, validateAuth } from '../scripts/stability-gates/lifecycle-private-custody.mjs';
import { verifierPins } from '../examples/playground/restricted/daemon/compiler-publication-constants.mjs';
import { release } from '../examples/playground/restricted/daemon/cli-parity/plan.mjs';
import { packCapture } from '../scripts/stability-gates/lifecycle-private-capture.mjs';
import { decodeTar } from '../scripts/distribution/archive-tar.mjs';

const capture = () => ({ downloads: ORIGINALS.map(name => ({ name, bytes: 1, sha256: '0'.repeat(64), url: 'https://example.invalid/' + name })) });
test('manual inputs are bounded data, not an approval or command channel', () => {
  const raw = canonical(capture());
  assert.equal(dispatchData(gzipSync(raw).toString('base64')), raw);
  assert.deepEqual(selection(raw, 'capture'), capture());
  for (const [value, mode] of [[{ approved: true }, 'execute'], [capture(), 'shell'], [{ downloads: [] }, 'capture']]) {
    assert.throws(() => selection(canonical(value), mode));
  }
  assert.throws(() => selection('{"downloads":[],"downloads":[]}', 'capture'));
  assert.throws(() => dispatchData(gzipSync(Buffer.alloc(262145)).toString('base64')));
});
test('fixed original identity is checked before any restore or tool', () => {
  const base = '/logical/lifecycle';
  const auth = { verifierPolicy: { ...verifierPins, path: base + '/run/cosign' } };
  for (const [key, bytes, sha256] of [['archive', release.archive.bytes, release.archive.sha256],
    ['envelope', 7253, release.envelopeSha256], ['bundle', 10990, release.bundleSha256],
    ['trustedRoot', verifierPins.trustedRootBytes, verifierPins.trustedRootSha256]]) {
    auth[key] = { sourcePath: base + '/originals/' + (key === 'trustedRoot' ? 'trusted-root' : key), bytes, sha256, ownerUid: 0, mode: 0o600 };
  }
  validateAuth(auth, base); // Pure schema consistency only, not authentication.
  assert.equal(relocate(base + '/run/manifest.json', base, '/tmp/private'), '/tmp/private/manifest.json');
  assert.equal(relocate(base + '/originals/python', base, '/tmp/private'), '/tmp/private/input/originals/python');
  const work = mkdtempSync(join(tmpdir(), 'life-'));
  try {
    assert.throws(() => restoreInputs(auth, [{ path: 'originals/archive', mode: 0o644, data: Buffer.from('replaced') }], base, work));
    assert.equal(existsSync(join(work, 'input')), false);
    assert.throws(() => validateAuth({ ...auth, envelope: { ...auth.envelope, ownerUid: 1001 } }, base));
    assert.throws(() => relocate(base + '/run/../outside', base, work));
  } finally { rmSync(work, { recursive: true }); }
});
test('a missing original, alias name, unsafe scheme or first-extra descriptor stops before I/O', () => {
  const mutations = [value => value.downloads.pop(), value => value.downloads.push(value.downloads[0]),
    value => { value.downloads[0].name = '../escape'; }, value => { value.downloads[0].url = 'http://example.invalid'; },
    value => { value.downloads[0].bytes = 192 * 1024 ** 2 + 1; }];
  for (const mutate of mutations) {
    const value = capture(); mutate(value);
    assert.throws(() => selection(canonical(value), 'capture'));
  }
});

test('raw capture preserves SemVer license paths and original bytes while refusing linked entries', async () => {
  const work = mkdtempSync(join(realpathSync(tmpdir()), 'life-pack-'));
  try {
    const source = join(work, 'capture'), output = join(work, 'packed');
    mkdirSync(source); mkdirSync(output); mkdirSync(join(source, 'target'));
    writeFileSync(join(source, 'target', 'excluded-cache'), 'not a raw observation');
    const expected = new Map([['metadata-before.stdout.raw', Buffer.from('{"original":true}\n')],
      ...['before', 'after'].map(phase => [
        `licenses-${phase}/licenses/rust/toml-0.9.12+spec-1.1.0/LICENSE-MIT`, Buffer.from([0, 255, 10]),
      ])]);
    for (const [path, data] of expected) {
      mkdirSync(join(source, ...path.split('/').slice(0, -1)), { recursive: true });
      writeFileSync(join(source, path), data, { flag: 'wx' });
    }
    const pins = await packCapture(source, output), actual = new Map();
    assert.equal(pins.length, 2);
    for (const pin of pins) {
      const bytes = readFileSync(join(output, pin.path)); assert.equal(bytes.length, pin.bytes);
      for (const file of await decodeTar(bytes, 'capture', 0)) {
        assert(!actual.has(file.path)); actual.set(file.path, file.data);
      }
    }
    assert.deepEqual(actual, expected);
    const refused = join(work, 'refused'); mkdirSync(refused);
    symlinkSync(output, join(source, 'linked-records'), process.platform === 'win32' ? 'junction' : 'dir');
    await assert.rejects(packCapture(source, refused));
    assert.equal(existsSync(join(refused, 'capture-main.tar.gz')), false);
  } finally { rmSync(work, { recursive: true }); }
});

