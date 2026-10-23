// Explicit root-approved new-domain observed artifact; no old artifact patch or memory export.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
export async function load(ordinaryPath, observedPath) {
  const ordinary = new WebAssembly.Module(readFileSync(ordinaryPath));
  assert.deepEqual(WebAssembly.Module.imports(ordinary),[]);
  assert.deepEqual(WebAssembly.Module.exports(ordinary).map(row=>[row.name,row.kind]),[['entry','function']]);
  const observed = new WebAssembly.Module(readFileSync(observedPath));
  assert.deepEqual(WebAssembly.Module.imports(observed),[]);
  assert.deepEqual(WebAssembly.Module.exports(observed).map(row=>[row.name,row.kind]).sort(),[['$zryna$observation','function'],['entry','function']]);
  const instance = new WebAssembly.Instance(observed,{}), getter=instance.exports.$zryna$observation;
  let fault=null;
  return { entry:instance.exports.entry,
    reset(value) { fault=value; const kind=value?.kind === 'allocation' ? 2 : value?.kind === 'clone' ? 6 : 0; assert.equal(getter((0x20000000 | (kind<<24) | (value?.ordinal ?? 1))|0),0); },
    finish() {
      assert.equal(getter(0),fault ? 2 : 0);
      const count=getter(0x40000000); assert(Number.isInteger(count) && count>=0 && count<=4096,'first-extra4097 trace overflow rejects');
      const words=Array.from({length:count},(_,index)=>getter(0x40000001+index)>>>0),records=[];
      for(let index=0;index<words.length;) {
        assert(index+5<=words.length,'whole trace header');
        const [tag,id,pointer,source,length]=words.slice(index,index+5); index+=5;
        assert([1,2,3,4,5].includes(tag)); assert(length<=words.length-index,'whole observed byte record');
        const bytes=words.slice(index,index+length); index+=length; assert(bytes.every(byte=>byte<=255));
        records.push({tag,id,identity:pointer.toString(16),sourceIdentity:source.toString(16),length,bytes_hex:Buffer.from(bytes).toString('hex')});
      }
      return {records};
    },
  };
}
