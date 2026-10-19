"""Inert V6 extension: two genuine nonroot observers of one fixed diagnostic guest."""
import hashlib,json,os,socket,stat,struct,subprocess,sys,time
from pathlib import Path
SOURCE='71fd69d99a39c22bdf262d1655a032aa76221f3e'
PINS={'examples/playground/restricted/daemon/preparation/engine.py': 'f8172f41398656d513366ad6b41f2b3dadd83cc1360d751a9d84b7ccef8e693b', 'examples/playground/restricted/daemon/preparation/plan.py': 'cf3c9c080303710e8613c85518864341269ea52c4673b848aa7cf3d7bc23fcfc', 'examples/playground/restricted/daemon/ui-qualification/owner_kernel.py': '4e09ab99cb9b5c66d05e1ba8a3f3847147ee7c18606268c1b16f0be85421878c', 'examples/playground/restricted/daemon/ui-qualification/owner_cgroup.py': '26659dc996b5729d111dc8928415fa5be6676500a18f10f39795b682d4aff6e0', 'examples/playground/restricted/daemon/ui-qualification/owner_mounts.py': '50f2503d9877bcabcf6cd388aa8cf7c8782e0ffafcf790c54c29d71b07780aa8', 'examples/playground/restricted/daemon/ui-qualification/owner_proc.py': '6770f5e9415ed09180f1fcbf46913cf55cbcde689df69293b0303a520d89a3ac'}

def observe(root,cid,uid):
    out=root/('observer-'+str(uid));result={'acceptance':False,'kind':'nonroot-readonly-operational-diagnostic','sourceHelperCommit':SOURCE,'observerPid':os.getpid(),'checks':{}}
    def check(name,fn):
        try:result['checks'][name]={'ok':True,'observed':fn()}
        except BaseException as e:result['checks'][name]={'ok':False,'error':type(e).__name__,'errno':getattr(e,'errno',None),'filename':getattr(e,'filename',None),'originalIo':getattr(e,'_ui_kernel_io',None),'message':str(e)[:2000]}
        return result['checks'][name].get('observed')
    try:
        sys.path.insert(0,str(root/'original-modules'))
        from engine import Engine
        import owner_kernel,owner_cgroup,owner_proc
        me=owner_kernel.fields(Path('/proc/self/status').read_text());result['hostStatus']=me
        assert me['Uid'].split()==[str(uid)]*4 and me['Gid'].split()==[str(uid)]*4 and os.getgroups()==[]
        assert all(int(me[k],16)==0 for k in ['CapInh','CapPrm','CapEff','CapBnd','CapAmb'])and me['NoNewPrivs']=='1'
        result['originalModulePins']={n:hashlib.sha256((root/'original-modules'/Path(n).name).read_bytes()).hexdigest()for n in PINS};assert result['originalModulePins']==PINS
        before=os.lstat('/var/run/docker.sock');result['socket']={'uid':before.st_uid,'gid':before.st_gid,'mode':oct(stat.S_IMODE(before.st_mode)),'device':before.st_dev,'inode':before.st_ino}
        assert stat.S_ISSOCK(before.st_mode)and before.st_uid==65534 and before.st_gid==1000 and stat.S_IMODE(before.st_mode)==0o660
        with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)as stream:
            stream.connect('/var/run/docker.sock');peer=struct.unpack('3i',stream.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));result['peer']={'pid':peer[0],'uid':peer[1],'gid':peer[2]};assert peer[1:]==(65534,65534)
        engine=Engine(65534,seconds=20);result['version']=engine.json('GET','/version');state=engine.json('GET','/containers/'+cid+'/json');pid=state['State']['Pid'];assert state['Id']==cid and state['State']['Running']and pid>1
        result.update(containerId=cid,guestPid=pid);deadline=time.monotonic()+10
        for name in ['status','stat','cgroup','mountinfo']:
            check('guest-'+name,lambda n=name:owner_kernel.read(f'/proc/{pid}/{n}',deadline,262144))
        check('guest-security',lambda:owner_kernel.security(owner_kernel.fields(owner_kernel.read(f'/proc/{pid}/status',deadline)),pid))
        links=check('guest-namespace-links',lambda:owner_kernel.namespace_links(pid,deadline))
        check('guest-network',lambda:owner_kernel.network(pid,deadline,links))
        check('original-complete-process-inventory',lambda:owner_proc.inventory(owner_kernel.read,deadline))
        membership=result['checks']['guest-cgroup'].get('observed')
        check('original-cgroup-mapping',lambda:owner_cgroup.mapping(owner_kernel.read,owner_kernel.numbers,membership,cid,deadline))
        end=engine.json('GET','/containers/'+cid+'/json');assert end['Id']==cid and end['State']['Running']and end['State']['Pid']==pid
        after=os.lstat('/var/run/docker.sock');assert (after.st_dev,after.st_ino,after.st_uid,after.st_gid,after.st_mode)==(before.st_dev,before.st_ino,before.st_uid,before.st_gid,before.st_mode)
        result['allRequestedReadsPassed']=all(x['ok']for x in result['checks'].values());result['status']='operational-baseline-observed-not-product-qualification'
    except BaseException as e:result['failure']={'type':type(e).__name__,'message':str(e)[:2000]}
    finally:(out/'RESULT.json').write_text(json.dumps(result,indent=2)+'\n')
    return 0 if not result.get('failure')else 1

def setup(root,source,cid):
    assert os.geteuid()==0 and len(cid)==64
    subprocess.run(['mount','--make-rprivate','/'],check=True)
    original=[l for l in Path('/proc/self/mountinfo').read_text().splitlines()if l.split(' - ')[0].split()[4]=='/sys/fs/cgroup'];assert len(original)==1
    options=original[0].split(' - ')[1].split()[2].split(',');assert set(options)<={'rw','nsdelegate','memory_recursiveprot'}and'rw'in options
    subprocess.run(['umount','/sys/fs/cgroup'],check=True);subprocess.run(['mount','-t','cgroup2','-o',','.join(['ro','nosuid','nodev','noexec']+[x for x in options if x!='rw']),'cgroup2','/sys/fs/cgroup'],check=True)
    runtime=root/'namespace-run';runtime.mkdir(mode=0o755);os.chown(runtime,65534,65534);subprocess.run(['mount','--bind',str(runtime),'/run'],check=True)
    modules=root/'original-modules';modules.mkdir(mode=0o755)
    for name,h in PINS.items():
        raw=(source/name).read_bytes();assert hashlib.sha256(raw).hexdigest()==h;target=modules/Path(name).name;target.write_bytes(raw);target.chmod(0o444)
    observer=root/'nonroot-observer.py';observer.write_bytes(Path(__file__).read_bytes());observer.chmod(0o444)
    broker_code=root/'broker-reviewed.py';broker_code.write_bytes(Path(__file__).with_name('broker.py').read_bytes());broker_code.chmod(0o444)
    def nobody():os.setgroups([]);os.setgid(65534);os.setuid(65534)
    broker=subprocess.Popen(['/usr/bin/python3','-I','-B',str(broker_code),str(root/'engine.sock')],preexec_fn=nobody,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    result={'acceptance':False,'scope':'One V6 trusted diagnostic guest, two actual nonroot readonly observers; no source evaluation/material/runtime qualification.','sourceHelperCommit':SOURCE,'originalModulePins':PINS,'observers':{}}
    try:
        for _ in range(100):
            if Path('/var/run/docker.sock').exists():break
            assert broker.poll()is None;time.sleep(.02)
        socket_path=Path('/var/run/docker.sock');before=socket_path.lstat();assert stat.S_ISSOCK(before.st_mode)and before.st_uid==65534
        result['socketBefore']={'uid':before.st_uid,'gid':before.st_gid,'mode':oct(stat.S_IMODE(before.st_mode)),'device':before.st_dev,'inode':before.st_ino}
        os.chown(socket_path,-1,1000);os.chmod(socket_path,0o660);after=socket_path.lstat();assert(after.st_dev,after.st_ino,after.st_uid,after.st_gid,stat.S_IMODE(after.st_mode))==(before.st_dev,before.st_ino,65534,1000,0o660)
        result['socketAfter']={'uid':after.st_uid,'gid':after.st_gid,'mode':oct(stat.S_IMODE(after.st_mode)),'device':after.st_dev,'inode':after.st_ino}
        for uid in [1000,65534]:
            work=root/('observer-'+str(uid));work.mkdir(mode=0o700);os.chown(work,uid,uid)
            argv=['/usr/bin/setpriv','--reuid='+str(uid),'--regid='+str(uid),'--clear-groups','--inh-caps=-all','--bounding-set=-all','--ambient-caps=-all','--no-new-privs','/usr/bin/python3','-I','-B',str(observer),'observe',str(root),cid,str(uid)]
            with (root/f'uid{uid}.stdout').open('xb')as stdout,(root/f'uid{uid}.stderr').open('xb')as stderr:
                try:p=subprocess.run(argv,stdout=stdout,stderr=stderr,stdin=subprocess.DEVNULL,timeout=20);row={'argv':argv,'exitCode':p.returncode,'timedOut':False}
                except subprocess.TimeoutExpired:row={'argv':argv,'exitCode':None,'timedOut':True}
            path=work/'RESULT.json'
            if path.exists():raw=path.read_bytes();(root/f'uid{uid}-RESULT.json').write_bytes(raw);row['result']=json.loads(raw)
            result['observers'][str(uid)]=row
        result['status']='two-nonroot-baselines-measured-see-individual-results'
    finally:
        broker.terminate()
        try:out,err=broker.communicate(timeout=5)
        except subprocess.TimeoutExpired:broker.kill();out,err=broker.communicate(timeout=2)
        (root/'broker.stdout').write_bytes(out);(root/'broker.stderr').write_bytes(err);result['brokerExitCode']=broker.returncode
        (root/'namespace-result.json').write_text(json.dumps(result,indent=2)+'\n')
    assert len(result['observers'])==2 and all(not x['timedOut']and'result'in x for x in result['observers'].values()),'each actual nonroot observation must complete'

if __name__=='__main__':
    if sys.argv[1]=='observe':sys.exit(observe(Path(sys.argv[2]),sys.argv[3],int(sys.argv[4])))
    setup(Path(sys.argv[1]),Path(sys.argv[2]),sys.argv[3])
