import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';

const [directory,provider]=process.argv.slice(2);
const bytes=readFileSync(join(directory,'EXPECTED.json'));
assert.equal(createHash('sha256').update(bytes).digest('hex'),'df511427cbaa9b2d51fd412f36c382d1611bfa60b1e09f9a0cbb06d08eb01303');
const expected=JSON.parse(bytes);
assert(expected.providers.includes(provider));
const module=await import(pathToFileURL(join(directory,'private.mjs')).href);
assert.deepEqual(Object.keys(module).sort(),[
  'testOnlyArm','testOnlySnapshot','zryna$encodeInvocation$v1','zryna$invoke$v1',
]);
let calls=0;
function invoke(phase,code,row){
  module.testOnlyArm(code);
  const result=module.zryna$invoke$v1();
  const encoded=module.zryna$encodeInvocation$v1(result);
  const snapshot=module.testOnlySnapshot();
  const observation={provider,phase,code,wire:Array.from(encoded),snapshot};
  // Retain each actual complete child observation before comparing it with the frozen oracle.
  process.stdout.write(`${JSON.stringify(observation)}\n`);
  calls++;
  assert.equal(Object.getPrototypeOf(result),null);
  assert(Object.isFrozen(result));
  assert(encoded instanceof Uint8Array);
  assert.equal(encoded.length,expected.wire.bytes);
  const view=new DataView(encoded.buffer,encoded.byteOffset,encoded.byteLength);
  assert.deepEqual([0,4,8,12,16].map(offset=>view.getUint32(offset,true)),[
    expected.wire.magic,row.status,expected.wire.scalar_type,row.scalar,expected.wire.end,
  ]);
  assert.throws(()=>module.zryna$encodeInvocation$v1(result));
  assert.deepEqual(snapshot.acquired,row.acquired);
  assert.deepEqual(snapshot.released,row.released);
  assert.equal(snapshot.clone_preflights,row.clone_preflights);
  assert.deepEqual(snapshot.kernel_exit,row.kernel_exit);
  assert.deepEqual(snapshot.account_exit,row.account_exit);
  assert.equal(snapshot.busy,false);
  assert.equal(snapshot.account_inactive,true);
  assert.equal(snapshot.armed,0);
  assert.equal(new Set(snapshot.acquired.map(value=>value.id)).size,snapshot.acquired.length);
  assert.deepEqual(snapshot.released.map(value=>value.id).sort((a,b)=>a-b),
    snapshot.acquired.map(value=>value.id).sort((a,b)=>a-b));
}
for(const identity of expected.identities){
  invoke('pristine-before',0,expected.pristine);
  invoke('private-identity-fault',identity.status,{...expected.fault_common,status:identity.status});
  invoke('pristine-after',0,expected.pristine);
}
assert.equal(calls,expected.calls_per_provider);
process.stdout.write(`${JSON.stringify({calls,same_instance:true,typed_artifact:false,
  boundary:'private authentic identities, not five canonical/source causes',provider})}\n`);
