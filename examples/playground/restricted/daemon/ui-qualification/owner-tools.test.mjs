import assert from 'node:assert/strict';
import test from 'node:test';
import { chmodSync,mkdtempSync,readFileSync,readdirSync,rmSync,statSync,symlinkSync,writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { captureOwnerTools,helperNames } from './owner-tools.mjs';
import { sha256 } from './wire.mjs';
function fixture(){
  const root=mkdtempSync('/tmp/ui-tools-test-');let index=0;
  function pin(bytes,executable=false){
    const path=join(root,String(index++));writeFileSync(path,bytes,{mode:executable?0o500:0o400});
    return{path,bytes:bytes.length,sha256:sha256(bytes),ownerUid:process.getuid()};
  }
  const elf=Buffer.alloc(64);elf.set([127,69,76,70,2,1,1]);elf.writeUInt16LE(3,16);elf.writeUInt16LE(62,18);
  const files=helperNames.map(name=>({name,...pin(Buffer.from('# inert helper '+name))}));
  const names=['os.py','socket.py','selectors.py','stat.py','pathlib.py','json/__init__.py','encodings/__init__.py',
    'encodings/utf_8.py','encodings/ascii.py','http/client.py',...Array.from({length:10},(_,i)=>'fixture'+i+'.py')];
  return{root,policy:{python:pin(elf,true),files,runtime:{version:'3.12',files:names.map(name=>({
    name:'lib/python3.12/'+name,...pin(Buffer.from('# inert stdlib '+name)),executable:false}))}}};
}
test('tool capture snapshots selected interpreter, helpers and explicit private stdlib without execution',()=>{
  const f=fixture();let captured;
  try{
    captured=captureOwnerTools(f.policy);
    assert.equal(captured.interpreter,join(captured.root,'bin/python3.12'));
    assert.equal(statSync(join(captured.root,'lib/python3.12/lib-dynload')).isDirectory(),true);
    assert.deepEqual(readdirSync(join(captured.root,'lib/python3.12/lib-dynload')),[]);
    assert.equal(readFileSync(join(captured.root,'lib/python3.12/os.py'),'utf8'),'# inert stdlib os.py');
    chmodSync(f.policy.files[0].path,0o600);writeFileSync(f.policy.files[0].path,'substitution');
    assert.equal(readFileSync(join(captured.root,helperNames[0]),'utf8'),'# inert helper '+helperNames[0]);
  }finally{captured?.cleanup();rmSync(f.root,{recursive:true,force:true});}
});
test('exact empty trusted stdlib initializer is preserved as zero bytes without changing source',()=>{
  const f=fixture();let captured;
  try{
    const path=join(f.root,'empty-init');writeFileSync(path,Buffer.alloc(0),{mode:0o400});
    f.policy.runtime.files.push({name:'lib/python3.12/urllib/__init__.py',path,bytes:0,
      sha256:sha256(Buffer.alloc(0)),ownerUid:process.getuid(),executable:false});
    captured=captureOwnerTools(f.policy);
    assert.equal(readFileSync(join(captured.root,'lib/python3.12/urllib/__init__.py')).length,0);
    assert.equal(readFileSync(path).length,0);
    chmodSync(path,0o600);writeFileSync(path,'later mutation');
    assert.equal(readFileSync(join(captured.root,'lib/python3.12/urllib/__init__.py')).length,0);
  }finally{captured?.cleanup();rmSync(f.root,{recursive:true,force:true});}
});
test('missing stdlib, duplicate modules, ambient packages, non-ELF and symlink substitution fail closed',()=>{
  const mutations=[p=>{delete p.runtime;},p=>{p.runtime.files[1]=p.runtime.files[0];},
    p=>{p.runtime.files[0].name='lib/python3.12/site-packages/injected.py';},
    p=>{chmodSync(p.python.path,0o700);writeFileSync(p.python.path,'#!/bin/sh\nexit 0\n');
      p.python.bytes=17;p.python.sha256=sha256(Buffer.from('#!/bin/sh\nexit 0\n'));},
    p=>{const path=p.files[0].path+'.link';symlinkSync(p.files[0].path,path);p.files[0].path=path;}];
  for(const mutate of mutations){const f=fixture();try{mutate(f.policy);assert.throws(()=>captureOwnerTools(f.policy));}
    finally{rmSync(f.root,{recursive:true,force:true});}}
});
