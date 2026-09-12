import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import Ajv from 'ajv';

const ajv = new Ajv({ strict: true, allErrors: true });
const base = 'https://zryna.com/schemas/';
for (const name of ['files', 'materials', 'inventory', '']) {
  const filename = `zryna-distribution-${name ? `${name}-` : ''}v1.schema.json`;
  ajv.addSchema(JSON.parse(readFileSync(new URL(`../schemas/${filename}`, import.meta.url), 'utf8')));
}
const validate = ajv.getSchema(`${base}zryna-distribution-files-v1.schema.json`);
const tuple = { path: 'LICENSE', size: 1, sha256: 'a'.repeat(64), role: 'license',
  mode: 0o644, material: 'source', licenses: ['LICENSE'] };

test('all distribution wire schemas compile under strict draft-07 validation', () => {
  for (const name of ['materials', 'inventory', '']) {
    assert.equal(typeof ajv.getSchema(`${base}zryna-distribution-${name ? `${name}-` : ''}v1.schema.json`), 'function');
  }
  assert.equal(validate([tuple]), true);
});

test('file schemas enforce role size ceilings and executable modes', () => {
  for (const [role, mode, maximum] of [
    ['cli', 0o755, 268435456], ['runtime', 0o755, 268435456],
    ['provider', 0o644, 16777216], ['metadata', 0o644, 2097152],
    ['license', 0o644, 2097152], ['notice', 0o644, 2097152],
  ]) {
    const value = { ...tuple, role, mode, size: maximum };
    assert.equal(validate([value]), true, ajv.errorsText(validate.errors));
    assert.equal(validate([{ ...value, size: maximum + 1 }]), false);
    assert.equal(validate([{ ...value, mode: mode === 0o755 ? 0o644 : 0o755 }]), false);
  }
});

test('file schemas reject unknown fields, unsafe numeric shapes and excess entries', () => {
  for (const value of [{ ...tuple, extra: true }, { ...tuple, size: 0 },
    { ...tuple, size: 1.5 }, { ...tuple, sha256: 'A'.repeat(64) }]) {
    assert.equal(validate([value]), false);
  }
  assert.equal(validate(Array(513).fill(tuple)), false);
});
