"""Trusted Linux installed-editor integration. One exact Code profile; systemd owns outer rescue only."""
import hashlib,json,os,pwd,re,selectors,stat,subprocess,sys,time
from pathlib import Path
FIELDS=('st_dev','st_ino','st_mode','st_uid','st_gid','st_nlink','st_size','st_mtime_ns','st_ctime_ns')
def need(v,message):
 if not v:raise RuntimeError(message)
def ordinary(path,limit=8388608,expected=None):
 fd=os.open(path,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK|os.O_CLOEXEC)
 try:
  a=os.fstat(fd);need(stat.S_ISREG(a.st_mode) and a.st_nlink==1 and 0<=a.st_size<=limit,'ordinary bounded file')
  chunks=[];count=0;h=hashlib.sha256()
  while count<=a.st_size:
   b=os.read(fd,min(65536,a.st_size+1-count))
   if not b:break
   count+=len(b);h.update(b);chunks.append(b)
  z=os.fstat(fd);named=os.stat(path,follow_symlinks=False)
  need(count==a.st_size and all(getattr(a,k)==getattr(z,k)==getattr(named,k) for k in FIELDS),'stable named file')
  if expected:need(expected['path']==path and count==expected['bytes'] and h.hexdigest()==expected['sha256'],'exact pinned bytes')
  return b''.join(chunks),a
 finally:os.close(fd)
def canonical(path):
 need(os.path.isabs(path) and os.path.realpath(path)==path and re.fullmatch(r'/[A-Za-z0-9._/-]+',path),'canonical path')
 here=path
 while True:
  need(stat.S_ISDIR(os.lstat(here).st_mode),'ordinary parent')
  parent=os.path.dirname(here)
  if here==parent:break
  here=parent
 return path
need(os.geteuid()==0 and len(sys.argv)==22,'root fixed gate and ten owning argument pairs required')
root=canonical(sys.argv[1]);need(re.fullmatch(r'm6-editor-linux-[1-9][0-9]*-[1-9][0-9]*',os.path.basename(root)),'fixed run root')
rstat=os.stat(root);account=pwd.getpwuid(rstat.st_uid);need(rstat.st_uid>0 and rstat.st_gid==account.pw_gid and stat.S_IMODE(rstat.st_mode)==0o700,'original runner primary uid/gid')
repo=str(Path(__file__).resolve().parents[3]);node='/opt/hostedtoolcache/node/22.22.1/x64/bin/node';probe=repo+'/scripts/m6/installed-editor.mjs'
names=['--handoff','--editor-archive','--source-commit','--source-tree','--source-epoch','--setup-archive-sha256','--setup-manifest-sha256','--server-sha256','--vsix-sha256','--output']
args=sys.argv[2:];need(args[::2]==names,'only original fixed argument grammar');values=dict(zip(names,args[1::2]))
parent=canonical(os.path.dirname(root));need(values['--handoff']==parent+'/m6-editor-handoff' and values['--editor-archive']==parent+'/m6-editor-material/editor.tar.gz' and values['--output']==parent+'/m6-installed-editor','fixed workflow paths')
need(all(re.fullmatch('[a-f0-9]{40}',values[k]) for k in ['--source-commit','--source-tree']) and re.fullmatch('[0-9]{1,12}',values['--source-epoch']) and all(re.fullmatch('[a-f0-9]{64}',values[k]) for k in names[5:9]),'fixed source/material pins')
for path in [node,probe]:ordinary(path,134217728)
c={'uid':rstat.st_uid,'gid':rstat.st_gid,'stage':root+'/tmp','source':{'commit':values['--source-commit'],'tree':values['--source-tree']}}
unit=os.path.basename(root)+'-'+os.urandom(6).hex()+'.service';profile_name=unit.removesuffix('.service')+'-code'
private='/run/'+unit.removesuffix('.service');os.mkdir(private,0o700);profile_path=private+'/scoped-code.profile'
profile=None;code=None;helper=None
root_dir=os.open(root,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW|os.O_CLOEXEC);root_identity=os.fstat(root_dir)
private_dir=os.open(private,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW|os.O_CLOEXEC);private_identity=os.fstat(private_dir)
need(root_identity.st_uid==c['uid'] and stat.S_IMODE(root_identity.st_mode)==0o700 and private_identity.st_uid==0 and stat.S_IMODE(private_identity.st_mode)==0o700,'protected retained parents')
fd=os.open(root+'/ROOT_RESULT.json',os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW|os.O_CLOEXEC,0o644)
record_identity=os.fstat(fd)
record={'format':'zryna.m6-editor-linux-service.v1','rescueDoesNotGrantInnerAcceptance':True,'unit':unit,'source':c['source'],'profile':None,'commands':[],'snapshots':[],'codeReleased':False,'cleanupProven':False,'profileUnloaded':False,'failure':None}
end=time.monotonic()+1200;anchor=None;child=None;loaded=False;stream_files=[];selector=None;sizes={}
expected_cgroup='/sys/fs/cgroup/system.slice/'+unit
def barrier():
 for path,held,original in [(root,root_dir,root_identity),(private,private_dir,private_identity)]:
  a=os.fstat(held);b=os.stat(path,follow_symlinks=False);need(all(getattr(a,k)==getattr(b,k)==getattr(original,k) for k in ['st_dev','st_ino','st_mode','st_uid','st_gid']),'same retained protected parent')
 a=os.fstat(fd);b=os.stat('ROOT_RESULT.json',dir_fd=root_dir,follow_symlinks=False);need(a.st_dev==b.st_dev==record_identity.st_dev and a.st_ino==b.st_ino==record_identity.st_ino and a.st_nlink==b.st_nlink==1 and a.st_uid==b.st_uid==0,'same durable root result')
 os.fsync(root_dir);os.fsync(private_dir)
def store():
 b=(json.dumps(record,indent=2)+'\n').encode();need(len(b)<=8388608,'bounded diagnostic result');os.lseek(fd,0,0);os.ftruncate(fd,0)
 while b:
  n=os.write(fd,b);need(n>0,'result write');b=b[n:]
 os.fsync(fd);barrier()
def run(args,timeout=4):
 need(time.monotonic()<end,'root absolute deadline');barrier();v=subprocess.run(args,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=min(timeout,max(.1,end-time.monotonic())),check=False,env={'PATH':'/usr/sbin:/usr/bin:/sbin:/bin','LANG':'C.UTF-8'})
 need(len(v.stdout)+len(v.stderr)<=1048576,'bounded platform output');record['commands'].append({'argv':args,'status':v.returncode,'stdout':v.stdout.decode('utf8','replace'),'stderr':v.stderr.decode('utf8','replace')});store();return v
properties=['Restart','Id','LoadState','ActiveState','SubState','ControlGroup','InvocationID','MainPID','User','Group','KillMode','RuntimeMaxUSec','TimeoutStopUSec','Delegate','Result']
def show():
 v=run(['/usr/bin/systemctl','show',unit,'--no-pager','--property='+','.join(properties)]);return dict(line.split('=',1) for line in v.stdout.decode().splitlines() if '=' in line)
def profiles():
 with open('/sys/kernel/security/apparmor/profiles','rb',buffering=0) as f:data=f.read(1048577)
 need(len(data)<=1048576,'bounded profile list');return [line.decode().split(' ',1)[0] for line in data.splitlines()]
def bounded(path,limit=16384):
 with open(path,'rb',buffering=0) as f:data=f.read(limit+1)
 need(len(data)<=limit,'bounded readonly kernel metadata');return data.decode('ascii')
def same_unit(state):
 original=record.get('originalUnit');return original is not None and state.get('Id')==unit and state.get('InvocationID')==original['InvocationID'] and state.get('ControlGroup')==original['ControlGroup'] and state.get('User')==str(c['uid']) and state.get('Group')==str(c['gid']) and state.get('KillMode')=='control-group' and state.get('RuntimeMaxUSec') in ('18min 20s','1100s','1100000000') and state.get('TimeoutStopUSec') in ('15s','15000000') and state.get('Restart')=='no'
def drain(wait=.05):
 if selector is None:return
 for key,_ in selector.select(wait):
  data=os.read(key.fileobj.fileno(),65536)
  if not data:selector.unregister(key.fileobj);continue
  sizes[key.data]+=len(data);need(sizes[key.data]<=1048576,'bounded service stream')
  while data:
   n=os.write(stream_files[key.data],data);need(n>0,'complete service stream write');data=data[n:]
def drain_end():
 until=min(end,time.monotonic()+2)
 while selector is not None and selector.get_map() and time.monotonic()<until:drain(.05)
 need(selector is None or not selector.get_map(),'service pipe EOF unconfirmed')
def snapshot(phase):
 need(len(record['snapshots'])<14 and sum(len(x['processes']) for x in record['snapshots'])<=4096,'bounded snapshot count and aggregate rows')
 row={'phase':phase,'atMonotonicMs':int(time.monotonic()*1000),'available':False,'processes':[]}
 try:
  need(anchor is not None and same_unit(show()),'no current original invocation/cgroup anchor');path,dev,ino=anchor;s=os.stat(path,follow_symlinks=False);need((s.st_dev,s.st_ino)==(dev,ino),'same exact unit cgroup inode')
  pids=bounded(path+'/cgroup.procs',4096).split();need(len(pids)<=256 and all(x.isdecimal() for x in pids),'bounded exact unit members')
  for name in pids:
   item={'pid':int(name)}
   try:
    base='/proc/'+name;raw=bounded(base+'/stat',8192);finish=raw.rindex(') ');parts=raw[finish+2:].split();item.update(start=parts[19],state=parts[0],parent=int(parts[1]))
    status=dict(x.split(':',1) for x in bounded(base+'/status').splitlines() if ':' in x)
    item['status']={k:status[k].strip() for k in ['Uid','Gid','NSpid','NoNewPrivs','Seccomp','Seccomp_filters','CapInh','CapPrm','CapEff','CapBnd','CapAmb'] if k in status}
    item['namespaces']={k:os.readlink(base+'/ns/'+k) for k in ['user','pid','mnt','net','ipc','uts','cgroup']}
    item['uidMap']=bounded(base+'/uid_map',4096);item['gidMap']=bounded(base+'/gid_map',4096)
    item['exe']=os.readlink(base+'/exe');again=bounded(base+'/stat',8192);need(again[again.rindex(') ')+2:].split()[19]==item['start'],'same sampled process epoch');item['available']=True
   except OSError as e:item.update(available=False,errno=e.errno)
   row['processes'].append(item)
  after=os.stat(path,follow_symlinks=False);need((after.st_dev,after.st_ino)==(dev,ino),'same cgroup after snapshot');row['available']=True
 except (OSError,RuntimeError) as e:row['unavailable']={'type':type(e).__name__,'errno':getattr(e,'errno',None)}
 row['finishedMonotonicMs']=int(time.monotonic()*1000);record['snapshots'].append(row);store()
def cgroup_clear():
 if anchor is None:return False
 path,dev,ino=anchor
 try:
  s=os.stat(path,follow_symlinks=False);need((s.st_dev,s.st_ino)==(dev,ino),'same original unit cgroup');members=bounded(path+'/cgroup.procs',4096).split();events=dict(x.split() for x in bounded(path+'/cgroup.events',4096).splitlines());return members==[] and events.get('populated')=='0'
 except FileNotFoundError:return True
def grant():
 global profile,code,helper,loaded
 marker,ms=ordinary(root+'/REQUEST.ready',0);need(ms.st_uid==c['uid'] and ms.st_gid==c['gid'] and stat.S_IMODE(ms.st_mode)==0o400,'original runner completed request marker');raw,request_stat=ordinary(root+'/REQUEST.json',65536);request=json.loads(raw);need(request_stat.st_uid==c['uid'] and request_stat.st_gid==c['gid'],'runner request identity')
 need(set(request)==set(['format','root','output','stage','invocationId','uid','gid','productCommit','prepared','code','helper']) and request['format']=='zryna.m6-editor-linux-userns-request.v1' and request['root']==root and request['output']==values['--output'] and request['uid']==c['uid'] and request['gid']==c['gid'],'fixed request');need(request['invocationId']==record['originalUnit']['InvocationID'],'request from original service invocation')
 stage=canonical(request['stage']);need(os.path.dirname(stage)==root+'/tmp' and re.fullmatch('zryna-m6-installed-editor-[A-Za-z0-9]+',os.path.basename(stage)),'exact mkdtemp stage')
 code=stage+'/editor/VSCode-linux-x64/code';helper=stage+'/editor/VSCode-linux-x64/chrome-sandbox';canonical(os.path.dirname(code))
 need(request['productCommit']=='7debcd0e2acdea1c52de81bf9ee1620444407dda','authenticated product commit')
 need(request['code']['path']==code and request['code']['bytes']==219843800 and request['code']['sha256']=='34912494509486296a3bd9aecb512694c98523d0f6f23cfceb5a95b5088c721b','exact Code')
 need(request['helper']['path']==helper and request['helper']['bytes']==15232 and request['helper']['sha256']=='2015304df43beff0b74409cdc4d858107efcde650d9bc8afb1288041d62cec4b','exact helper')
 need(request['prepared']['path']==values['--output']+'/PREPARED.json','fixed owning preparation receipt')
 prepared_raw,_=ordinary(request['prepared']['path'],8388608,request['prepared']);prepared=json.loads(prepared_raw)
 need(prepared['source']==c['source'] and prepared['stage']==stage and prepared['editor']['descriptor']==request['code'] and prepared['editor']['productCommit']==request['productCommit'] and prepared['archive']['bytes']==344482658 and prepared['archive']['sha256']=='59fb0f88eab2fe2e3a053ac020e18fcb2b5c113d255e41c82d26302b675d580c','owning complete archive/preparation correspondence')
 for item in [request['code'],request['helper']]:ordinary(item['path'],268435456,item)
 need(stat.S_IMODE(os.lstat(helper).st_mode)==0o755 and os.lstat(helper).st_uid==c['uid'],'helper unchanged runner0755')
 need(same_unit(show()),'original invocation before profile load')
 profile=('abi <abi/4.0>,\nprofile '+profile_name+' "'+code+'" flags=(unconfined) {\n  userns,\n}\n').encode()
 pfd=os.open(profile_path,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW|os.O_CLOEXEC,0o600)
 try:
  pending=profile
  while pending:
   n=os.write(pfd,pending);need(n>0,'complete profile reservation');pending=pending[n:]
  os.fsync(pfd)
 finally:os.close(pfd)
 record['request']={'sha256':hashlib.sha256(raw).hexdigest(),'value':request};record['profile']={'name':profile_name,'path':profile_path,'sha256':hashlib.sha256(profile).hexdigest(),'bytes':len(profile),'content':profile.decode()};store()
 load=run(['/usr/sbin/apparmor_parser','-a',profile_path]);need(load.returncode==0,'exact scoped profile load');loaded=True;need(profile_name in profiles() and same_unit(show()),'exact loaded profile and original invocation')
 data=(json.dumps({'format':'zryna.m6-editor-linux-userns-grant.v1','requestSha256':record['request']['sha256'],'unit':unit,'invocationId':record['originalUnit']['InvocationID'],'cgroup':record['cgroupAnchor']})+'\n').encode()
 gate=os.open(root+'/GRANT.json',os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o444)
 try:
  while data:
   n=os.write(gate,data);need(n>0,'complete grant');data=data[n:]
  os.fsync(gate)
 finally:os.close(gate)
 store();ready=os.open(root+'/GRANT.ready',os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o444);os.fsync(ready);os.close(ready);record['codeReleased']=True;store()
try:
 store();need(profile_name not in profiles(),'profile name must be absent');need(show().get('LoadState')=='not-found' and not os.path.lexists(expected_cgroup),'unit name and literal cgroup must be absent');record['initialUnitAndCgroupAbsent']=True;store()
 home=pwd.getpwuid(c['uid']).pw_dir
 argv=['/usr/bin/systemd-run','--unit='+unit,'--property=Type=exec','--property=Restart=no','--property=User='+str(c['uid']),'--property=Group='+str(c['gid']),'--property=KillMode=control-group','--property=RuntimeMaxSec=1100s','--property=TimeoutStopSec=15s','--wait','--pipe','--collect','--setenv=PATH=/usr/bin:/bin','--setenv=HOME='+home,'--setenv=LANG=C.UTF-8','--setenv=TMPDIR='+c['stage'],'--setenv=ZRYNA_M6_EDITOR_LINUX_GATE='+root,'--','/usr/bin/xvfb-run','-a',node,probe,*args]
 record['serviceArgv']=argv;store();child=subprocess.Popen(argv,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env={'PATH':'/usr/sbin:/usr/bin:/sbin:/bin','LANG':'C.UTF-8'})
 selector=selectors.DefaultSelector();sizes={}
 for index,pipe in enumerate([child.stdout,child.stderr]):
  os.set_blocking(pipe.fileno(),False);selector.register(pipe,selectors.EVENT_READ,index);sizes[index]=0;stream_files.append(os.open(root+'/SERVICE.'+['stdout','stderr'][index],os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o644))
 last_show=0
 while child.poll() is None:
  need(time.monotonic()<end-18,'bounded root service observation')
  drain(.05)
  if anchor is None and time.monotonic()-last_show>.1:
   last_show=time.monotonic();state=show();group=state.get('ControlGroup','')
   if group:
    need(group=='/system.slice/'+unit and state.get('Id')==unit and re.fullmatch('[a-f0-9]{32}',state.get('InvocationID','')) and state.get('User')==str(c['uid']) and state.get('Group')==str(c['gid']) and state.get('KillMode')=='control-group' and state.get('RuntimeMaxUSec') in ('18min 20s','1100s','1100000000') and state.get('TimeoutStopUSec') in ('15s','15000000') and state.get('Restart')=='no','exact created service/user/group/controlgroup/budgets')
    path='/sys/fs/cgroup'+group;s=os.stat(path,follow_symlinks=False);anchor=(path,s.st_dev,s.st_ino);record['originalUnit']=state;record['cgroupAnchor']={'path':path,'dev':s.st_dev,'ino':s.st_ino};snapshot('ORIGINAL_SERVICE_BEFORE_GRANT')
  if anchor is not None and not record['codeReleased'] and os.path.exists(root+'/REQUEST.ready'):grant()
 record['serviceClientStatus']=child.wait(timeout=2);drain_end();snapshot('SERVICE_TERMINAL');record['unitTerminal']=show()
 final_raw,_=ordinary(values['--output']+'/RESULT.json',8388608);final=json.loads(final_raw);record['innerResult']={'sha256':hashlib.sha256(final_raw).hexdigest(),'bytes':len(final_raw),'accepted':final.get('accepted'),'primaryFailure':final.get('primaryFailure'),'cleanupFailure':final.get('cleanupFailure')};need(final.get('accepted') is True and final.get('source')==c['source'] and final.get('passed')==9 and final.get('failed')==0 and final.get('skipped')==0,'original nine checks and original owner acceptance required')
except BaseException as e:record['failure']={'type':type(e).__name__,'message':str(e)[:1024]};store()
finally:
 try:
  state=show()
  if state.get('LoadState')!='not-found' and state.get('ActiveState') not in ('inactive','failed'):
   if same_unit(state):
    stop=run(['/usr/bin/systemctl','stop',unit],17);record['stopStatus']=stop.returncode
   else:record['stopRefusedUnknownInvocation']=True
 except BaseException as e:record['stopFailure']={'type':type(e).__name__,'message':str(e)[:1024]}
 try:
  while child is not None and child.poll() is None and time.monotonic()<end-5:
   try:drain(.05)
   except BaseException as e:record['streamDrainFailure']={'type':type(e).__name__,'message':str(e)[:1024]}
  if child is not None:
   record['serviceClientFinalStatus']=child.poll()
   if child.poll() is not None:drain_end()
  state=show();record['unitAfterStop']=state
  if anchor is not None:
   until=min(end-4,time.monotonic()+15)
   while not cgroup_clear() and time.monotonic()<until:time.sleep(.05)
   record['cleanupProven']=cgroup_clear() and state.get('ActiveState') in ('inactive','failed') and state.get('LoadState') in ('loaded','not-found')
   record['cleanupBasis']='ORIGINAL_CGROUP_EMPTY_OR_GONE'
  else:
   absent=state.get('LoadState')=='not-found' and not os.path.lexists(expected_cgroup)
   record['cleanupProven']=child is None and not record['codeReleased'] and record.get('initialUnitAndCgroupAbsent') is True and absent
   record['partialCreationManagerAndLiteralCgroupGone']=child is not None and child.poll() is not None and not record['codeReleased'] and absent
   record['cleanupBasis']='NO_SERVICE_CREATION_ATTEMPT' if record['cleanupProven'] else 'UNCONFIRMED_PARTIAL_CREATION_NO_IDENTITY_ADOPTION'
  no_active_code=record['cleanupProven'] or record.get('partialCreationManagerAndLiteralCgroupGone') is True
  if profile_name in profiles() and no_active_code:
   current,_=ordinary(profile_path,65536);need(current==profile,'same exact one-profile content before unload');unload=run(['/usr/sbin/apparmor_parser','-R',profile_path]);record['profileUnloaded']=unload.returncode==0 and profile_name not in profiles()
  elif profile_name not in profiles():record['profileUnloaded']=True
  if record.get('request'):
   request=record['request']['value'];ordinary(code,268435456,request['code']);ordinary(helper,65536,request['helper']);need(stat.S_IMODE(os.lstat(helper).st_mode)==0o755,'helper remains0755');record['helperAndCodeBytesUnchanged']=True
 except BaseException as e:record['cleanupFailure']={'type':type(e).__name__,'message':str(e)[:1024]}
 for stream in stream_files:os.fsync(stream);os.close(stream)
 store();os.close(fd);os.close(root_dir);os.close(private_dir)

if record['failure'] or any(record.get(k) for k in ['cleanupFailure','streamDrainFailure','stopRefusedUnknownInvocation','stopFailure']) or not record['cleanupProven'] or not record['profileUnloaded'] or record.get('stopStatus',0)!=0 or record.get('serviceClientStatus')!=0:sys.exit(1)
