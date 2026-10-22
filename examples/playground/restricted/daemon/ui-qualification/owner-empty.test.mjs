import assert from 'node:assert/strict';
import test from 'node:test';
import { closeSync,linkSync,mkdirSync,mkdtempSync,rmSync,symlinkSync,writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { captureEmptyPython } from './owner-empty.mjs';
import { sha256 } from './wire.mjs';
test('empty stdlib capture rejects changed bytes, false pins, links and executable initializers',()=>{
  for(const kind of ['nonempty','wronghash','symlink','ancestor-symlink','hardlink','executable','wrongowner']){
    const root=mkdtempSync('/tmp/empty-stdlib-test-');
    try{
      let path=join(root,'init.py');writeFileSync(path,kind==='nonempty'?'x':'',{mode:kind==='executable'?0o500:0o400});
      if(kind==='symlink'){symlinkSync(path,join(root,'link.py'));path=join(root,'link.py');}
      if(kind==='ancestor-symlink'){
        mkdirSync(join(root,'actual'));writeFileSync(join(root,'actual/init.py'),'');
        symlinkSync(join(root,'actual'),join(root,'alias'));path=join(root,'alias/init.py');
      }
      if(kind==='hardlink')linkSync(path,join(root,'alias.py'));
      const policy={path,bytes:0,sha256:kind==='wronghash'?'0'.repeat(64):sha256(Buffer.alloc(0)),
        ownerUid:process.getuid()+(kind==='wrongowner'?1:0)};
      assert.throws(()=>{const material=captureEmptyPython(policy);closeSync(material.fd);});
    }finally{rmSync(root,{recursive:true,force:true});}
  }
});
