// Private constructor observation of actual emitted code; this does not execute the source entry.
import assert from 'node:assert/strict';
const keys=['tag','payload','$dropParent','$dropType','$dropIndex'];
let prototypeCallbacks=0,prototypeMutations=0;
function noPrototypeMutation(body) {
 const original=Object.setPrototypeOf;
 Object.setPrototypeOf=()=>{prototypeMutations++;throw Error('Enum construction must not rewrite a prototype');};
 try{return body();}finally{Object.setPrototypeOf=original;}
}
const previous=keys.map(key=>[key,Object.getOwnPropertyDescriptor(Object.prototype,key)]);
for(const key of keys) Object.defineProperty(Object.prototype,key,{configurable:true,get(){prototypeCallbacks++;throw Error('inherited getter');},set(){prototypeCallbacks++;throw Error('inherited setter');}});
function shape(owner,tag,payload) {
 assert.equal(Object.getPrototypeOf(owner),null);
 assert.deepEqual(Reflect.ownKeys(owner),keys);
 assert.deepEqual(Object.values(owner),[tag,payload,null,-1,-1]);
 for(const key of keys) {
  const descriptor=Object.getOwnPropertyDescriptor(owner,key);
  assert.deepEqual(Object.keys(descriptor),['value','writable','enumerable','configurable']);
  assert.equal(descriptor.writable,true);assert.equal(descriptor.enumerable,true);assert.equal(descriptor.configurable,true);
 }
}
try {
 const leaf=Object.freeze(Object.create(null));
 const trace=[];
 const first=noPrototypeMutation(()=>$enumOwner((trace.push('tag'),$actualTag),(trace.push('payload'),leaf)));
 assert.deepEqual(trace,['tag','payload']);shape(first,$actualTag,leaf);
 const second=noPrototypeMutation(()=>$enumOwner($actualTag,leaf));shape(second,$actualTag,leaf);
 assert.notEqual(first,second);assert.equal(first.payload,leaf);assert.equal(second.payload,leaf);
 const transported=noPrototypeMutation(()=>$transportOwner(first));shape(transported,$actualTag,leaf);
 assert.notEqual(transported,first);assert.equal(transported.payload,leaf);
 const reads=[];
 const getterSource=Object.defineProperties(Object.create(null),{
  tag:{get(){reads.push('tag');return $actualTag;}},
  payload:{get(){reads.push('payload');return leaf;}}
 });
 shape(noPrototypeMutation(()=>$transportOwner(getterSource)),$actualTag,leaf);assert.deepEqual(reads,['tag','payload']);

 first.$dropParent=second;first.$dropType=19;first.$dropIndex=3;
 shape(second,$actualTag,leaf);shape(transported,$actualTag,leaf);
 first.$dropParent=null;first.$dropType=-1;first.$dropIndex=-1;shape(first,$actualTag,leaf);
 const failure=Object.freeze(Object.create(null));const evaluation=[];
 assert.throws(()=>$enumOwner((evaluation.push('tag'),$actualTag),(()=>{evaluation.push('payload');throw failure;})()),error=>error===failure);
 assert.deepEqual(evaluation,['tag','payload']);
 shape(noPrototypeMutation(()=>$enumOwner(0,undefined)),0,undefined);
 assert.equal(prototypeCallbacks,0);assert.equal(prototypeMutations,0);
} finally {
 for(const [key,descriptor] of previous) {
  if(descriptor) Object.defineProperty(Object.prototype,key,descriptor);
  else delete Object.prototype[key];
 }
}
console.log(JSON.stringify({constructorRepresentationOnly:true,sourceRuntimeConformance:false,prototypeCallbacks,prototypeMutations}));
