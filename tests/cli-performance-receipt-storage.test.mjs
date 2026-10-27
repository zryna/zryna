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
  {
    family: 'm7-generic-struct-v4-contextual-extra', originalBytes: 63792, compactBytes: 45864,
    originalSha: '17f2ec76171371e3422124d305236011852e461e935088e9d760cbab57ca8760',
    compactSha: '9c18af1df6922e590dffba6cae3168f92adebbaace0b1a9b21b2c7236fb5a264',
    cases: ['imported-alias-decoy', 'member-height-256', 'member-height-257', 'member-height-shared-prefix'],
  },
  {
    family: 'm7-generic-execution-partial-static', originalBytes: 47663, compactBytes: 37699,
    originalSha: '7fabc5a1c7963448a4997b54dc58355facf4e866e5ce22bfb8566bbfeac048c2',
    compactSha: 'c6e81d2794a26164dd3a63199960c14e84706ea24cc6d4726f7e834a79a3e063',
    receiptPin: false,
    cases: ['struct-survivor', 'struct-refill-whole', 'partial-root-replacement', 'nested-disjoint',
      'array-survivor', 'array-refill', 'opaque-child', 'sibling-loan', 'failure-after-hole',
      'branch-same-hole', 'loop-refill', 'double-field', 'whole-after-hole', 'write-borrowed-field',
      'unrestored-loop-field', 'wrong-field-rhs'],
  },
  {
    family: 'm7-generic-execution-dynamic-index', originalBytes: 47951, compactBytes: 37987,
    originalSha: '8d0aacb8c82d746e16ebc7d52bacebaf09eb39add07fb5c0f9ba5dd67c05ba13',
    compactSha: 'f5a6fdafe5a5341421012da25155aebed6a264c864f9f730fbd6ecf8cabaac18',
    receiptPin: false,
    cases: ['array-exclusive-copy', 'array-indexed-rhs-match', 'array-shared-copy', 'chained-shared-copy',
      'copy-read-control', 'dynamic-array-overlap', 'index-once-before-rhs', 'indexed-rhs-moves-owner',
      'opaque-indexed-consume', 'owner-move-while-indexed', 'shared-element-write', 'vec-exclusive-replace',
      'vec-indexed-replace', 'vec-shared-clone', 'vector-constant-overlap', 'zero-length-bounds'],
  },
  {
    family: 'm7-generic-execution-borrowed-match', originalBytes: 48063, compactBytes: 38099,
    originalSha: '3c4efd6874d58f348b1fcbbc2c8ae3c7889a43a2cbea0b90108d26045fa71b21',
    compactSha: '8344f4aa396dfb07cb0379e35adc214b3ee043358e10c4eee8a0cd5cb63de65a',
    receiptPin: false,
    cases: ['nominal-shared-copy', 'nominal-shared-clone', 'nominal-exclusive-copy', 'nominal-exclusive-replace',
      'option-shared-both', 'result-shared-opaque-bound', 'result-exclusive-both', 'opaque-payload-forward',
      'nested-exclusive-result', 'branch-arm-cleanup', 'shared-reborrow-from-exclusive', 'shared-payload-move',
      'shared-to-exclusive-call', 'exclusive-parent-frozen', 'opaque-payload-clone', 'arm-loan-result-escape'],
  },
];

for (const fixture of fixtures) {
  const directory = new URL(`./${fixture.family}/`, import.meta.url);
  const read = name => readFileSync(new URL(name, directory));
  test(`${fixture.family}: exact original receipt bytes and complete inventory survive storage encoding`, () => {
    const receipts = JSON.parse(read('provider-receipts.json')), packets = JSON.parse(read('packet-identities.json'));
    const original = Buffer.from(`${JSON.stringify(receipts, null, 2)}\n`);
    assert.equal(original.length, fixture.originalBytes); assert.equal(sha(original), fixture.originalSha);
    if (fixture.receiptPin === false) assert(!Object.hasOwn(packets, 'originalReceiptsSha256'), 'no invented historical pin');
    else assert.equal(packets.originalReceiptsSha256, fixture.originalSha);
    const inventory = fixture.cases.flatMap(name => ['native', 'typescript'].map(provider => [name, provider]));
    assert.deepEqual(receipts.records.map(row => [row.case, row.provider]), inventory);
    assert.deepEqual(packets.records.map(row => [row.case, row.provider]), inventory);
  });
  test(`${fixture.family}: physical compact encoding has its own pin and adds no authority`, () => {
    const bytes = read('provider-receipts.json'), receipts = JSON.parse(bytes);
    assert.equal(bytes.length, fixture.compactBytes); assert.equal(sha(bytes), fixture.compactSha);
    assert.equal(bytes.toString(), `${JSON.stringify(receipts)}\n`);
    assert(/source authentication|No compiler source\/semantic\/seal\/runtime claim/iu.test(receipts.status));
  });
}
