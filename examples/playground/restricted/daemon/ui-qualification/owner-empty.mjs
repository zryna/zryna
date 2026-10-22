// Exact empty stdlib package initializers; production material policy stays unchanged.
import { closeSync,constants,fstatSync,openSync,readSync } from 'node:fs';
import { isAbsolute,posix } from 'node:path';
import { exact,fail,hash } from './wire.mjs';
const emptySha256='e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855';
const fields=['dev','ino','size','mode','uid','gid','mtimeNs','ctimeNs','nlink'];
export function captureEmptyPython(policy){
  exact(policy,['path','bytes','sha256','ownerUid']);
  if(policy.bytes!==0 || !hash(policy.sha256) || policy.sha256!==emptySha256 ||
    !Number.isInteger(policy.ownerUid) || policy.ownerUid<0 || policy.ownerUid>4294967295 ||
    !isAbsolute(policy.path) || posix.normalize(policy.path)!==policy.path ||
    !/^\/[A-Za-z0-9._/-]+$/.test(policy.path))fail();
  const ancestors=[];let fd;
  try{
    let parent=openSync('/',constants.O_RDONLY|constants.O_DIRECTORY|constants.O_NOFOLLOW);ancestors.push(parent);
    const parts=policy.path.slice(1).split('/');
    for(const part of parts.slice(0,-1)){
      parent=openSync(`/proc/self/fd/${parent}/${part}`,constants.O_RDONLY|constants.O_DIRECTORY|constants.O_NOFOLLOW);
      ancestors.push(parent);
    }
    fd=openSync(`/proc/self/fd/${parent}/${parts.at(-1)}`,constants.O_RDONLY|constants.O_NOFOLLOW|constants.O_NONBLOCK);
    const state=fstatSync(fd,{bigint:true}),afterRead=Buffer.alloc(1);
    if(!state.isFile() || state.uid!==BigInt(policy.ownerUid) || state.size!==0n || state.nlink!==1n ||
      state.mode&0o133n || readSync(fd,afterRead,0,1,0)!==0)fail();
    const after=fstatSync(fd,{bigint:true});if(fields.some(key=>state[key]!==after[key]))fail();
    const retained=fd;fd=undefined;return{fd:retained,path:policy.path,sha256:policy.sha256,state};
  }finally{
    if(fd!==undefined)closeSync(fd);
    for(const descriptor of ancestors.reverse())closeSync(descriptor);
  }
}
