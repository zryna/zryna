// Storage compatibility only; historical observations do not become new execution evidence.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import test from 'node:test';
const directory = new URL('./m7-generic-struct-clone-v5/', import.meta.url);
const read = name => readFileSync(new URL(name, directory));
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const ORIGINAL = '9b425828a1354fb04e2d34a8f7f1b4278b19438280e0bba8ddde35869a76bc47';

test('compact receipt storage reconstructs every original byte under the unchanged historical pin', () => {
  const receipts = JSON.parse(read('provider-receipts.json')), packets = JSON.parse(read('packet-identities.json'));
  const original = Buffer.from(`${JSON.stringify(receipts, null, 2)}\n`);
  assert.equal(original.length, 193907); assert.equal(sha(original), ORIGINAL);
  assert.equal(packets.originalReceiptsSha256, ORIGINAL);
  assert.equal(receipts.records.length, 24);
  assert.deepEqual(receipts.records.map(row => [row.case, row.provider]), packets.records.map(row => [row.case, row.provider]));
});
test('current physical receipt encoding is the separately pinned compact form without adding authority', () => {
  const bytes = read('provider-receipts.json'), receipts = JSON.parse(bytes);
  assert.equal(bytes.length, 139599);
  assert.equal(sha(bytes), '420021c88db4a8f0f05e061adae6d670f1c54fa0827d131c8088d5d07544db74');
  assert.equal(bytes.toString(), `${JSON.stringify(receipts)}\n`);
  assert(receipts.status.includes('No source authentication'));
});
