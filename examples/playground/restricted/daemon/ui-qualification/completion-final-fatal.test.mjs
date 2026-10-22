// Owned temporary copies with two exact dependency substitutions: control-flow fixture only.
import test from 'node:test';import assert from 'node:assert/strict';
import { mkdtemp, readFile, writeFile, rm } from 'node:fs/promises';import { tmpdir } from 'node:os';import { join } from 'node:path';import { pathToFileURL } from 'node:url';
import { EventEmitter } from 'node:events';import { tagTransport } from './wire.mjs';
async function fixture({ late = false, early = false, snapshotThrows = false } = {}) {
  const directory=await mkdtemp(join(tmpdir(),'zryna-completion-final-'));
  try {
    const source=await readFile(new URL('./browser-probe.mjs',import.meta.url),'utf8');
    let copy=source;
    for(const name of ['wire','browser-diagnostics','completion-record']) copy=copy.replaceAll(`from './${name}.mjs'`,`from ${JSON.stringify(new URL('./'+name+'.mjs',import.meta.url).href)}`);
    for(const name of ['browser-intercept','browser-corpus']) {
      const from=`from './${name}.mjs'`;assert.equal(copy.split(from).length-1,1);copy=copy.replace(from,`from ${JSON.stringify('./'+name+'.mjs')}`);
    }
    await writeFile(join(directory,'browser-probe.mjs'),copy,{flag:'wx'});
    await writeFile(join(directory,'state.mjs'),'export const state={late:null,events:[],snapshotThrows:false};',{flag:'wx'});
    await writeFile(join(directory,'browser-intercept.mjs'),`import {state} from './state.mjs';
      export async function interceptBrowser(){const failures=new Promise(()=>{});return {
        failures,beginPublication(){return Object.freeze({});},publish(){},proveInitialAbort(){},
        probeState(){return{id:1};},abortProof(){return[{id:1,paused:true,acknowledged:true,event:'requestfailed',httpResponseClosed:true}];},
        async beginCorpus(){},seal(){return{sealed:true};},accept(){},invalidate(){},
        failureBranch(){return'UNSPECIFIED';},assertHealthy(){if(state.late)throw state.late;},
        completionFailure(){if(state.snapshotThrows)throw Error('fixture snapshot');return null;},
        async stop(){state.events.push('intercept');}};}`,{flag:'wx'});
    await writeFile(join(directory,'browser-corpus.mjs'),"export async function runBrowserCorpus(){return{fixture:true};}",{flag:'wx'});
    const {state}=await import(pathToFileURL(join(directory,'state.mjs')));state.snapshotThrows=snapshotThrows;
    const {runInitialBrowserProbe,browserFailure}=await import(pathToFileURL(join(directory,'browser-probe.mjs')));
    const browser=new EventEmitter(),context=new EventEmitter(),page=new EventEmitter(),cdp=new EventEmitter();let evaluated=0;
    const sourceControl={async waitFor(){},async focus(){},async fill(){},async press(){}};
    Object.assign(browser,{async newContext(){return context;},async close(){state.events.push('browser');}});
    Object.assign(context,{async newPage(){return page;},async newCDPSession(){return cdp;},async close(){
      state.events.push('context');if(late)state.late=tagTransport(new Error('PLAYGROUND-UI-TRANSPORT'),'INTERCEPT_DROP');}});
    Object.assign(page,{setDefaultTimeout(){},async goto(){},async waitForFunction(){},
      getByRole(role){if(role==='textbox')return sourceControl;return{async click(){},filter(){return{async waitFor(){}};}};},
      async evaluate(){return evaluated++===0?{hashRemoved:true,pathname:'/examples/playground/restricted/',title:early?'wrong':'Zryna source playground',role:'status',live:'polite'}:
        {compileDisabled:false,runDisabled:true,diagnosticChildren:0};}});
    cdp.send=async()=>({nodes:[]});let observation,primary;
    try {
      const result=await runInitialBrowserProbe({chromium:{async launch(){return browser;}},configuration:{version:1,type:'start',runId:'a'.repeat(32),
        origin:'http://127.0.0.1:12345',launchPath:'/examples/playground/restricted/',capability:'b'.repeat(64),planSha256:'c'.repeat(64)},
        send(){},register(){},onFailure(error){primary=error;},onCleanup(value){observation=value;}});
      return{result,events:state.events,observation,primary};
    } catch(error){return{error,detail:browserFailure(error),events:state.events,observation,primary};}
  } finally {await rm(directory,{recursive:true,force:true});}
}
test('tentative success is defeated by latefatal duringcleanup withoutfalsecleanupfailure',async()=>{
  const value=await fixture({late:true});assert.equal(value.result,undefined);assert.equal(value.error,value.primary);
  assert.equal(value.detail.branch,'INTERCEPT_DROP');assert.equal(value.detail.cleanupFailed,false);
  assert.deepEqual(value.events,['intercept','context','browser']);assert.deepEqual(value.observation.cleanup,{intercept:false,context:false,browser:false});
});
test('an earlier browserprimary survives latefatal and every originalcleanup stillruns',async()=>{
  const value=await fixture({late:true,early:true});assert.equal(value.detail.cause,'INITIAL_TITLE');assert.equal(value.detail.branch,'UNSPECIFIED');
  assert.deepEqual(value.events,['intercept','context','browser']);assert.equal(value.detail.cleanupFailed,false);
});
test('completion observation failure cannot skip cleanup or replace tentative success',async()=>{
  const value=await fixture({snapshotThrows:true});assert.equal(value.error,undefined);assert.equal(value.result.browserClosed,true);
  assert.deepEqual(value.events,['intercept','context','browser']);assert.equal(value.observation.completionFailure,null);
});
