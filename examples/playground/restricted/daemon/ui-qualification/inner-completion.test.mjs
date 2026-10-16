// Driver controlflow fixture only: import substitution never executes or qualifies real Chrome.
import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { PassThrough } from 'node:stream';
import { createFrameWriter, createFrameDecoder } from './framing.mjs';
import { sha256, emptyBrowserProcess, transportBranch } from './wire.mjs';
import { planBytes } from './plan.mjs';

test('stream failure during successful work cleanup retains finalized observations without success upgrade',async()=>{
  const root=mkdtempSync(join(tmpdir(),'zryna-ui-completion-fixture-'));
  const fixture=pathToFileURL(join(root,'probe.mjs')).href;
  const completed={cleanupFailed:false,cleanup:{intercept:false,context:false,browser:false},
    lifecycle:{browserDisconnected:false,pageClosed:false,pageCrashed:false,contextClosed:false},
    process:{...emptyBrowserProcess(),attached:true,identity:'STABLE',stderrSeen:true,stderrUnknown:true}};
  try {
    // Only the probe dependency is stubbed. The real driver, framing, validators and plan are used.
    const original=readFileSync(new URL('./inner.mjs',import.meta.url),'utf8');
    const probeLine="import { runInitialBrowserProbe, browserFailure } from './browser-probe.mjs';";
    assert.equal(original.split(probeLine).length,2);
    let copied=original.replace(probeLine,`import { runInitialBrowserProbe, browserFailure } from ${JSON.stringify(fixture)};`);
    for(const name of ['framing','wire','plan']){
      const selected=`'./${name}.mjs'`;assert.equal(copied.split(selected).length,2);
      copied=copied.replace(selected,JSON.stringify(new URL(`./${name}.mjs`,import.meta.url).href));
    }
    writeFileSync(join(root,'inner.mjs'),copied,{flag:'wx',mode:0o400});
    writeFileSync(join(root,'probe.mjs'),`
      export { browserFailure } from ${JSON.stringify(new URL('./browser-probe.mjs',import.meta.url).href)};
      let entered, release;export const cleaning=new Promise(resolve=>entered=resolve);
      const gate=new Promise(resolve=>release=resolve);export const finish=()=>release();
      export async function runInitialBrowserProbe({checkpoint,onCleanup}){
        checkpoint('RESULT');entered();await gate;onCleanup(${JSON.stringify(completed)});
        return {fixtureOnly:true};
      }
    `,{flag:'wx',mode:0o400});
    const {runUiDriver}=await import(pathToFileURL(join(root,'inner.mjs')).href);
    const selected=await import(fixture),input=new PassThrough(),output=new PassThrough(),records=[];
    const decoder=createFrameDecoder(record=>records.push(record));output.on('data',chunk=>decoder.push(chunk));
    const run=runUiDriver(input,output,async()=>({fixtureOnly:true}),sha256(planBytes()));run.catch(()=>{});
    const writer=createFrameWriter(input);
    await writer.write({version:1,type:'start',runId:'a'.repeat(32),origin:'http://127.0.0.1:12345',
      launchPath:'/examples/playground/restricted/',capability:'b'.repeat(64),planSha256:sha256(planBytes())});
    await selected.cleaning;input.write(Buffer.from([255,255,255,127]));
    await new Promise(resolve=>setImmediate(resolve));selected.finish();
    await assert.rejects(run,error=>transportBranch(error)==='FRAMING_002');decoder.end();
    assert.equal(records.length,1);assert.equal(records[0].type,'failure');assert.equal(records[0].phase,'RESULT');
    assert.equal(records[0].cause,'TRANSPORT');assert.equal(records[0].branch,'FRAMING_002');
    for(const key of ['cleanupFailed','cleanup','lifecycle','process'])assert.deepEqual(records[0][key],completed[key]);
    await writer.stop();input.destroy();output.destroy();
  }finally{rmSync(root,{recursive:true,force:false});}
});
