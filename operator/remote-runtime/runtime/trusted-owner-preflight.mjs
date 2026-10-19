// Fixed host-native ownership smoke only. No guest, browser, compiler or source evaluation.
import assert from 'node:assert/strict';
import {readFileSync,lstatSync,writeFileSync,mkdirSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {performance} from 'node:perf_hooks';
import {pathToFileURL} from 'node:url';
const [repo,client,work,expectedRaw]=process.argv.slice(2),python='/usr/bin/python3.12';
assert.equal(process.argv.length,6);assert.equal(repo,'/workspace/zryna');assert.equal(process.version,'v22.22.1');
assert.match(work,/^\/workspace\/scratch\/M6_410_EPHEMERAL_HOST_[1-9][0-9]*_[1-9][0-9]*\/preparation-work\/trusted-owner-smoke$/);
const host=work.slice(0,work.indexOf('/preparation-work/'));
assert.equal(client,host+'/candidate/caller-view/runtime-operators/trusted-owner/trusted-ui-owner-client.mjs');
const sha=b=>createHash('sha256').update(b).digest('hex');
const raw=readFileSync('/proc/self/status','utf8'),status=Object.fromEntries(raw.trim().split('\n').map(l=>{const i=l.indexOf(':');return[l.slice(0,i),l.slice(i+1).trim()];}));
assert.deepEqual(status.Uid.split(/\s+/),['1000','1000','1000','1000']);assert.deepEqual(status.Gid.split(/\s+/),['1000','1000','1000','1000']);
assert.equal(status.Groups,'');assert.equal(status.NoNewPrivs,'1');for(const k of ['CapInh','CapPrm','CapEff','CapBnd','CapAmb'])assert.equal(BigInt('0x'+status[k]),0n);
const save=(n,v)=>writeFileSync(work+'/'+n,Buffer.isBuffer(v)?v:JSON.stringify(v,null,2)+'\n',{flag:'wx',mode:0o400});
save('CALLER_IDENTITY.json',{pid:process.pid,parentPid:process.ppid,uid:process.getuid(),euid:process.geteuid(),gid:process.getgid(),egid:process.getegid(),kernelGroups:[],nodeGetgroups:process.getgroups(),rawStatus:raw,status,statusSha256:sha(Buffer.from(raw))});
const expected=JSON.parse(expectedRaw),st=lstatSync(python),bytes=readFileSync(python);
assert(st.isFile()&&!st.isSymbolicLink()&&st.uid===0&&st.nlink===1&&!(st.mode&0o022));assert.deepEqual({path:python,bytes:bytes.length,sha256:sha(bytes)},expected);
const nativeCode=String.raw`import ctypes,json,os,signal,sys
assert sys.executable=='/usr/bin/python3.12' and sys.version_info[:2]==(3,12)
assert os.getuid()==os.geteuid()==os.getgid()==os.getegid()==1000 and os.getgroups()==[]
fd=os.pidfd_open(os.getpid(),0)
try:signal.pidfd_send_signal(fd,0,None,0)
finally:os.close(fd)
libc=ctypes.CDLL(None,use_errno=True)
assert libc.prctl(36,1,0,0,0)==0
state=ctypes.c_int();assert libc.prctl(37,ctypes.byref(state),0,0,0)==0 and state.value==1
print(json.dumps({'runtime':sys.executable,'version':sys.version,'versionInfo':list(sys.version_info),'prefix':sys.prefix,'pid':os.getpid(),'pidfdOpenSelf':True,'pidfdSignalZero':True,'nativeChildSubreaper':state.value}))`;
const probe=spawnSync(python,['-I','-B','-c',nativeCode],{cwd:repo,env:process.env,encoding:'buffer',timeout:10000,maxBuffer:1048576});
save('NATIVE_FEATURES.stdout',probe.stdout??Buffer.alloc(0));save('NATIVE_FEATURES.stderr',probe.stderr??Buffer.alloc(0));save('NATIVE_FEATURES.command.json',{argv:[python,'-I','-B','-c',nativeCode],status:probe.status,signal:probe.signal,error:probe.error?.message??null});
assert.equal(probe.error,undefined);assert.equal(probe.status,0);assert.equal(probe.signal,null);assert.equal(probe.stderr.length,0);const native=JSON.parse(probe.stdout);
const packageArgs=['-W','-f=${binary:Package}\t${Version}\n','python3.12-minimal','libpython3.12-stdlib'];
const pkg=spawnSync('/usr/bin/dpkg-query',packageArgs,{cwd:repo,env:process.env,encoding:'buffer',timeout:10000,maxBuffer:1048576});
save('PACKAGE_VERSION.stdout',pkg.stdout??Buffer.alloc(0));save('PACKAGE_VERSION.stderr',pkg.stderr??Buffer.alloc(0));save('PACKAGE_VERSION.command.json',{argv:['/usr/bin/dpkg-query',...packageArgs],status:pkg.status,signal:pkg.signal,error:pkg.error?.message??null});
assert.equal(pkg.error,undefined);assert.equal(pkg.status,0);assert.equal(pkg.signal,null);assert.equal(pkg.stderr.length,0);
mkdirSync(work+'/proof',{mode:0o700});mkdirSync(work+'/stage',{mode:0o700});
const {launchOwned}=await import(pathToFileURL(client));
const owned=await launchOwned('/usr/bin/true',[],{cwd:repo,env:process.env,output:work+'/proof',stage:work+'/stage',name:'true',deadline:performance.now()+30000});
const completed=await owned.finish();assert.deepEqual(completed.ended,{status:0,signal:null});assert.equal(completed.stdout,'');
const record=JSON.parse(readFileSync(work+'/proof/true.json'));assert.equal(record.failure,null);assert.equal(record.cleanupConfirmed,true);assert.equal(record.ownerExit.status,0);assert.equal(record.complete.cleanupConfirmed,true);assert.equal(record.complete.status,0);assert.deepEqual(record.complete.failures,[]);
const ready=record.events.filter(x=>x.kind==='ready');assert.equal(ready.length,1);assert.equal(ready[0].runtime,python);assert.equal(ready[0].subreaper,true);
assert.deepEqual({path:record.pins.runtime.path,bytes:record.pins.runtime.bytes,sha256:record.pins.runtime.sha256},expected);assert(record.pins.dependencies.length>0&&record.pins.dependencies.length<=256);
save('RESULT.json',{status:'original-system-python-owner-native-feature-and-true-cleanup-smoke-passed-not-runtime-qualification',provider:expected,nativeFeatures:native,packageVersions:pkg.stdout.toString('utf8'),ready:ready[0],dependencies:record.pins.dependencies,rootExit:completed.ended,ownerExit:record.ownerExit,complete:record.complete,cleanupConfirmed:true,scope:'Fixed host-native original owner/ready/dependency/ACK protocol under actual1000 groups[] caps5zero NNP1. No guest or product source execution.'});
process.stdout.write(JSON.stringify({status:'trusted-owner-smoke-complete',provider:expected,nativeVersion:native.version,cleanupConfirmed:true})+'\n');
