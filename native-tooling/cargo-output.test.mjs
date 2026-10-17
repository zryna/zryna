import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,mkdir,writeFile,link,symlink,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {performance} from 'node:perf_hooks';
import {observeCargoOutput,observeAuthenticatedPin} from './binding-producer/recorder/capture.mjs';
import {descriptor} from './owner/files.mjs';

test('only fixed Cargo output observations admit stable hardlinks; all original material guards stay strict', async()=>{
  const root=await mkdtemp(join(tmpdir(),'m6410-build-pins-'));
  try {
    const debug=join(root,'debug');await mkdir(debug);await mkdir(join(debug,'deps'));
    const path=join(debug,'zryna'),alias=join(debug,'deps','zryna-test');
    await writeFile(path,'actual stable build output\n');await link(path,alias);
    const deadline=performance.now()+10000;
    const observed=await observeCargoOutput(path,debug,deadline);
    assert.equal(observed.identity.links,'2');
    assert.equal((await observeCargoOutput(alias,debug,deadline)).sha256,observed.sha256);
    await assert.rejects(descriptor(path),/Single-link/);
    const pin={path,bytes:observed.bytes,sha256:observed.sha256};
    await assert.rejects(observeAuthenticatedPin(pin,deadline));
    await assert.rejects(observeCargoOutput(path,root,deadline),/Fixed Cargo/);
    await symlink(path,join(debug,'linked'));
    await assert.rejects(observeCargoOutput(join(debug,'linked'),debug,deadline));
    await assert.rejects(observeCargoOutput(path,debug,performance.now()-1),/deadline/);
  } finally {await rm(root,{recursive:true});}
});
