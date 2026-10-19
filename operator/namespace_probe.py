"""One fixed trusted-tool guest; AppArmor ON and exact readonly /proc, never source qualification."""
import hashlib,json,os,socket,stat,struct,subprocess,sys,time
from pathlib import Path
SOURCE='71fd69d99a39c22bdf262d1655a032aa76221f3e'
PINS={'engine.py':'f8172f41398656d513366ad6b41f2b3dadd83cc1360d751a9d84b7ccef8e693b','plan.py':'cf3c9c080303710e8613c85518864341269ea52c4673b848aa7cf3d7bc23fcfc'}
IMAGE='sha256:30ecbe1509090b38ef3260f3609d64d8261230464eaa4d235901db496e9647fb'

def require(value,message):
    if not value:raise ValueError(message)

# Byte-identical original source visible_proc; effective RO checked separately below.
def visible_proc(text):
    rows, ids = [], set()
    for line in text.splitlines():
        left, separator, right = line.partition(' - ')
        before, after = left.split(), right.split()
        require(separator and len(before) >= 6 and len(after) >= 3, 'complete proc mount row')
        require(before[0].isdigit() and before[1].isdigit() and int(before[0]) > 0 and
                before[0] not in ids, 'unique actual mount IDs')
        ids.add(before[0])
        if before[4] == '/proc':
            require(before[3] == '/' and after[0] == after[1] == 'proc' and
                    all(not option.startswith('hidepid') or option == 'hidepid=0'
                        for option in before[5].split(',') + after[2].split(',')), 'visible complete proc mount')
            rows.append({'id': before[0], 'parent': before[1], 'device': before[2],
                         'options': before[5].split(','), 'superOptions': after[2].split(',')})
    require(len(rows) in [1, 2], 'one proc mount or exact readonly remount stack')
    if len(rows) == 2:
        stacks = [(base, top) for base in rows for top in rows if top['parent'] == base['id']]
        require(len(stacks) == 1, 'unique actual proc parent-child stack')
        base, top = stacks[0]
        require(base['parent'] != top['id'] and base['device'] == top['device'] and
                base['superOptions'] == top['superOptions'] and
                {'rw', 'nosuid', 'nodev', 'noexec'} <= set(base['options']) and 'ro' not in base['options'] and
                {'ro', 'nosuid', 'nodev', 'noexec'} <= set(top['options']) and 'rw' not in top['options'],
                'same full proc filesystem with exact readonly effective remount')

def observe(root,image,name,nonce):
    out=root/'caller';result={'acceptance':False,'sourceHelperCommit':SOURCE,'image':image,'name':name};cid=None
    def save(n,v):
        p=out/n
        with p.open('xb')as f:f.write(v if isinstance(v,bytes)else(json.dumps(v,indent=2)+'\n').encode())
    try:
        status=dict(line.split(':',1)for line in Path('/proc/self/status').read_text().splitlines());status={k:v.strip()for k,v in status.items()}
        assert status['Uid'].split()==status['Gid'].split()==['1000']*4 and not status['Groups'] and status['NoNewPrivs']=='1'
        assert all(int(status[k],16)==0 for k in ['CapInh','CapPrm','CapEff','CapBnd','CapAmb']);result['actualCallerStatus']=status
        sys.path.insert(0,str(root/'original-modules'));from engine import Engine,Connection
        for n,h in PINS.items():assert hashlib.sha256((root/'original-modules'/n).read_bytes()).hexdigest()==h
        with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)as s:
            s.connect('/var/run/docker.sock');peer=struct.unpack('3i',s.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert peer[1:]==(65534,65534);result['peer']={'pid':peer[0],'uid':peer[1],'gid':peer[2]}
        engine=Engine(65534,seconds=45);result['version']=engine.json('GET','/version');assert result['version']['Version']=='28.4.0'
        config={'Image':image,'User':'65534:65534','Entrypoint':['/bin/sleep'],'Cmd':['60'],'WorkingDir':'/tmp','Env':['PATH=/bin:/usr/bin'],'Tty':False,'OpenStdin':False,'Labels':{'zryna410.host-probe':nonce},'HostConfig':{'Runtime':'runc','NetworkMode':'none','CapDrop':['ALL'],'Privileged':False,'ReadonlyRootfs':True,'SecurityOpt':['no-new-privileges:true'],'ReadonlyPaths':['/proc'],'PidsLimit':16,'Memory':67108864,'MemorySwap':67108864,'CpuPeriod':100000,'CpuQuota':200000,'CpusetCpus':'0,1','CgroupnsMode':'private','IpcMode':'none','LogConfig':{'Type':'none','Config':{}},'AutoRemove':False}}
        save('CREATE_REQUEST.json',config);cid=engine.json('POST','/containers/create?name='+name,config)['Id'];assert len(cid)==64 and all(c in '0123456789abcdef'for c in cid);result['cid']=cid
        engine.request('POST','/containers/'+cid+'/start',statuses=(204,));state=engine.json('GET','/containers/'+cid+'/json');save('INITIAL_INSPECT.json',state)
        assert state['Id']==cid and state['Image']==image and state['Name']=='/'+name and state['State']['Running'] and state['AppArmorProfile']=='docker-default'
        hc=state['HostConfig'];assert hc['ReadonlyPaths']==['/proc'] and hc['ReadonlyRootfs']is True and hc['CapDrop']==['ALL'] and hc['Privileged']is False and hc['NetworkMode']=='none' and hc['PidsLimit']==16 and hc['Memory']==hc['MemorySwap']==67108864 and hc['CpusetCpus']=='0,1'
        pid=state['State']['Pid'];mounts=Path(f'/proc/{pid}/mountinfo').read_text();save('guest-mountinfo.stdout',mounts.encode());visible_proc(mounts);proc=[x for x in mounts.splitlines()if x.split(' - ')[0].split()[4]=='/proc'];effective=proc[0]if len(proc)==1 else next(x for x in proc if any(x.split()[1]==y.split()[0]for y in proc if y!=x));flags=set(effective.split(' - ')[0].split()[5].split(','));assert {'ro','nosuid','nodev','noexec'}<=flags and 'rw'not in flags;result['actualGuestProcMount']=proc;result['effectiveReadonlyProcMount']=effective
        guest=Path(f'/proc/{pid}/status').read_text();save('guest-status.stdout',guest.encode());gs=dict(line.split(':',1)for line in guest.splitlines());assert gs['Uid'].split()==['65534']*4 and gs['Gid'].split()==['65534']*4 and gs['NoNewPrivs'].strip()=='1' and all(int(gs[k],16)==0 for k in ['CapInh','CapPrm','CapEff','CapBnd','CapAmb'])
        request={'AttachStdin':False,'AttachStdout':True,'AttachStderr':True,'Tty':False,'Privileged':False,'User':'65534:65534','WorkingDir':'/tmp','Env':['PATH=/bin:/usr/bin'],'Cmd':['/bin/true']};save('EXEC_REQUEST.json',request)
        eid=engine.json('POST','/containers/'+cid+'/exec',request)['Id'];assert len(eid)==64;result['execId']=eid
        initial=engine.json('GET','/exec/'+eid+'/json');save('EXEC_BEFORE.json',initial);assert initial['ID']==eid and initial['ContainerID']==cid and initial['ProcessConfig']['entrypoint']=='/bin/true' and initial['ProcessConfig']['arguments']in(None,[]) and initial['ProcessConfig']['user']=='65534:65534' and initial['ProcessConfig']['privileged']is False and initial['ProcessConfig']['tty']is False
        connection=Connection(time.monotonic()+10,65534)
        try:
            connection.request('POST','/v1.49/exec/'+eid+'/start',body=b'{"Detach":false,"Tty":false}',headers={'Content-Type':'application/json','Connection':'close'});response=connection.getresponse();body=response.read(4097);assert len(body)<=4096
            save('EXEC_START_RESPONSE.bin',body);save('EXEC_START_HTTP.json',{'status':response.status,'headers':response.getheaders(),'bytes':len(body),'sha256':hashlib.sha256(body).hexdigest()});start_status=response.status
        finally:connection.close()
        ended=engine.json('GET','/exec/'+eid+'/json');save('EXEC_AFTER.json',ended);assert start_status==200 and body==b'';assert ended['ID']==eid and ended['ContainerID']==cid and ended['Running']is False and ended['ExitCode']==0
        end=engine.json('GET','/containers/'+cid+'/json');save('FINAL_INSPECT.json',end);assert end['Id']==cid and end['State']['Running'] and end['State']['Pid']==pid and end['AppArmorProfile']=='docker-default'and end['HostConfig']['ReadonlyPaths']==['/proc']
        result['status']='fixed-proc-readonly-apparmor-exec-passed-not-product-qualification'
    except BaseException as e:result['failure']={'type':type(e).__name__,'message':str(e)[:2000]}
    finally:
        if cid:
            try:
                cleanup=Engine(65534,seconds=10);s=cleanup.json('GET','/containers/'+cid+'/json');assert s['Id']==cid and s['Image']==image and s['Name']=='/'+name and s['Config']['Labels']=={'zryna410.host-probe':nonce};save('CLEANUP_INSPECT.json',s)
                cleanup.request('DELETE','/containers/'+cid+'?force=1&v=0',statuses=(204,));raw,_=cleanup.request('GET','/containers/'+cid+'/json',statuses=(404,));save('ABSENCE_RESPONSE.bin',raw);assert isinstance(json.loads(raw),dict);result['exactCIDAbsent']=True
            except BaseException as e:result['cleanupFailure']={'type':type(e).__name__,'message':str(e)[:2000]}
        save('RESULT.json',result)
    return 0 if result.get('exactCIDAbsent')and not result.get('failure')and not result.get('cleanupFailure')else 1

def setup(root,source,image,name,nonce):
    assert os.geteuid()==0 and image==IMAGE and name=='zryna410-host-probe-'+nonce and len(nonce)==32
    subprocess.run(['mount','--make-rprivate','/'],check=True);before=[x for x in Path('/proc/self/mountinfo').read_text().splitlines()if x.split(' - ')[0].split()[4]=='/sys/fs/cgroup'];assert len(before)==1
    options=before[0].split(' - ')[1].split()[2].split(',');assert set(options)<={'rw','nsdelegate','memory_recursiveprot'}and'rw'in options
    subprocess.run(['umount','/sys/fs/cgroup'],check=True);subprocess.run(['mount','-t','cgroup2','-o',','.join(['ro','nosuid','nodev','noexec']+[x for x in options if x!='rw']),'cgroup2','/sys/fs/cgroup'],check=True)
    runtime=root/'namespace-run';runtime.mkdir(mode=0o755);os.chown(runtime,65534,65534);subprocess.run(['mount','--bind',str(runtime),'/run'],check=True)
    modules=root/'original-modules';modules.mkdir(mode=0o755)
    for n,h in PINS.items():
        raw=(source/'examples/playground/restricted/daemon/preparation'/n).read_bytes();assert hashlib.sha256(raw).hexdigest()==h;p=modules/n;p.write_bytes(raw);p.chmod(0o444)
    helper=root/'readonly-proc-observer.py';helper.write_bytes(Path(__file__).read_bytes());helper.chmod(0o444)
    code=root/'broker-reviewed.py';code.write_bytes(Path(__file__).with_name('broker.py').read_bytes());code.chmod(0o444)
    def nobody():os.setgroups([]);os.setgid(65534);os.setuid(65534)
    broker=subprocess.Popen(['/usr/bin/python3','-I','-B',str(code),str(root/'engine.sock')],preexec_fn=nobody,stdout=subprocess.PIPE,stderr=subprocess.PIPE);result={'acceptance':False,'sourceHelperCommit':SOURCE}
    try:
        for _ in range(100):
            if Path('/var/run/docker.sock').exists():break
            assert broker.poll()is None;time.sleep(.02)
        p=Path('/var/run/docker.sock');s=p.lstat();assert stat.S_ISSOCK(s.st_mode)and s.st_uid==65534;os.chown(p,-1,1000);os.chmod(p,0o660);t=p.lstat();assert(s.st_dev,s.st_ino,t.st_uid,t.st_gid,stat.S_IMODE(t.st_mode))==(t.st_dev,t.st_ino,65534,1000,0o660)
        out=root/'caller';out.mkdir(mode=0o700);os.chown(out,1000,1000)
        argv=['/usr/bin/setpriv','--reuid=1000','--regid=1000','--clear-groups','--inh-caps=-all','--bounding-set=-all','--ambient-caps=-all','--no-new-privs','/usr/bin/python3','-I','-B',str(helper),'observe',str(root),image,name,nonce]
        with(root/'caller.stdout').open('xb')as stdout,(root/'caller.stderr').open('xb')as stderr:p=subprocess.run(argv,stdout=stdout,stderr=stderr,stdin=subprocess.DEVNULL,timeout=65)
        result.update(observerArgv=argv,observerExit=p.returncode,actual=json.loads((out/'RESULT.json').read_bytes()));assert p.returncode==0
    finally:
        broker.terminate()
        try:out,err=broker.communicate(timeout=5)
        except subprocess.TimeoutExpired:broker.kill();out,err=broker.communicate(timeout=2)
        (root/'broker.stdout').write_bytes(out);(root/'broker.stderr').write_bytes(err);result['brokerExit']=broker.returncode
        result['retainedCallerFiles']=[];total=0
        for original in sorted((root/'caller').iterdir()):
            before=original.lstat();assert stat.S_ISREG(before.st_mode)and before.st_uid==1000 and before.st_nlink==1 and not original.is_symlink()and before.st_size<=4194304
            raw=original.read_bytes();after=original.lstat();assert all(getattr(before,k)==getattr(after,k)for k in ['st_dev','st_ino','st_mode','st_nlink','st_uid','st_gid','st_size','st_mtime_ns','st_ctime_ns']);total+=len(raw);assert len(result['retainedCallerFiles'])<32 and total<=8388608
            target=root/('caller__'+original.name)
            with target.open('xb')as f:f.write(raw)
            target.chmod(0o444);result['retainedCallerFiles'].append({'originalPath':str(original),'originalOwnerUid':before.st_uid,'originalOwnerGid':before.st_gid,'originalMode':stat.S_IMODE(before.st_mode),'capturedPath':str(target),'bytes':len(raw),'sha256':hashlib.sha256(raw).hexdigest()})
        (root/'namespace-result.json').write_text(json.dumps(result,indent=2)+'\n')

if __name__=='__main__':
    if sys.argv[1]=='observe':sys.exit(observe(Path(sys.argv[2]),*sys.argv[3:]))
    setup(Path(sys.argv[1]),Path(sys.argv[2]),*sys.argv[3:])
