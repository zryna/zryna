// Exact original pre-provider oracle; runtime identities are checked independently of bytes.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
export function oracle(path, name) {
  const bytes = readFileSync(path);
  assert.equal(createHash('sha256').update(bytes).digest('hex'), '99570cb50f7c75af51c88339584817f65069495e5f25eb6c04b78a724ffde16d');
  const row = JSON.parse(bytes).cases.find(row => row.name === name);
  assert.equal(row.semantic, 'accept'); assert([0,2].includes(row.allocations));
  return row;
}
export function check(row, fault, records) {
  const expectedReleases = fault ? fault.releases : row.string_releases;
  const allocations = records.filter(record => record.tag === 1 || record.tag === 2);
  const releases = records.filter(record => record.tag === 3);
  const failures = records.filter(record => record.tag === 4 || record.tag === 5);
  assert.equal(failures.length, fault ? 1 : 0);
  const successCount = fault ? (fault.kind === 'clone' ? 1 : fault.ordinal - 1) : row.allocations;
  assert.equal(allocations.length, successCount);
  assert.deepEqual(allocations.map(record => record.id), Array.from({length:successCount}, (_,i) => i+1));
  const live = new Map(), identities = new Set();
  for (const record of records) {
    assert(Number.isInteger(record.id) && record.id > 0);
    assert.equal(record.bytes_hex, row.string_utf8_hex[record.tag === 1 || record.tag === 2 || record.tag === 3 ? record.id - 1 : 0]);
    assert.equal(record.length, Buffer.from(record.bytes_hex, 'hex').length);
    if (record.tag === 1 || record.tag === 2) {
      assert.notEqual(record.identity, '0'); assert(!identities.has(record.identity));
      identities.add(record.identity);
      if (record.tag === 1) { assert.equal(record.id, 1); assert.equal(record.sourceIdentity, '0'); }
      else { assert.equal(record.id, 2); assert.equal(record.sourceIdentity, live.get(1)?.identity); assert.notEqual(record.identity, record.sourceIdentity); }
      live.set(record.id, record);
    } else if (record.tag === 3) {
      assert.equal(record.identity, live.get(record.id)?.identity); assert.equal(record.sourceIdentity, '0'); live.delete(record.id);
    } else {
      assert(fault); assert.equal(record.identity, '0'); assert.equal(record.tag, fault.kind === 'allocation' ? 4 : 5); assert.equal(record.id, fault.ordinal);
      assert.equal(record.sourceIdentity, fault.kind === 'allocation' && fault.ordinal === 1 ? '0' : live.get(1)?.identity);
    }
  }
  assert.equal(live.size, 0);
  assert.deepEqual(releases.map(record => record.id), expectedReleases);
  assert.equal(new Set(releases.map(record => record.identity)).size, allocations.length);
  assert.equal(releases.length, allocations.length);
  const allocationAttempts = records.filter(record => [1,2,4].includes(record.tag)).length;
  const cloneCalls = records.filter(record => record.tag === 2 || record.tag === 5 || (record.tag === 4 && record.sourceIdentity !== '0')).length;
  assert.equal(allocationAttempts, fault ? (fault.kind === 'allocation' ? fault.ordinal : 1) : row.allocations);
  assert.equal(cloneCalls, fault ? (fault.kind === 'allocation' && fault.ordinal === 1 ? 0 : 1) : row.clone_calls);
  assert.deepEqual(records.map(record => record.tag), fault ? (fault.kind === 'allocation' && fault.ordinal === 1 ? [4] : [1,fault.kind === 'allocation' ? 4 : 5,3]) : row.allocations === 0 ? [] : [1,2,3,3]);
  return { records, allocationAttempts, cloneCalls, actualAcquisitionsReleasedExactlyOnce:true };
}
