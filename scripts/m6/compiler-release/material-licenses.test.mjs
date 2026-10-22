import assert from 'node:assert/strict';
import test from 'node:test';
import { sha256 } from '../../distribution/canonical.mjs';
import { captureCompilerLicenses } from './material-licenses.mjs';
import { syntheticArchive } from './material-fixtures.mjs';

function fixture() {
  const data = Buffer.from('Synthetic test license only, no publication authority\n');
  const archive = syntheticArchive('fixture-1.0.0', [{ path: 'LICENSE', data, mode: 0o644 }]);
  const pin = { name: 'fixture', version: '1.0.0', crateSha256: sha256(archive), files: [{
    origin: 'https://static.crates.io/crates/fixture/fixture-1.0.0.crate', sourcePath: 'LICENSE',
    path: 'licenses/rust/fixture-1.0.0/LICENSE', size: data.length, sha256: sha256(data),
  }] };
  return { data, archive, pin };
}

test('selected license bytes are independently copied from authenticated ordinary crate member', () => {
  const { data, archive, pin } = fixture();
  const files = captureCompilerLicenses(pin, archive, null);
  assert.equal(files.length, 1);
  assert.deepEqual(files[0].data, data);
  assert.equal(files[0].path, pin.files[0].path);
  assert.equal(files[0].mode, 0o644);
  files[0].data.fill(0);
  assert.deepEqual(captureCompilerLicenses(pin, archive, null)[0].data, data);
});

for (const [name, mutate] of [
  ['wrong original checksum', x => { x.pin.crateSha256 = 'f'.repeat(64); }],
  ['missing member', x => { x.pin.files[0].sourcePath = 'MISSING'; }],
  ['changed licensed bytes', x => { x.pin.files[0].sha256 = 'f'.repeat(64); }],
  ['changed licensed size', x => { x.pin.files[0].size++; }],
  ['foreign license origin', x => { x.pin.files[0].origin = 'https://foreign.invalid/LICENSE'; }],
  ['empty license record', x => { x.pin.files = []; }],
]) {
  test(`compiler license extraction rejects ${name}`, () => {
    const input = fixture(); mutate(input);
    assert.throws(() => captureCompilerLicenses(input.pin, input.archive, null));
  });
}

test('Wasmtime upstream exception requires exact independently approved URL and actual bytes', () => {
  const { pin, data, archive } = fixture();
  pin.files[0].origin = 'https://raw.githubusercontent.com/bytecodealliance/wasmtime/'
    + '7bac2c2775808aaec5d4aa5627a5e447b51102cf/LICENSE';
  assert.throws(() => captureCompilerLicenses(pin, archive, null));
  assert.throws(() => captureCompilerLicenses(pin, archive, Buffer.from('substitution')));
  assert.deepEqual(captureCompilerLicenses(pin, archive, data)[0].data, data);
  pin.files[0].origin = pin.files[0].origin.replace('7bac2c', 'ffffff');
  assert.throws(() => captureCompilerLicenses(pin, archive, data));
});
