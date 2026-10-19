// Fixed trusted operator; no public or guest command selector. All fixture files removed.
import {openSync,writeSync,closeSync,unlinkSync,statSync,statfsSync,readdirSync,readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
const role=process.argv[1];assert(['compiler','evaluator'].includes(role));
const cases=[['/tmp',role==='compiler'?67108864:134217728],['/output',2097152],['/dev/shm',role==='compiler'?1048576:67108864]];
const rows=[],buffer=Buffer.alloc(1048576,90);
for(const [path,limit]of cases){
 const before=statfsSync(path);assert.equal(before.type,0x01021994);assert.equal(before.bsize*before.blocks,limit);assert.equal(before.bsize*before.bavail,limit);assert.equal(readdirSync(path).length,0);
 const name=path+'/fixed-boundary-fixture',fd=openSync(name,'wx',0o600);let bytes=0,extraError;
 try{
  while(bytes<limit){const n=writeSync(fd,buffer,0,Math.min(buffer.length,limit-bytes));assert(n>0);bytes+=n}
  assert.equal(statSync(name).size,limit);assert.equal(statfsSync(path).bavail,0);
  try{writeSync(fd,Buffer.from([91]))}catch(error){extraError=error.code}assert.equal(extraError,'ENOSPC');assert.equal(statSync(name).size,limit);
 }finally{closeSync(fd);unlinkSync(name)}
 const after=statfsSync(path);assert.equal(after.bavail,before.bavail);assert.equal(readdirSync(path).length,0);
 rows.push({path,limit,acceptedBytes:bytes,firstExtraError:extraError,restoredEmpty:true});
}
let inputError;try{const fd=openSync('/input/fixed-boundary-fixture','wx',0o600);closeSync(fd)}catch(error){inputError=error.code}assert.equal(inputError,'EROFS');assert.equal(readdirSync('/input').length,0);const input=statfsSync('/input');assert.equal(input.type,0x01021994);assert.equal(input.blocks*input.bsize,2097152);
console.log(JSON.stringify({status:'actual-original-role-storage-boundaries-pass',role,rows,input:{bytes:2097152,guestWriteError:inputError,emptyBeforeSource:true}}));
