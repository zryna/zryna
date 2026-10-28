// Fixed material Node ELF declarations only; no actual binaries or material authentication.
import assert from 'node:assert/strict';
import test from 'node:test';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { createLifecycleOwnerPlan } from '../scripts/stability-gates/lifecycle-owner-inputs.mjs';
import { fixture, reject, selected, materialNode, nodeLibrary } from './lifecycle-owner-inputs-fixture.mjs';
test('the fixed material Node reaches image libraries without becoming an image file or approval', () => {
  const value = fixture(), node = value.runtime.elf.find(row => row.path === materialNode);
  assert.deepEqual(Object.keys(value.runtime).sort(), ['elf', 'files', 'python', 'support']);
  assert.deepEqual(node.needed, [{ name: 'libnode-fixture.so', path: nodeLibrary }]);
  assert(!value.runtime.files.some(file => file.path === materialNode));
  assert(value.runtime.files.some(file => file.path === nodeLibrary));
  assert(value.runtime.elf.filter(row => row.path !== materialNode).every(row =>
    row.interpreter !== nodeLibrary && row.needed.every(item => item.path !== nodeLibrary)), 'Node-only dependency');
  const plan = createLifecycleOwnerPlan(value);
  assert.equal(plan.runtime.files, value.runtime.files.length);
  assert.equal(plan.runtime.bytes, value.runtime.files.reduce((sum, file) => sum + file.bytes, 0));
  assert.equal(plan.materials.bytes, 133968848);
  assert.equal(plan.runtimeApproved, false); assert.equal(plan.materialsApproved, false);
  assert.equal(plan.executionApproved, false); assert.equal(plan.proofQualified, false);
});
test('Node material bytes remain separate when declared image bytes reach the unchanged 128 MiB ceiling', () => {
  const value = fixture(), imageBytes = value.runtime.files.reduce((sum, file) => sum + file.bytes, 0);
  const file = { ...selected('/resources/runtime-data.bin', 128 * 1024 * 1024 - imageBytes), mode: 0o444,
    kind: 'regular', nlink: 1, uid: 0, gid: 0 };
  value.runtime.files.push(file); value.runtime.files.sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  value.runtime.support.push(file.path);
  value.expected.runtimeSha256 = sha256(Buffer.from(canonical(value.runtime)));
  const plan = createLifecycleOwnerPlan(value);
  assert.equal(plan.runtime.bytes, 128 * 1024 * 1024); assert.equal(plan.materials.bytes, 133968848);
});
test('the material Node declaration is mandatory even when the former image-only closure is complete', () => {
  const value = fixture();
  value.runtime.elf = value.runtime.elf.filter(row => row.path !== materialNode && row.path !== nodeLibrary);
  value.runtime.files = value.runtime.files.filter(file => file.path !== nodeLibrary);
  value.expected.runtimeSha256 = sha256(Buffer.from(canonical(value.runtime)));
  assert.throws(() => createLifecycleOwnerPlan(value), /fixed material Node ELF root required/);
});
test('no other material ELF root or caller-selected root can replace or accompany the fixed Node', () => {
  for (const change of [v => { v.runtime.elf.find(row => row.path === materialNode).path = '/materials/runtime/other/bin/node'; },
    v => { v.runtime.elf.push({ ...structuredClone(v.runtime.elf.find(row => row.path === materialNode)),
      path: '/materials/lib/zryna/bootstrap/worker.mjs' }); },
    v => { v.runtime.roots = [materialNode]; },
    v => { v.materials.at(-1).path = '/materials/runtime/other/bin/node'; },
    v => { v.materials.at(-1).mode = 0o444; },
    v => { v.materials.at(-1).bytes++; },
    v => { v.materials.at(-1).sha256 = '0'.repeat(64); }]) {
    reject(v => { change(v); v.runtime.elf.sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0); }, true);
  }
});
test('all Node needed and interpreter edges resolve declared image ELF files', () => {
  for (const change of [v => { v.runtime.files = v.runtime.files.filter(file => file.path !== nodeLibrary); },
    v => { v.runtime.elf = v.runtime.elf.filter(row => row.path !== nodeLibrary); },
    v => { v.runtime.elf.find(row => row.path === materialNode).interpreter = '/missing/loader'; },
    v => { v.runtime.elf.find(row => row.path === materialNode).interpreter = materialNode; },
    v => { v.runtime.elf.find(row => row.path === materialNode).needed[0].path = '/materials/lib/zryna/bootstrap/worker.mjs'; },
    v => { v.runtime.elf.find(row => row.path === materialNode).needed[0].path = materialNode; }]) reject(change, true);
});
test('the additional root does not admit disconnected ELF or unclassified image files', () => {
  for (const elf of [false, true]) reject(v => {
    const path = '/usr/lib/disconnected-fixture.so';
    v.runtime.files.push({ ...selected(path), mode: 0o555, kind: 'regular', nlink: 1, uid: 0, gid: 0 });
    if (elf) v.runtime.elf.push({ ...structuredClone(v.runtime.elf.find(row => row.path === nodeLibrary)), path });
    for (const rows of [v.runtime.files, v.runtime.elf]) rows.sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  }, true);
});
test('the material Node remains forbidden in the mount-shadowed image inventory', () => {
  const value = fixture(), node = value.materials.find(file => file.path === materialNode);
  value.runtime.files.push({ ...node, kind: 'regular', nlink: 1, uid: 0, gid: 0 });
  value.runtime.files.sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  value.expected.runtimeSha256 = sha256(Buffer.from(canonical(value.runtime)));
  assert.throws(() => createLifecycleOwnerPlan(value), /runtime cannot be shadowed by mounts/);
});
