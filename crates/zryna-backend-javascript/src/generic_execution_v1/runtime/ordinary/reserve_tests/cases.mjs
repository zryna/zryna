import assert from 'node:assert/strict';

const cases = $reserveExpected.cases.filter(row => row.group === process.argv[2]);
assert.ok(cases.length > 0, 'known frozen contract group');
for (const row of cases) {
  $runtimeEnter();
  const element = Object.freeze({id:row.layout,stride:row.stride,alignment:row.alignment});
  const initial = $rt.vecAllocate(element,row.old);
  assert.equal(initial.status,0,`${row.name}: valid initial allocation`);
  const old = initial.handle;
  const before = old === 0 ? 0 : Object.getOwnPropertyDescriptors(old);
  const result = $rt.vecReserve(element,old,row.required);
  assert.equal(result.status,row.status,`${row.name}: canonical status`);
  assert.equal(Object.getPrototypeOf(result),null,`${row.name}: private result prototype`);
  assert.ok(Object.isFrozen(result),`${row.name}: immutable status record`);
  if (row.status === 0) {
    assert.equal(result.handle.capacity,row.capacity,`${row.name}: exact canonical capacity`);
    assert.equal(result.handle.length,0,`${row.name}: retained old length`);
    if (row.required <= row.old) {
      assert.equal(result.handle,old,`${row.name}: no-growth storage identity`);
    } else if (old !== 0) {
      assert.notEqual(result.handle.pointer,old.pointer,`${row.name}: grown allocation identity`);
      assert.equal($rt.vecReleaseStorage(element,old),255,`${row.name}: old allocation retired`);
    }
    assert.equal($rt.vecReleaseStorage(element,result.handle),0,`${row.name}: release result`);
  } else {
    assert.equal(result.pointer,0,`${row.name}: zero failed pointer`);
    assert.equal(result.handle,0,`${row.name}: zero failed handle`);
    assert.equal(result.value,false,`${row.name}: initialized failed value`);
    assert.deepEqual(old === 0 ? 0 : Object.getOwnPropertyDescriptors(old),before,`${row.name}: input unchanged`);
    const retained = $rt.vecReserve(element,old,row.old);
    assert.equal(retained.status,0,`${row.name}: old storage remains usable`);
    assert.equal(retained.handle,old,`${row.name}: old storage remains identical`);
    assert.equal($rt.vecReleaseStorage(element,old),0,`${row.name}: release retained original`);
    const retry = $rt.vecReserve(element,0,1);
    assert.equal(retry.status,0,`${row.name}: valid retry after failure`);
    assert.equal(retry.handle.capacity,4,`${row.name}: pristine geometric retry`);
    assert.equal($rt.vecReleaseStorage(element,retry.handle),0,`${row.name}: release retry`);
  }
  $runtimeExit();
}
