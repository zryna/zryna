"""ROOT ONLY push-branch diagnostic. One scoped profile; standard systemd owns the service."""
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
need(os.geteuid()==0 and len(sys.argv)==2,'root fixed diagnostic root required')
root=canonical(sys.argv[1]);need(re.fullmatch(r'm6-editor-diag-[1-9][0-9]*-[1-9][0-9]*',os.path.basename(root)),'fixed run root')
raw,config_stat=ordinary(root+'/CONFIG.json');c=json.loads(raw)
need(c['format']=='private.editor-scoped-userns-diagnostic.v1' and c['root']==root and type(c['uid']) is int and c['uid']>0 and type(c['gid']) is int and c['gid']>=0 and c['uid']==config_stat.st_uid==os.stat(root).st_uid,'original nonroot runner')
account=pwd.getpwuid(c['uid']);need(c['gid']==config_stat.st_gid==os.stat(root).st_gid==account.pw_gid,'original runner primary gid and metadata')
repo=str(Path(__file__).resolve().parents[3]);need(c['repo']==repo and c['stage']==root+'/stage','fixed source/stage paths')
unit=c['unit'];need(re.fullmatch(r'm6-editor-diag-[1-9][0-9]*-[1-9][0-9]*-[a-f0-9]{12}\.service',unit),'unique literal unit')
node='/opt/hostedtoolcache/node/22.22.1/x64/bin/node';need(c['node']['path']==node,'fixed setup-node bootstrap')
probe=repo+'/scripts/m6/installed-editor/diagnostic-probe.mjs';need(c['probe']['path']==probe,'fixed diagnostic probe only')
code=c['stage']+'/editor/VSCode-linux-x64/code';helper=c['stage']+'/editor/VSCode-linux-x64/chrome-sandbox'
need(c['code']['path']==code and c['code']['bytes']==219843800 and c['code']['sha256']=='34912494509486296a3bd9aecb512694c98523d0f6f23cfceb5a95b5088c721b','exact selected Code')
need(c['helper']['path']==helper and c['helper']['bytes']==15232 and c['helper']['sha256']=='2015304df43beff0b74409cdc4d858107efcde650d9bc8afb1288041d62cec4b','exact unchanged helper')
for item in (c['node'],c['probe'],c['code'],c['helper'],c['prepared']):ordinary(item['path'],268435456,item)
need(stat.S_IMODE(os.lstat(helper).st_mode)==0o755 and os.lstat(helper).st_uid==c['uid'],'helper stays unprivileged0755')
canonical(os.path.dirname(code));profile_name=unit.removesuffix('.service')+'-code'
profile=('abi <abi/4.0>,\nprofile '+profile_name+' "'+code+'" flags=(unconfined) {\n  userns,\n}\n').encode()
private='/run/'+unit.removesuffix('.service');os.mkdir(private,0o700);profile_path=private+'/scoped-code.profile'
pfd=os.open(profile_path,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW|os.O_CLOEXEC,0o600)
try:
 remaining_profile=profile
 while remaining_profile:
  n=os.write(pfd,remaining_profile);need(n>0,'complete profile reservation');remaining_profile=remaining_profile[n:]
 os.fsync(pfd)
finally:os.close(pfd)
root_dir=os.open(root,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW|os.O_CLOEXEC);root_identity=os.fstat(root_dir)
private_dir=os.open(private,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW|os.O_CLOEXEC);private_identity=os.fstat(private_dir)
need(root_identity.st_uid==c['uid'] and stat.S_IMODE(root_identity.st_mode)==0o700 and private_identity.st_uid==0 and stat.S_IMODE(private_identity.st_mode)==0o700,'protected retained parents')
fd=os.open(root+'/ROOT_RESULT.json',os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW|os.O_CLOEXEC,0o644)
record_identity=os.fstat(fd)
record={'format':'private.editor-scoped-userns-diagnostic-root.v1','diagnosticOnly':True,'qualification':False,'unit':unit,'source':c['source'],'profile':{'name':profile_name,'path':profile_path,'bytes':len(profile),'sha256':hashlib.sha256(profile).hexdigest(),'content':profile.decode()},'commands':[],'snapshots':[],'codeReleased':False,'cleanupProven':False,'profileUnloaded':False,'failure':None}
end=time.monotonic()+100;anchor=None;child=None;loaded=False;stream_files=[];selector=None;sizes={}
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
 original=record.get('originalUnit');return original is not None and state.get('Id')==unit and state.get('InvocationID')==original['InvocationID'] and state.get('ControlGroup')==original['ControlGroup'] and state.get('User')==str(c['uid']) and state.get('Group')==str(c['gid']) and state.get('KillMode')=='control-group' and state.get('RuntimeMaxUSec') in ('1min','60s','60000000') and state.get('TimeoutStopUSec') in ('15s','15000000') and state.get('Restart')=='no'
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
try:
 store();need(profile_name not in profiles(),'profile name must be absent');need(show().get('LoadState')=='not-found' and not os.path.lexists(expected_cgroup),'unit name and literal cgroup must be absent');record['initialUnitAndCgroupAbsent']=True;store()
 load=run(['/usr/sbin/apparmor_parser','-a',profile_path]);need(load.returncode==0,'exact scoped profile load');loaded=True;need(profile_name in profiles(),'exact loaded profile observed')
 home=pwd.getpwuid(c['uid']).pw_dir
 argv=['/usr/bin/systemd-run','--unit='+unit,'--property=Type=exec','--property=Restart=no','--property=User='+str(c['uid']),'--property=Group='+str(c['gid']),'--property=KillMode=control-group','--property=RuntimeMaxSec=60s','--property=TimeoutStopSec=15s','--wait','--pipe','--collect','--setenv=PATH=/usr/bin:/bin','--setenv=HOME='+home,'--setenv=LANG=C.UTF-8','--setenv=TMPDIR='+c['stage']+'/tmp','--','/usr/bin/xvfb-run','-a',node,probe,root]
 record['serviceArgv']=argv;store();child=subprocess.Popen(argv,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env={'PATH':'/usr/sbin:/usr/bin:/sbin:/bin','LANG':'C.UTF-8'})
 selector=selectors.DefaultSelector();sizes={}
 for index,pipe in enumerate([child.stdout,child.stderr]):
  os.set_blocking(pipe.fileno(),False);selector.register(pipe,selectors.EVENT_READ,index);sizes[index]=0;stream_files.append(os.open(root+'/SERVICE.'+['stdout','stderr'][index],os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o644))
 ready_sample=False;last_show=0;released_at=None;early_count=0;next_early=0
 while child.poll() is None:
  need(time.monotonic()<end-18,'bounded root service observation')
  drain(.05)
  if anchor is None and time.monotonic()-last_show>.1:
   last_show=time.monotonic();state=show();group=state.get('ControlGroup','')
   if group:
    need(group=='/system.slice/'+unit and state.get('Id')==unit and re.fullmatch('[a-f0-9]{32}',state.get('InvocationID','')) and state.get('User')==str(c['uid']) and state.get('Group')==str(c['gid']) and state.get('KillMode')=='control-group' and state.get('RuntimeMaxUSec') in ('1min','60s','60000000') and state.get('TimeoutStopUSec') in ('15s','15000000') and state.get('Restart')=='no','exact created service/user/group/controlgroup/budgets')
    path='/sys/fs/cgroup'+group;s=os.stat(path,follow_symlinks=False);anchor=(path,s.st_dev,s.st_ino);record['originalUnit']=state;record['cgroupAnchor']={'path':path,'dev':s.st_dev,'ino':s.st_ino};snapshot('BASELINE_BEFORE_CODE')
    gate=os.open(root+'/RELEASE_START',os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o644);os.fsync(gate);os.close(gate);record['codeReleased']=True;released_at=time.monotonic();next_early=released_at;store()
  if released_at is not None and early_count<10 and time.monotonic()<=released_at+2 and time.monotonic()>=next_early:
   early_count+=1;snapshot('EARLY_STARTUP_'+str(early_count));next_early=time.monotonic()+.1
  if not ready_sample and os.path.exists(root+'/GUI_READY.json'):snapshot('GUI_READY_HOLD');ready_sample=True
 record['serviceClientStatus']=child.wait(timeout=2);drain_end();snapshot('SERVICE_TERMINAL');record['unitTerminal']=show()
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
  ordinary(code,268435456,c['code']);ordinary(helper,65536,c['helper']);need(stat.S_IMODE(os.lstat(helper).st_mode)==0o755,'helper remains0755')
  record['helperAndCodeBytesUnchanged']=True
 except BaseException as e:record['cleanupFailure']={'type':type(e).__name__,'message':str(e)[:1024]}
 for stream in stream_files:os.fsync(stream);os.close(stream)
 store();os.close(fd);os.close(root_dir);os.close(private_dir)

if record['failure'] or any(record.get(k) for k in ['cleanupFailure','streamDrainFailure','stopRefusedUnknownInvocation','stopFailure']) or not record['cleanupProven'] or not record['profileUnloaded'] or record.get('stopStatus',0)!=0 or record.get('serviceClientStatus')!=0:sys.exit(1)
