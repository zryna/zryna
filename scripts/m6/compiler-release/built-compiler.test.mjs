import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { syncBuiltinESMExports } from 'node:module';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { captureBuiltCompiler } from './built-compiler.mjs';
import { ordinary, PRODUCT, TARGET } from './common.mjs';

function fixture() {
  const root = fs.mkdtempSync(join(tmpdir(),'compiler-output-'));
  const target = join(root,'target'), binary = join(target,TARGET,'release',PRODUCT);
  fs.mkdirSync(dirname(binary),{recursive:true});
  fs.writeFileSync(binary,Buffer.from('INERT BUILD OUTPUT FIXTURE\n'));
  return {root,target,binary};
}

test('Cargo hardlinked output is captured without weakening ordinary publication material admission',()=>{
  const f = fixture();
  try {
    const alias = join(dirname(f.binary),'deps-entry'); fs.linkSync(f.binary,alias);
    const before = fs.statSync(f.binary,{bigint:true});
    assert.equal(before.nlink,2n);
    assert.throws(()=>ordinary(f.binary),/ordinary file/);
    const captured = captureBuiltCompiler(f.target);
    assert.deepEqual(captured,fs.readFileSync(alias));
    const after = fs.statSync(f.binary,{bigint:true});
    for (const key of ['dev','ino','size','mode','nlink','mtimeNs','ctimeNs']) assert.equal(before[key],after[key]);
    const published = join(f.root,'compiler.bin'); fs.writeFileSync(published,captured,{flag:'wx'});
    assert.equal(fs.statSync(published).nlink,1); assert.deepEqual(ordinary(published),captured);
  } finally { fs.rmSync(f.root,{recursive:true,force:true}); }
});

test('fixed output capture rejects symbolic-link files and ancestors and closes retained descriptors',()=>{
  const f = fixture();
  try {
    const count = ()=>fs.readdirSync('/proc/self/fd').length;
    const before = count(), alias = join(f.root,'target-link'); fs.symlinkSync(f.target,alias);
    assert.throws(()=>captureBuiltCompiler(alias)); assert.equal(count(),before);
    const original = join(f.root,'original'); fs.renameSync(f.binary,original); fs.symlinkSync(original,f.binary);
    assert.throws(()=>captureBuiltCompiler(f.target)); assert.equal(count(),before);
  } finally { fs.rmSync(f.root,{recursive:true,force:true}); }
});

test('empty and oversized build outputs refuse before reading a binary',()=>{
  const f = fixture();
  try {
    fs.truncateSync(f.binary,0); assert.throws(()=>captureBuiltCompiler(f.target),/completed build output/);
    fs.truncateSync(f.binary,128*1024*1024+1);
    assert.throws(()=>captureBuiltCompiler(f.target),/completed build output/);
  } finally { fs.rmSync(f.root,{recursive:true,force:true}); }
});

test('mutation through a Cargo alias during capture cannot be adopted as a new output identity',()=>{
  const f = fixture(), originalRead = fs.readSync;
  try {
    const alias = join(dirname(f.binary),'deps-entry'); fs.linkSync(f.binary,alias);
    const originalBytes = fs.readFileSync(alias); let mutated = false;
    fs.readSync = function(fd,...args) {
      const count = originalRead.call(this,fd,...args);
      if (!mutated && count > 0) {
        mutated = true; fs.writeFileSync(alias,'CHANGED FIXTURE'); fs.writeFileSync(alias,originalBytes);
        fs.utimesSync(alias,0,0);
      }
      return count;
    };
    syncBuiltinESMExports();
    assert.throws(()=>captureBuiltCompiler(f.target),/stable build output identity/);
    assert.equal(mutated,true); assert.deepEqual(fs.readFileSync(alias),originalBytes);
  } finally {
    fs.readSync = originalRead; syncBuiltinESMExports(); fs.rmSync(f.root,{recursive:true,force:true});
  }
});
