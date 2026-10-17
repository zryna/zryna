// Fixed trusted Linux CI integration; no Code may run before the root-owned grant.
import assert from 'node:assert/strict';
import {lstat,writeFile} from 'node:fs/promises';
import {join,resolve,dirname} from 'node:path';
import {setTimeout as delay} from 'node:timers/promises';
import {descriptor,ordinaryRoot,readBytes} from './files.mjs';
import {remaining} from './process.mjs';
export async function awaitLinuxUserns({output,stage,editor,deadline}) {
 assert.equal(process.platform,'linux');
 const root=process.env.ZRYNA_M6_EDITOR_LINUX_GATE;
 assert(root&&root===resolve(root)&&/^m6-editor-linux-[1-9][0-9]*-[1-9][0-9]*$/.test(root.split('/').at(-1)));
 await ordinaryRoot(root);const parent=await lstat(root);
 assert(parent.uid===process.getuid()&&parent.gid===process.getgid()&&(parent.mode&0o777)===0o700);
 assert(stage.startsWith(join(root,'tmp','m6e-')));
 const prepared=await descriptor(join(output,'PREPARED.json'),deadline);
 const helper=await descriptor(join(dirname(editor.executable),'chrome-sandbox'),deadline);
 const request={format:'zryna.m6-editor-linux-userns-request.v1',root,output,stage,invocationId:process.env.INVOCATION_ID,uid:process.getuid(),gid:process.getgid(),
  productCommit:editor.productCommit,prepared,code:editor.descriptor,helper};
 assert(/^[a-f0-9]{32}$/.test(request.invocationId));
 await writeFile(join(root,'REQUEST.json'),JSON.stringify(request),{flag:'wx',mode:0o400});
 const requestPin=await descriptor(join(root,'REQUEST.json'),deadline);
 await writeFile(join(root,'REQUEST.ready'),'',{flag:'wx',mode:0o400});
 for(;;){
  remaining(deadline);
  try{
   const marker=await lstat(join(root,'GRANT.ready'));assert(marker.isFile()&&!marker.isSymbolicLink()&&marker.nlink===1&&marker.uid===0&&marker.gid===0&&(marker.mode&0o777)===0o444&&marker.size===0);
   const path=join(root,'GRANT.json'),s=await lstat(path);
   assert(s.isFile()&&!s.isSymbolicLink()&&s.nlink===1&&s.uid===0&&(s.mode&0o777)===0o444&&s.size<=4096);
   const grant=JSON.parse(await readBytes(path,deadline,4096));
   assert.deepEqual(Object.keys(grant),['format','requestSha256','unit','invocationId','cgroup']);
   assert.equal(grant.format,'zryna.m6-editor-linux-userns-grant.v1');assert.equal(grant.requestSha256,requestPin.sha256);
   assert(/^m6-editor-linux-[1-9][0-9]*-[1-9][0-9]*-[a-f0-9]{12}\.service$/.test(grant.unit));
   assert.equal(grant.invocationId,request.invocationId);
   assert.deepEqual(Object.keys(grant.cgroup),['path','dev','ino']);
   assert.equal(grant.cgroup.path,'/sys/fs/cgroup/system.slice/'+grant.unit);
   assert(Number.isSafeInteger(grant.cgroup.dev)&&Number.isSafeInteger(grant.cgroup.ino)&&grant.cgroup.ino>0);
   return;
  }catch(e){if(e.code!=='ENOENT')throw e;}
  await delay(25);
 }
}
