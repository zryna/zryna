// Storage compatibility only; historical observations do not become new execution evidence.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import test from 'node:test';
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const fixtures = [
  {
    family: 'm7-generic-struct-clone-v5', originalBytes: 193907, compactBytes: 139599,
    originalSha: '9b425828a1354fb04e2d34a8f7f1b4278b19438280e0bba8ddde35869a76bc47',
    compactSha: '420021c88db4a8f0f05e061adae6d670f1c54fa0827d131c8088d5d07544db74',
    cases: ['nested-prefix', 'repeat-source', 'shared-owner', 'phantom-opaque', 'fixed-member',
      'family-fields', 'copy-struct', 'opaque-direct', 'opaque-stored', 'opaque-inactive',
      'moved-source', 'exclusive-source'],
  },
  {
    family: 'm7-generic-struct-clone-compact-dag', originalBytes: 48242, compactBytes: 34750,
    originalSha: 'ed8d5e5c57dee38e8203afe504c1deb7439a720f00946d22fc47921520256085',
    compactSha: 'bac64af07f6443a6211efa1307a5ef02040c5c486d02c90a09246357bb2b1bd3',
    cases: ['compact-dag-n04', 'compact-dag-n12', 'compact-dag-n16'],
  },
];

for (const fixture of fixtures) {
  const directory = new URL(`./${fixture.family}/`, import.meta.url);
  const read = name => readFileSync(new URL(name, directory));
  test(`${fixture.family}: every original receipt byte and complete inventory retain the historical pin`, () => {
    const receipts = JSON.parse(read('provider-receipts.json')), packets = JSON.parse(read('packet-identities.json'));
    const original = Buffer.from(`${JSON.stringify(receipts, null, 2)}\n`);
    assert.equal(original.length, fixture.originalBytes); assert.equal(sha(original), fixture.originalSha);
    assert.equal(packets.originalReceiptsSha256, fixture.originalSha);
    const inventory = fixture.cases.flatMap(name => ['native', 'typescript'].map(provider => [name, provider]));
    assert.deepEqual(receipts.records.map(row => [row.case, row.provider]), inventory);
    assert.deepEqual(packets.records.map(row => [row.case, row.provider]), inventory);
  });
  test(`${fixture.family}: physical compact encoding has its own pin and adds no authority`, () => {
    const bytes = read('provider-receipts.json'), receipts = JSON.parse(bytes);
    assert.equal(bytes.length, fixture.compactBytes); assert.equal(sha(bytes), fixture.compactSha);
    assert.equal(bytes.toString(), `${JSON.stringify(receipts)}\n`);
    assert(receipts.status.toLowerCase().includes('source authentication'));
  });
}
