// Existing five-word String protocol: exact bytes, live identities, releases and interleaving.
import assert from 'node:assert/strict';
export function check(getter, run, fault) {
  assert.equal(getter(0), fault ? 2 : 0);
  const count=getter(0x40000000);
  assert(Number.isInteger(count)&&count>=0&&count<=4096,'first-extra4097 remains rejected');
  const words=Array.from({length:count},(_,index)=>getter(0x40000001+index)>>>0);
  const live=new Map(),acquired=[],released=[],events=[],failures=[];
  const prefix=fault ? fault.successful_acquisitions : run.acquisitions.length;
  for(let cursor=0;cursor<words.length;) {
    assert(cursor+5<=words.length,'whole trace header');
    const [tag,id,pointer,source,length]=words.slice(cursor,cursor+5);cursor+=5;
    assert(length<=words.length-cursor,'whole byte payload');
    const bytes=words.slice(cursor,cursor+length);cursor+=length;
    assert(bytes.every(byte=>byte<=255));
    const hex=Buffer.from(bytes).toString('hex');
    if(tag===1||tag===2) {
      assert.equal(id,acquired.length+1);assert(id<=prefix);assert.notEqual(pointer,0);
      assert([...live.values()].every(row=>row.pointer!==pointer),'no live allocation alias');
      const expected=run.acquisitions[id-1];assert.equal(hex,expected.utf8_hex);
      if(expected.source_ids.length===1) {
        assert.equal(tag,2);assert.equal(source,live.get(expected.source_ids[0])?.pointer);
      } else {
        // Existing concat success uses tag1/source0; no second source field is invented.
        assert.equal(tag,1);assert.equal(source,0);
      }
      live.set(id,{pointer,hex});acquired.push(id);events.push(`acquire:${id}`);
    } else if(tag===3) {
      assert.equal(pointer,live.get(id)?.pointer);assert.equal(hex,live.get(id)?.hex);
      assert.equal(source,0);live.delete(id);released.push(id);events.push(`release:${id}`);
    } else {
      assert(fault);assert([4,5].includes(tag));assert.equal(pointer,0);
      assert.equal(tag,fault.kind==='allocation'?4:5);assert.equal(id,fault.ordinal);
      failures.push(tag);events.push(`failure:${fault.kind}:${fault.ordinal}`);
    }
  }
  const expected=fault??run.pristine;
  assert.equal(live.size,0,'every actual successful allocation released exactly once');
  assert.deepEqual(acquired,Array.from({length:prefix},(_,index)=>index+1));
  assert.deepEqual(released,expected.release_acquisition_ids);
  const silentConcatFailure=fault?.kind==='allocation'
    &&run.acquisitions[fault.ordinal-1].source_ids.length===2;
  assert.equal(failures.length,fault&&!silentConcatFailure?1:0);
  assert.deepEqual(events,silentConcatFailure ? expected.events.filter(event=>!event.startsWith('failure:')) : expected.events);
  return {acquired,released,events,actualAcquisitionsReleasedExactlyOnce:true};
}
