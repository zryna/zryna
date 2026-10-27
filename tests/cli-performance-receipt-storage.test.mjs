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
    records: 24, inventorySha: '525afdc6bd481c0c43ceb716987c1d91939d1290503c55a49dcdf2e2575bb86f',
  },
  {
    family: 'm7-generic-struct-clone-compact-dag', originalBytes: 48242, compactBytes: 34750,
    originalSha: 'ed8d5e5c57dee38e8203afe504c1deb7439a720f00946d22fc47921520256085',
    compactSha: 'bac64af07f6443a6211efa1307a5ef02040c5c486d02c90a09246357bb2b1bd3',
    records: 6, inventorySha: '30dce2ad9c742181c595fdec79031031158e0828c15fbc8509a53ba4dbf510d9',
  },
  {
    family: 'm7-generic-struct-v4-contextual-extra', originalBytes: 63792, compactBytes: 45864,
    originalSha: '17f2ec76171371e3422124d305236011852e461e935088e9d760cbab57ca8760',
    compactSha: '9c18af1df6922e590dffba6cae3168f92adebbaace0b1a9b21b2c7236fb5a264',
    records: 8, inventorySha: 'b95582bc1ef6859798efc7c071e03e629b520739cc710c3808f52842cf8eb29d',
  },
  {
    family: 'm7-generic-execution-partial-static', originalBytes: 47663, compactBytes: 37699,
    originalSha: '7fabc5a1c7963448a4997b54dc58355facf4e866e5ce22bfb8566bbfeac048c2',
    compactSha: 'c6e81d2794a26164dd3a63199960c14e84706ea24cc6d4726f7e834a79a3e063',
    receiptPin: false,
    records: 32, inventorySha: '40727b6dfcba48bb7ddecf6c003384feadb828e69fbee2d7e45707b576d5f691',
  },
  {
    family: 'm7-generic-execution-dynamic-index', originalBytes: 47951, compactBytes: 37987,
    originalSha: '8d0aacb8c82d746e16ebc7d52bacebaf09eb39add07fb5c0f9ba5dd67c05ba13',
    compactSha: 'f5a6fdafe5a5341421012da25155aebed6a264c864f9f730fbd6ecf8cabaac18',
    receiptPin: false,
    records: 32, inventorySha: 'b7cf0b86736f756008299063db5bc683ac8cf3118b11bd3a893166de4c482a5a',
  },
  {
    family: 'm7-generic-execution-borrowed-match', originalBytes: 48063, compactBytes: 38099,
    originalSha: '3c4efd6874d58f348b1fcbbc2c8ae3c7889a43a2cbea0b90108d26045fa71b21',
    compactSha: '8344f4aa396dfb07cb0379e35adc214b3ee043358e10c4eee8a0cd5cb63de65a',
    receiptPin: false,
    records: 32, inventorySha: '8fe599755476c9330353682c728078100eeb125168b8e2f87a06e800853a577a',
  },
  {
    family: 'm7-generic-execution-original-deep', originalBytes: 16318, compactBytes: 11818,
    originalSha: '9514e2f97b913cb2c7810256ccc3557b0420b0fa175ff775c290937332496333',
    compactSha: 'fb97f9f5803e4717f0e23d5729034cb8e669288ad86ed42f9322981bdcf89f43',
    records: 2, inventorySha: '1168e4722f5bc603447f501eeaa91979c11ebd29c6926e9547d38a00bed96ad1',
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
    const inventory = receipts.records.map(row => [row.case, row.provider]);
    assert.equal(inventory.length, fixture.records);
    assert.equal(sha(Buffer.from(JSON.stringify(inventory))), fixture.inventorySha);
    assert.deepEqual(packets.records.map(row => [row.case, row.provider]), inventory);
  });
  test(`${fixture.family}: physical compact encoding has its own pin and adds no authority`, () => {
    const bytes = read('provider-receipts.json'), receipts = JSON.parse(bytes);
    assert.equal(bytes.length, fixture.compactBytes); assert.equal(sha(bytes), fixture.compactSha);
    assert.equal(bytes.toString(), `${JSON.stringify(receipts)}\n`);
    assert(/source authentication|No compiler source\/semantic\/seal\/runtime claim/iu.test(receipts.status));
  });
}

test('frozen Copy enum DTO retains its complete original byte identity', () => {
  const bytes = readFileSync(new URL('./m7-generic-owned-copy-enum-phi/2-reference.json', import.meta.url));
  const original = Buffer.from(`${JSON.stringify(JSON.parse(bytes), null, 2)}\n`);
  assert.equal(original.length, 508297);
  assert.equal(sha(original), '6c34a7ba79583de8d612371c5ead79989989a22a4e2fbe32bb68204404672eb0');
});
test('frozen Copy enum DTO storage has an independent compact identity', () => {
  const bytes = readFileSync(new URL('./m7-generic-owned-copy-enum-phi/2-reference.json', import.meta.url));
  assert.equal(bytes.length, 159910);
  assert.equal(sha(bytes), 'e073b60885c6c90a5c7f6f17811c968d416c4d134f553a2876266d04d5f06aaa');
  assert.equal(bytes.toString(), `${JSON.stringify(JSON.parse(bytes))}\n`);
});
