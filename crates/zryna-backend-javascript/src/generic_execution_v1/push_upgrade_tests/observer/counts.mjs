import assert from 'node:assert/strict';
import {state,record,release} from './state.mjs';

function control(value) {
  assert.equal(Object.getPrototypeOf(value),null);
  assert.equal(state.live.get(value.$ptr)?.kind,'Shared control');
  for(const field of ['$strong','$weak'])assert(Number.isInteger(value[field])&&value[field]>=0&&value[field]<=0xffffffff);
  assert(value.$weak>0);assert(!value.$pending);
  const previous=state.controls.get(value.$ptr);
  if(previous)assert.equal(previous,value);else state.controls.set(value.$ptr,value);
}
function increment(value,operation,strong) {
  control(value);if(strong)assert(value.$strong>0);
  const field=strong?'$strong':'$weak';
  if(value[field]===0xffffffff)return 3;
  value[field]++;state.countEvents.push(operation);return 0;
}
export const counts={
  strongClone(value){return increment(value,'strongClone',true);},
  weakClone(value){return increment(value,'weakClone',false);},
  weakDowngrade(value){
    control(value);assert(value.$strong>0);
    if(state.fault.kind==='weak-downgrade-status'){state.countEvents.push('weakDowngrade:failure');return 3;}
    return increment(value,'weakDowngrade',false);
  },
  weakUpgrade(value){
    control(value);
    if(state.fault.kind==='weak-upgrade-status'){state.countEvents.push('weakUpgrade:failure');return 3;}
    if(value.$strong===0){state.countEvents.push('weakUpgrade:expired');return 5;}
    if(value.$strong===0xffffffff)return 3;
    value.$strong++;state.countEvents.push('weakUpgrade:live');return 0;
  },
  strongReleaseBegin(value){
    control(value);assert(value.$strong>0);value.$strong--;value.$pending=value.$strong===0;
    state.countEvents.push(`strongReleaseBegin:${value.$pending?'last':'notLast'}`);
    return record({status:0,value:value.$pending});
  },
  strongReleaseFinish(value){
    assert.equal(state.controls.get(value.$ptr),value);assert(state.live.has(value.$ptr));
    assert(value.$pending&&value.$strong===0&&value.$weak>0);
    value.$pending=false;value.$weak--;
    if(value.$weak===0){release(value.$ptr,'Shared control');state.controls.delete(value.$ptr);}
    state.countEvents.push(`strongReleaseFinish:${value.$weak===0?'deallocated':'retained'}`);return 0;
  },
  weakRelease(value){
    control(value);assert(value.$weak>(value.$strong>0?1:0));
    value.$weak--;state.countEvents.push('weakRelease');const deallocated=value.$weak===0;
    if(deallocated){assert.equal(value.$strong,0);release(value.$ptr,'Shared control');state.controls.delete(value.$ptr);}
    return record({status:0,value:deallocated});
  },
};
