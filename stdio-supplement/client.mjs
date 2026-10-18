// Prospective private fixed-input operator. Parent must independently authenticate
// source, complete setup archive/manifest and current exact-main hosted provenance.
// Run under the unchanged native process owner; this is no hostile containment claim.
import assert from 'node:assert/strict';
import {spawn,spawnSync} from 'node:child_process';
import {mkdirSync,readFileSync,writeFileSync,openSync,closeSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {join,resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
const argv=process.argv.slice(2);
assert.equal(argv.length,10);
assert.deepEqual(argv.filter((_,i)=>i%2===0),['--source','--installation','--manifest-sha256','--source-commit','--output']);
const [source,installation,digest,commit,output]=argv.filter((_,i)=>i%2===1);
assert([source,installation,output].every(p=>resolve(p)===p));assert.match(digest,/^[a-f0-9]{64}$/);assert.match(commit,/^[a-f0-9]{40}$/);
assert.equal(process.version,'v22.22.1');assert.equal(process.arch,'x64');assert(['linux','win32'].includes(process.platform));
mkdirSync(output,{mode:0o700});const resultFd=openSync(join(output,'RESULT.json'),'wx',0o400);
const put=(name,data)=>writeFileSync(join(output,name),data,{flag:'wx',mode:0o400});
const json=(name,value)=>put(name,JSON.stringify(value,null,2)+'\n');
const sha=b=>createHash('sha256').update(b).digest('hex');
const require=createRequire(pathToFileURL(join(source,'package.json')));
const {verifyInstallation}=require(join(source,'editors/vscode-zryna/src/installation.cjs'));
let child,closed=false,stdout=[],stderr=[],sent=[],outBytes=0,errBytes=0,rows=[],failure=null;
let resolveClose;const closePromise=new Promise(r=>resolveClose=r);
let outcome={status:'failed',scope:'Fixed installed CLI override and LSP stdio supplement; parent native ownership proof required.'};
const work=join(output,'practice');mkdirSync(work,{mode:0o700});mkdirSync(join(output,'empty-path'));mkdirSync(join(output,'temporary'));
const env={PATH:join(output,'empty-path'),HOME:work,USERPROFILE:work,TMPDIR:join(output,'temporary'),TEMP:join(output,'temporary'),TMP:join(output,'temporary'),LANG:'C.UTF-8',LC_ALL:'C.UTF-8',TZ:'UTC',...(process.platform==='win32'?{SystemRoot:process.env.SystemRoot,WINDIR:process.env.SystemRoot,PATHEXT:'.EXE',COMSPEC:join(process.env.SystemRoot,'System32/cmd.exe')}:{})};
let timer;const deadline=new Promise((_,reject)=>{timer=setTimeout(()=>reject(new Error('FIXED_SUPPLEMENT_45000MS_DEADLINE')),45000)});deadline.catch(()=>{});
try{
 const config=verifyInstallation(installation,digest);assert.equal(config.manifest.sourceCommit,commit);
 assert.equal(config.manifest.compilerVersion,'0.2.3');assert.equal(config.manifest.serverVersion,'0.5.0');
 json('APPROVED_INVOCATION.json',{source,installation,manifestSha256:digest,sourceCommit:commit,compiler:config.compilerPath,server:config.serverPath,environment:env,lifetimeMs:45000,serverStdoutLimit:65536,serverStderrLimit:16384,scope:'Literal fixed known sources and invalid flags only; no arbitrary selector or extra method.'});
 const sourceText='// é😀e\u0301\r\nexport function identity(x: i32): i32 { return x; }\r\n';
 const sourcePath=join(work,'main.zry');writeFileSync(sourcePath,sourceText,{flag:'wx',mode:0o600});
 for(const [name,flag,value] of [['root','--root',join(output,'absent-root')],['node','--node',join(output,'absent-node')]]){
  const args=['build','main.zry','--project-root',work,'--target','javascript',flag,value,'--json'];
  const actual=spawnSync(config.compilerPath,args,{cwd:work,env,shell:false,windowsHide:true,timeout:10000,maxBuffer:65536,killSignal:'SIGKILL'});
  const out=Buffer.from(actual.stdout??[]),err=Buffer.from(actual.stderr??[]);put(name+'-override.stdout',out);put(name+'-override.stderr',err);
  json(name+'-override-command.json',{executable:config.compilerPath,argv:args,cwd:work,environment:env,status:actual.status,signal:actual.signal,errorCode:actual.error?.code??null,stdoutBytes:out.length,stdoutSha256:sha(out),stderrBytes:err.length,stderrSha256:sha(err)});
  assert.equal(actual.status,2);assert.equal(actual.signal,null);assert.equal(actual.error,undefined);assert.equal(err.length,0);
  const v=JSON.parse(out);assert.equal(v.command,'build');assert.equal(v.ok,false);assert(v.diagnostics.some(d=>d.code==='ZRYNA-C2001'&&d.message==='installed commands do not accept compiler-root or Node.js overrides'));
  rows.push({id:'installed-forbidden-'+name,status:'pass',exitCode:2});
 }
 let buffer=Buffer.alloc(0),terminal=null;const waiting=new Map(),notifications=[];
 const rejectAll=e=>{terminal??=e;for(const {reject}of waiting.values())reject(e);waiting.clear()};
 const receive=chunk=>{
  outBytes+=chunk.length;if(outBytes>65536){rejectAll(new Error('SERVER_STDOUT_LIMIT'));return}
  stdout.push(Buffer.from(chunk));buffer=Buffer.concat([buffer,chunk]);
  for(;;){const at=buffer.indexOf('\r\n\r\n');if(at<0){if(buffer.length>256)rejectAll(new Error('HEADER_LIMIT'));return}
   const h=buffer.subarray(0,at).toString('ascii');assert.match(h,/^Content-Length: [1-9][0-9]{0,4}$/i);const n=Number(h.split(':')[1]);assert(n<=65536);
   if(buffer.length<at+4+n)return;const raw=buffer.subarray(at+4,at+4+n);buffer=buffer.subarray(at+4+n);const v=JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(raw));
   if(Object.hasOwn(v,'id')){const pending=waiting.get(v.id);assert(pending,'unexpected/duplicate response id');waiting.delete(v.id);pending.resolve(v)}else{assert.equal(typeof v.method,'string');notifications.push(v)}
  }
 };
 child=spawn(config.serverPath,['--installed-root',config.compilerRoot],{cwd:work,env,shell:false,windowsHide:true,stdio:['pipe','pipe','pipe']});
 child.stdout.on('data',b=>{try{receive(b)}catch(e){rejectAll(e)}});
 child.stderr.on('data',b=>{errBytes+=b.length;if(errBytes>16384)rejectAll(new Error('SERVER_STDERR_LIMIT'));else stderr.push(Buffer.from(b))});
 for(const stream of [child.stdin,child.stdout,child.stderr])stream.on('error',rejectAll);
 child.on('error',rejectAll);child.on('close',(status,signal)=>{closed=true;resolveClose({status,signal});if(waiting.size)rejectAll(new Error('SERVER_EARLY_CLOSE'))});
 json('SERVER_COMMAND.json',{executable:config.serverPath,argv:['--installed-root',config.compilerRoot],cwd:work,environment:env,pid:child.pid});
 const send=v=>{assert(!terminal,terminal?.message);const raw=Buffer.from(JSON.stringify(v));const framed=Buffer.concat([Buffer.from('Content-Length: '+raw.length+'\r\n\r\n'),raw]);sent.push(framed);child.stdin.write(framed)};
 const request=(id,method,params)=>Promise.race([new Promise((resolve,reject)=>{assert(!waiting.has(id));waiting.set(id,{resolve,reject});send({jsonrpc:'2.0',id,method,...(params===undefined?{}:{params})})}),deadline]);
 const init=await request(1,'initialize',{rootUri:pathToFileURL(work).href,capabilities:{general:{positionEncodings:['utf-16']}}});
 assert.equal(init.result.serverInfo.version,'0.5.0');assert.equal(init.result.capabilities.positionEncoding,'utf-16');assert.equal(init.result.capabilities.experimental.zrynaSourceCommit,commit);
 send({jsonrpc:'2.0',method:'initialized',params:{}});const uri=pathToFileURL(sourcePath).href;
 send({jsonrpc:'2.0',method:'textDocument/didOpen',params:{textDocument:{uri,languageId:'zryna',version:1,text:sourceText}}});
 const split=await request(2,'textDocument/definition',{textDocument:{uri},position:{line:0,character:sourceText.indexOf('😀')+1}});assert.equal(split.error.code,-32602);
 const line=sourceText.split('\r\n')[1],reference=line.lastIndexOf('x;'),declaration=line.indexOf('x:');
 const expected={uri,range:{start:{line:1,character:declaration},end:{line:1,character:declaration+1}}};
 const valid=await request(3,'textDocument/definition',{textDocument:{uri},position:{line:1,character:reference}});assert.deepEqual(valid.result,expected);
 const unsupported=await request(4,'textDocument/rename',{textDocument:{uri},position:{line:1,character:reference},newName:'renamed'});assert.equal(unsupported.error.code,-32601);
 const recovered=await request(5,'textDocument/definition',{textDocument:{uri},position:{line:1,character:reference}});assert.deepEqual(recovered.result,expected);
 const shutdown=await request(6,'shutdown');assert.equal(shutdown.result,null);send({jsonrpc:'2.0',method:'exit'});child.stdin.end();
 const exit=await Promise.race([closePromise,deadline]);assert.equal(exit.status,0);assert.equal(exit.signal,null);assert.equal(buffer.length,0);assert.equal(waiting.size,0);assert.equal(errBytes,0);assert.equal(terminal,null);
 assert(notifications.some(v=>v.method==='textDocument/publishDiagnostics'&&v.params.version===1&&v.params.diagnostics.length===0));
 assert.equal(readFileSync(sourcePath,'utf8'),sourceText);verifyInstallation(installation,digest);
 rows.push({id:'installed-unicode-crlf-astral-split',status:'pass',errorCode:-32602},{id:'installed-scalar-definition-recovery',status:'pass',definition:expected},{id:'installed-unsupported-method',status:'pass',errorCode:-32601},{id:'installed-recovery-after-unsupported',status:'pass',definition:expected});
 outcome={status:'captured-fixed-supplement-pass',sourceCommit:commit,manifestSha256:digest,rows,serverExit:exit,sourceSha256:sha(Buffer.from(sourceText)),sourcePreserved:true,fullInstallationReverified:true,serverStdoutBytes:outBytes,serverStdoutSha256:sha(Buffer.concat(stdout)),serverStderrBytes:errBytes,serverStderrSha256:sha(Buffer.concat(stderr)),requestBytes:Buffer.concat(sent).length,requestSha256:sha(Buffer.concat(sent)),scope:'Actual installed fixed CLI/LSP captures; separate unchanged parent native owner completion/cleanup required before acceptance.'};
}catch(error){failure=error;outcome.failure=String(error.stack??error).slice(0,5000);throw error}
finally{
 clearTimeout(timer);if(child&&!closed){child.stdin.destroy();child.kill('SIGKILL');await Promise.race([closePromise,new Promise(r=>setTimeout(r,2000))])}
 put('server.stdout',Buffer.concat(stdout));put('server.stderr',Buffer.concat(stderr));put('client.stdin',Buffer.concat(sent));writeFileSync(resultFd,JSON.stringify(outcome,null,2)+'\n');closeSync(resultFd);
}
console.log(JSON.stringify(outcome));
