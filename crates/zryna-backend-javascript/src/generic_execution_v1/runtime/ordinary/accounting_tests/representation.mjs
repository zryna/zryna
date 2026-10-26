import assert from 'node:assert/strict';
let poisonCalls=0;
Object.defineProperty(Object.prototype,'A',{configurable:true,get(){poisonCalls++;throw new Error('inherited getter');},set(){poisonCalls++;throw new Error('inherited setter');}});
try {
 const ctx=$accountStart();
 assert.equal(Object.getPrototypeOf(ctx),null);assert.equal(Object.isSealed(ctx),true);
 assert.deepEqual(Object.keys(ctx),['brand','A','L','S','H','pending','positive','debt']);
 for(const key of Object.keys(ctx)){
  const descriptor=Object.getOwnPropertyDescriptor(ctx,key);
  assert.equal(descriptor.configurable,false);assert.equal(descriptor.enumerable,true);assert.equal(descriptor.writable,true);assert.equal('value' in descriptor,true);
 }
 assert.equal(ctx.brand,$accountBrand);assert.equal(Object.isFrozen(ctx.brand),true);
 assert.deepEqual([ctx.A,ctx.L,ctx.S,ctx.H,ctx.pending,ctx.positive,ctx.debt],[0,0,0,0,false,0,0]);
 assert.throws(()=>$accountStart(),/ZRYNA-RT-ABI/);
 const foreign=Object.assign(Object.create(null),ctx);
 assert.throws(()=>$accountAdmit(foreign,0,1),/ZRYNA-RT-ABI/);
 $accountAdmit(ctx,1,1);$accountResult(ctx,true);$accountRelease(ctx,1);$accountReleased(ctx);
 assert.deepEqual([ctx.A,ctx.L,ctx.S,ctx.H],[1,0,2,0]);$accountExit(ctx);
 const retry=$accountStart();assert.notEqual(retry,ctx);assert.equal(Object.getPrototypeOf(retry),null);$accountExit(retry);
 assert.equal(poisonCalls,0);
} finally { delete Object.prototype.A; }
console.log(JSON.stringify({samePrivateContextContract:true,prototypeGetterSetterCalls:poisonCalls,representationOnly:true}));
