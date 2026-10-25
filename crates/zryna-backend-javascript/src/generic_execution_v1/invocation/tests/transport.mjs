import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {join} from 'node:path';
const [directory,code,bits,instrument]=process.argv.slice(2);
const load=name=>import(pathToFileURL(join(directory,name)).href);
const ordinary=await load('ordinary.mjs');
assert.deepEqual(Object.keys(ordinary).sort(),['zryna$encodeInvocation$v1','zryna$invoke$v1']);
function wire(module,result,status,scalar){
  assert.equal(Object.getPrototypeOf(result),null);assert(Object.isFrozen(result));
  const bytes=module.zryna$encodeInvocation$v1(result);assert(bytes instanceof Uint8Array);assert.equal(bytes.length,20);
  const view=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength);
  assert.deepEqual([0,4,8,12,16].map(offset=>view.getUint32(offset,true)),[0x3149475a,status,1,scalar,0x31444e45]);
  assert.throws(()=>module.zryna$encodeInvocation$v1(result));
}
wire(ordinary,ordinary.zryna$invoke$v1(),Number(code),Number(bits));
const stale=ordinary.zryna$invoke$v1(),current=ordinary.zryna$invoke$v1();
assert.throws(()=>ordinary.zryna$encodeInvocation$v1(stale));assert.throws(()=>ordinary.zryna$encodeInvocation$v1(current));
const beforeBadArity=ordinary.zryna$invoke$v1();assert.throws(()=>ordinary.zryna$invoke$v1(1));assert.throws(()=>ordinary.zryna$encodeInvocation$v1(beforeBadArity));
const authentic=ordinary.zryna$invoke$v1();assert.throws(()=>ordinary.zryna$encodeInvocation$v1(Object.freeze({...authentic})));assert.throws(()=>ordinary.zryna$encodeInvocation$v1(authentic));
wire(ordinary,ordinary.zryna$invoke$v1(),Number(code),Number(bits));
if(instrument==='yes'){
  const busy=await load('private-reentry.mjs'),beforeReentry=busy.zryna$invoke$v1();
  busy.privateBusy(true);assert.throws(()=>busy.zryna$invoke$v1());busy.privateBusy(false);
  assert.throws(()=>busy.zryna$encodeInvocation$v1(beforeReentry));wire(busy,busy.zryna$invoke$v1(),0,14);
  // These five modules are explicit test-only instrumentation; no source reachability claim.
  for(let status=1;status<=5;status++){
    const module=await load(`private-trap-${status}.mjs`);wire(module,module.zryna$invoke$v1(),status,0);
  }
  const forged=await load('forged.mjs');assert.throws(()=>forged.zryna$invoke$v1(),error=>error instanceof Error&&error.message==='zryna.trap.bounds-v1');
  assert.throws(()=>forged.zryna$encodeInvocation$v1({status:1,type:1,scalar:0}));
}
process.stdout.write('carrier checks passed\n');
