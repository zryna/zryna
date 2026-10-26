// Existing owner funding is a private invariant, independent of source quota and not a source witness.
import assert from 'node:assert/strict';
const layout=Object.freeze({stride:4,alignment:4,id:1});
const positive=Object.freeze({capacity:1});
let entered=0,allow=false;
function $vecReserve(_layout,storage){entered++;if(!allow)throw new Error('unfunded canonical entry');return Object.freeze({status:0,handle:storage});}
function $vecFree(){entered++;return 0;}
function status(code){assert.equal(code,0);}
const $status_vecReserve=status,$status_vecFree=status;
function state(ctx){return {A:ctx.A,L:ctx.L,S:ctx.S,H:ctx.H,pending:ctx.pending,positive:ctx.positive,debt:ctx.debt};}
const cases=[
 {pre:{A:0,L:0,S:0,H:0},storage:0,required:0},
 {pre:{A:1,L:1,S:1,H:0},storage:positive,required:1},
 {pre:{A:1,L:0,S:1,H:1},storage:positive,required:1}
];
for(const row of cases){
 const ctx=$accountStart();Object.assign(ctx,row.pre);const before=state(ctx);entered=0;
 assert.throws(()=>$accountVecReserve(layout,row.storage,row.required,ctx),error=>error.message==='ZRYNA-RT-ABI');
 assert.equal(entered,0);assert.deepEqual(state(ctx),before);
 // Deliberately corrupt seeded context: test reset carries no source cleanup authority.
 $accountActive=null;
}
allow=true;
for(const storage of [0,positive]){
 const ctx=$accountStart();Object.assign(ctx,{A:1,L:storage===0?0:1,S:1,H:1});entered=0;
 assert.equal($accountVecReserve(layout,storage,storage===0?0:1,ctx),storage);
 assert.deepEqual(state(ctx),{A:1,L:storage===0?0:1,S:2,H:1,pending:false,positive:0,debt:0});
 assert.equal(entered,1);$vecStorageFree(layout,storage,ctx);
 assert.deepEqual(state(ctx),{A:1,L:0,S:3,H:0,pending:false,positive:0,debt:0});$accountExit(ctx);
}
console.log(JSON.stringify({unfundedCases:cases.length,fundedCases:2,seededPrivateCounterEvidence:true,
 evidence:'real private reserve boundary, genuine retained Vec role, synthetic corrupt/funded numeric prestates only'}));
