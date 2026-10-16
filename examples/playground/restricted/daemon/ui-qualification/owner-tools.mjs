// Independently selected operator material pins; private snapshots never authenticate themselves.
import { closeSync, chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { capture, stable } from '../bridge-material.mjs';
import { exact, fail, sha256 } from './wire.mjs';
import { captureEmptyPython } from './owner-empty.mjs';
export const helperNames=Object.freeze(['engine.py','plan.py','owner_main.py','owner_relay.py',
  'owner_lifecycle.py','owner_attach.py','owner_config.py','owner_identity.py','owner_audit.py','owner_kernel.py',
  'owner_proc.py','owner_cgroup.py','owner_mounts.py']);
export function captureOwnerTools(policy) {
  exact(policy,['python','files','runtime']);
  exact(policy.runtime,['version','files']);
  if(policy.runtime.version!=='3.12' || !Array.isArray(policy.runtime.files) ||
    policy.runtime.files.length<20 || policy.runtime.files.length>1024)fail();
  if(!Array.isArray(policy.files) || policy.files.length!==helperNames.length)fail();
  const captured=[];let root;
  try{
    const python=capture(policy.python,true);captured.push(python);
    const bytes=readFileSync(python.fd);stable(python);
    if(bytes.length!==policy.python.bytes || sha256(bytes)!==policy.python.sha256 || bytes.length<64 ||
      !bytes.subarray(0,7).equals(Buffer.from([127,69,76,70,2,1,1])) ||
      ![2,3].includes(bytes.readUInt16LE(16)) || bytes.readUInt16LE(18)!==62)fail();
    const entries=new Map();
    for(const file of policy.files){
      exact(file,['name','path','bytes','sha256','ownerUid']);
      if(!helperNames.includes(file.name) || entries.has(file.name))fail();
      const {name,...selected}=file,material=capture(selected,false);captured.push(material);
      const data=readFileSync(material.fd);stable(material);
      if(data.length!==selected.bytes || sha256(data)!==selected.sha256)fail();
      entries.set(name,data);
    }
    root=mkdtempSync('/tmp/zryna-ui-owner-');mkdirSync(join(root,'bin'),{mode:0o700});
    const interpreter=join(root,'bin/python3.12');writeFileSync(interpreter,bytes,{flag:'wx',mode:0o500});chmodSync(interpreter,0o500);
    // CPython getpath needs this directory even when native extensions are built in.
    mkdirSync(join(root,'lib/python3.12/lib-dynload'),{recursive:true,mode:0o700});
    const names=new Set();let total=bytes.length;
    for(const file of policy.runtime.files){
      exact(file,['name','path','bytes','sha256','ownerUid','executable']);
      const {name,executable,...selected}=file;
      if(typeof executable!=='boolean' || typeof name!=='string' ||
        !/^lib\/python3\.12\/(?:[A-Za-z0-9_-]+\/)*[A-Za-z0-9_.-]+\.(?:py|so)$/.test(name) ||
        name.includes('/site-packages/') || name.includes('/__pycache__/') || names.has(name) ||
        executable!==name.endsWith('.so') || (total+=selected.bytes)>134217728)fail();
      names.add(name);const material=selected.bytes===0&&!executable
        ?captureEmptyPython(selected):capture(selected,executable);captured.push(material);
      const data=readFileSync(material.fd);stable(material);
      if(data.length!==selected.bytes || sha256(data)!==selected.sha256)fail();
      mkdirSync(dirname(join(root,name)),{recursive:true,mode:0o700});
      writeFileSync(join(root,name),data,{flag:'wx',mode:executable?0o500:0o400});
    }
    for(const name of ['os.py','socket.py','selectors.py','stat.py','pathlib.py','json/__init__.py',
      'encodings/__init__.py','encodings/utf_8.py','encodings/ascii.py','http/client.py'])
      if(!names.has('lib/python3.12/'+name))fail();
    for(const [name,data]of entries)writeFileSync(join(root,name),data,{flag:'wx',mode:0o400});
    for(const material of captured)stable(material);
    return Object.freeze({root,interpreter,entry:join(root,'owner_main.py'),
      cleanup(){rmSync(root,{recursive:true,force:false});}});
  }catch(error){if(root)rmSync(root,{recursive:true,force:true});throw error;}
  finally{for(const material of captured.reverse())closeSync(material.fd);}
}
