import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, writeFile, realpath, rename } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { performance } from 'node:perf_hooks';
import { chromium } from 'playwright-core';
import { launchOwned } from './owner/owner-client.mjs';
import { command } from './owner/process.mjs';
import { acquire } from '../scripts/m6/installed-editor-acquire.mjs';
import { verifyEditor } from '../scripts/m6/installed-editor/materials.mjs';
import { debuggerEndpoint } from '../scripts/m6/installed-editor/trust.mjs';
const assertContext = () => {
  assert.equal(process.platform, 'win32'); assert.equal(process.arch, 'x64');
  assert.equal(process.version, 'v22.22.1');
  assert.equal(process.env.GITHUB_REPOSITORY, 'zryna/zryna');
  assert.equal(process.env.GITHUB_REF, 'refs/heads/m6-job-owner-code2-20261017');
};
async function context(name) {
  assertContext(); const base = await realpath(process.env.RUNNER_TEMP);
  const stage = await mkdtemp(join(base, 'm6-own-diag-'));
  const output = join(base, 'm6-owner-diagnostic'); await mkdir(output);
  await writeFile(join(output, 'SCOPE.json'), JSON.stringify({ diagnosticOnly: true,
    operator: process.env.GITHUB_SHA, product: 'd3c2ade51730bc9998a6b65e196facecfebfa455',
    kind: name, stage, installedVsix: false, finalAcceptance: false }, null, 2));
  return { stage, output, cwd: stage, env: { ...process.env }, deadline: performance.now() + 600000 };
}
export async function natural() {
  const c = await context('natural');
  const o = await launchOwned(process.execPath, ['-e', "require('node:fs').writeSync(1,'root-natural\\n')"], { ...c, name: 'natural' });
  await o.finish();
}
export async function fanout() {
  const c = await context('fanout');
  const source = `const {spawn}=require('node:child_process'); const path=require('node:path');
    for(let n=0;n<12;n++){const p=spawn(path.join(process.env.SystemRoot,'System32','whoami.exe'),[],
      {detached:true,stdio:'ignore',windowsHide:false});p.unref();}
    setTimeout(()=>{},2000);`;
  const o = await launchOwned(process.execPath, ['-e', source], { ...c, name: 'fanout' });
  await o.finish();
}
export async function code() {
  const c = await context('code'); const acquired = await acquire(join(c.stage, 'material'));
  const archive = join(c.stage, 'authenticated-editor.archive'); await rename(acquired, archive);
  const root = join(c.stage, 'editor'); await mkdir(root);
  const runtime = join(process.env.ProgramFiles, 'PowerShell', '7', 'pwsh.exe');
  await command(runtime, ['-NoLogo','-NoProfile','-NonInteractive','-File',
    resolve('scripts/m6/installed-editor/extract-editor-windows.ps1'),archive,root],
    { ...c, name:'extract-code' });
  const editor = await verifyEditor(root, c.deadline);
  const profile = join(c.stage,'profile'), extensions = join(c.stage,'extensions'), folder = join(c.stage,'exact-diagnostic-folder');
  for(const p of [profile,extensions,folder]) await mkdir(p);
  await mkdir(join(profile,'User'));
  await writeFile(join(profile,'User','settings.json'), JSON.stringify({'security.workspace.trust.startupPrompt':'always'}), {flag:'wx'});
  const env={...c.env};delete env.ELECTRON_RUN_AS_NODE;
  const owner = await launchOwned(editor.executable, ['--new-window','--skip-welcome','--skip-release-notes',
    '--disable-updates','--locale=en-US','--remote-debugging-address=127.0.0.1','--remote-debugging-port=0',
    '--user-data-dir='+profile,'--extensions-dir='+extensions,folder],{...c,env,name:'code'});
  let browser, primary=null, cleanup=null, snapshots=[];
  try {
    const end=Math.min(c.deadline,performance.now()+30000);let endpoint;
    while(!endpoint){assert(performance.now()<end,'Debugger readiness deadline');if(owner.failure())throw owner.failure();
      try{endpoint=debuggerEndpoint(await readFile(join(profile,'DevToolsActivePort'),'utf8'));}
      catch(e){if(e.code!=='ENOENT')throw e;}
      if(!endpoint)await new Promise(r=>setTimeout(r,50));}
    browser=await chromium.connectOverCDP(endpoint,{timeout:Math.max(1,Math.floor(end-performance.now())),noDefaults:true});
    for(let n=0;n<20;n++){
      assert(performance.now()<end);if(owner.failure())throw owner.failure();
      const pages=browser.contexts().flatMap(x=>x.pages());assert(pages.length<=16);
      const snapshot=[];
      for(const p of pages)snapshot.push({url:p.url().slice(0,2048),dom:await p.evaluate(()=>({title:document.title.slice(0,256),
        body:(document.body?.innerText??'').slice(0,16384),dialogs:[...document.querySelectorAll('[role="dialog"],[role="alertdialog"]')]
          .slice(0,8).map(x=>({role:x.getAttribute('role'),text:(x.textContent??'').slice(0,8192)}))}))});
      snapshots.push(snapshot);await writeFile(join(c.output,'GUI-'+n+'.json'),JSON.stringify(snapshot,null,2),{flag:'wx'});
      await new Promise(r=>setTimeout(r,500));}
  }catch(e){primary=String(e.stack??e);}
  finally{if(browser)await browser.close();try{await owner.finish({stop:true});}catch(e){cleanup=String(e.stack??e);}}
  const record=JSON.parse(await readFile(join(c.output,'code.json'),'utf8'));
  const confirmed=record.complete?.cleanupConfirmed===true && record.complete.failures.length===0;
  await writeFile(join(c.output,'DIAGNOSTIC_RESULT.json'),JSON.stringify({primary,cleanup,confirmed,
    observedSnapshots:snapshots.length,forcedStop:true,actualTargetStatus:record.complete?.status??null,
    productAcceptance:false,installedVsix:false},null,2),{flag:'wx'});
  assert(!primary && confirmed && snapshots.some(x=>x.some(p=>p.dom.body.length>0)),'Diagnostic incomplete; no product acceptance claimed');
}
