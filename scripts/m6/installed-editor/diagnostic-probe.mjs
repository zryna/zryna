// Runs ONLY as original nonroot runner inside the exact bounded transient service.
import assert from 'node:assert/strict';
import {readFile,writeFile,lstat} from 'node:fs/promises';
import {join,resolve,dirname} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createRequire} from 'node:module';
import {setTimeout as delay} from 'node:timers/promises';
import {createHash} from 'node:crypto';
import {chromium} from 'playwright-core';
import {command,startEditor,remaining} from './process.mjs';
import {debuggerEndpoint} from './trust.mjs';
const require=createRequire(import.meta.url),{editorCli}=require('../../portable-setup/setup.cjs');
const root=process.argv[2];assert(process.argv.length===3&&root===resolve(root));
const c=JSON.parse(await readFile(join(root,'CONFIG.json'),'utf8'));
assert(c.format==='private.editor-scoped-userns-diagnostic.v1'&&c.root===root&&process.getuid()===c.uid&&process.getgid()===c.gid&&c.uid>0);
const out=join(c.stage,'probe'),deadline=performance.now()+50000;
const write=(name,value)=>writeFile(join(root,name),JSON.stringify(value),{flag:'wx',mode:0o400});
let owner,browser,phase='ROOT_RELEASE_GATE',primary=null,ownerCleanup=null,ready=false;
let quitRequested=false,quitAcknowledged=false,naturalExit=null,browserDisconnected=false;
async function bounded(action,ms,label){
 const timeout=Math.min(ms,remaining(deadline));let timer;
 try{const expired=new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error(label)),timeout);});return await Promise.race([Promise.resolve().then(action),expired]);}
 finally{clearTimeout(timer);}
}
await write('PROBE_STARTED.json',{pid:process.pid,uid:process.getuid(),gid:process.getgid(),diagnosticOnly:true});
try{
 for(;;){remaining(deadline);try{const s=await lstat(join(root,'RELEASE_START'));assert(s.isFile()&&!s.isSymbolicLink()&&s.uid===0&&s.size===0);break;}catch(e){if(e.code!=='ENOENT')throw e;}await delay(25);}
 phase='PACKAGED_CLI';const cli=editorCli(c.code.path),cliOut=await command(c.code.path,[cli,'--version'],{cwd:c.stage,stage:c.stage,env:{...process.env,ELECTRON_RUN_AS_NODE:'1'},output:out,name:'packaged-js-cli',deadline});
 assert(cliOut.split(/\r?\n/)[0]==='1.138.0');
 phase='ORIGINAL_GUI_OWNER';const env={...process.env};delete env.ELECTRON_RUN_AS_NODE;
 owner=await startEditor(c.code.path,['--new-window','--skip-welcome','--skip-release-notes','--disable-updates','--locale=en-US',
  '--remote-debugging-address=127.0.0.1','--remote-debugging-port=0',`--user-data-dir=${join(c.stage,'profile')}`,`--extensions-dir=${join(c.stage,'extensions')}`],
 {cwd:c.stage,stage:c.stage,env,output:out,deadline});
 phase='DEVTOOLS_ACTIVE_PORT';let endpoint;
 for(;;){remaining(deadline);if(owner.failure())throw owner.failure();if(owner.ended())throw Error('EDITOR_EARLY_EXIT');
  try{const p=join(c.stage,'profile/DevToolsActivePort'),s=await lstat(p);assert(s.isFile()&&!s.isSymbolicLink()&&s.size<=4096);endpoint=debuggerEndpoint(await readFile(p,'utf8'));break;}catch(e){if(e.code!=='ENOENT')throw e;}await delay(25);}
 phase='ACTUAL_CODE_PAGE';browser=await chromium.connectOverCDP(endpoint,{timeout:remaining(deadline),noDefaults:true});
 const expected=pathToFileURL(join(dirname(c.code.path),'resources/app/out/vs/code/electron-browser/workbench/workbench.html')).pathname;
 let selected;
 for(;;){remaining(deadline);if(owner.failure())throw owner.failure();if(owner.ended())throw Error('EDITOR_EARLY_EXIT');
  const pages=browser.contexts().flatMap(x=>x.pages()).filter(p=>{try{const u=new URL(p.url());return u.protocol==='vscode-file:'&&u.hostname==='vscode-app'&&u.pathname===expected;}catch{return false;}});
  assert(pages.length<=1);if(pages.length===1&&await pages[0].evaluate(()=>document.readyState==='complete')){selected=pages[0];break;}await delay(25);}
 assert(selected);ready=true;phase='GUI_READY_HOLD';await write('GUI_READY.json',{diagnosticOnly:true,pid:owner.pid,actualCodePage:true,devtoolsActivePort:true,atMs:Math.floor(performance.now())});
 const until=performance.now()+3000;while(performance.now()<until){if(owner.failure())throw owner.failure();if(owner.ended())throw Error('EDITOR_EXIT_DURING_HOLD');await delay(50);}
 phase='WORKBENCH_QUIT_REQUEST';quitRequested=true;
 await bounded(()=>selected.keyboard.press('Control+q',{timeout:Math.min(2000,remaining(deadline))}),2000,'WORKBENCH_QUIT_ACK_TIMEOUT');quitAcknowledged=true;
 phase='NATURAL_EDITOR_EXIT';const quitUntil=Math.min(deadline,performance.now()+5000);
 while(!owner.ended()){if(owner.failure())throw owner.failure();assert(performance.now()<quitUntil,'NATURAL_EDITOR_EXIT_TIMEOUT');await delay(25);}
 naturalExit=owner.ended();assert.equal(naturalExit.status,0,'Natural editor exit must be zero.');
}catch(e){primary={phase,type:e.name,code:typeof e.code==='string'?e.code:null,message:String(e.message).slice(0,2048),stack:String(e.stack??e.message).slice(0,8192),messageSha256:createHash('sha256').update(String(e.message)).digest('hex')};}
finally{
 if(owner){try{await owner.finish();ownerCleanup=true;}catch(e){ownerCleanup=false;primary??={phase:'ORIGINAL_OWNER_FINISH',type:e.name,code:typeof e.code==='string'?e.code:null,message:String(e.message).slice(0,2048),stack:String(e.stack??e.message).slice(0,8192)};}}
 if(browser){try{await bounded(()=>browser.close(),1000,'CDP_DISCONNECT_TIMEOUT');browserDisconnected=true;}catch(e){primary??={phase:'CDP_DISCONNECT',type:e.name,code:typeof e.code==='string'?e.code:null,message:String(e.message).slice(0,2048),stack:String(e.stack??e.message).slice(0,8192)};}}
 await write('PROBE_RESULT.json',{diagnosticOnly:true,qualification:false,guiReady:ready,quitRequested,quitAcknowledged,naturalExit,browserDisconnected,originalOwnerCleanup:ownerCleanup,primary,
  scope:'Keyboard acknowledgement is not natural zero exit or original owner cleanup; outer systemd cleanup is separate diagnostic rescue.'});
 if(primary||ownerCleanup!==true||!ready||!quitAcknowledged||naturalExit?.status!==0||!browserDisconnected)process.exitCode=1;
}
