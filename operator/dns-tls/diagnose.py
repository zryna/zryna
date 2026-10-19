"""Inert transport-only diagnostic; no product, Docker, guest or approval action."""
import datetime,hashlib,importlib.util,json,os,pathlib,re,signal,stat,subprocess,sys,time
BASE='d73f616082749dab3d18e5be04c96650086e9be7'
TREE='4f1b259d4a9eea12dc76733ed6efc0f8838b6e2e'
SEED_SHA='1ef008c037febecfca7a050fc5303aed26aae06b9921c499e6b9573bc214a4ff'
MANIFEST_SHA='2ef8e0aa7a493b62a4c380a5c64a439e431e700871053fa2152468c718b805f1'
MATERIAL_SHA='7eb41c14dff5eacf521211c563e396d132f39f5b66ba98840a754824ef4649e7'
URL='https://api.github.com/repos/zryna/zryna/releases/388328382'

def need(value,message):
    if not value:raise ValueError(message)

def now():return datetime.datetime.now(datetime.timezone.utc).isoformat()
def pin(path):
    path=pathlib.Path(path);s=path.lstat();need(stat.S_ISREG(s.st_mode) and not path.is_symlink() and s.st_nlink==1,'ordinary selected file')
    b=path.read_bytes();a=path.stat();need((s.st_ino,s.st_size,s.st_mtime_ns,s.st_ctime_ns)==(a.st_ino,a.st_size,a.st_mtime_ns,a.st_ctime_ns),'stable selected file')
    return {'path':str(path),'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()}

def save(path,value):
    with pathlib.Path(path).open('x') as f:json.dump(value,f,indent=2);f.write('\n')
    pathlib.Path(path).chmod(0o400)

def copy(path,data,mode=0o400):
    with pathlib.Path(path).open('xb') as f:f.write(data)
    pathlib.Path(path).chmod(mode)

def load(path):
    spec=importlib.util.spec_from_file_location('selected_materials',path);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m

def safe_error(error):
    result={'class':type(error).__name__,'errno':getattr(error,'errno',None),'sslVerifyCode':getattr(error,'verify_code',None)}
    for k in ['errno','sslVerifyCode']:
        if not isinstance(result[k],(int,type(None))):result[k]=None
    status=re.fullmatch(r'whole GET HTTP ([1-5][0-9]{2})',str(error))
    if status:result['httpStatus']=int(status[1])
    return result

def resolv():
    p=pathlib.Path('/etc/resolv.conf');s=p.lstat();resolved=p.resolve(strict=True);t=resolved.stat()
    need(stat.S_ISREG(t.st_mode) and t.st_size<=65536,'bounded original resolver config')
    data=resolved.read_bytes();need(pin(resolved)['sha256']==hashlib.sha256(data).hexdigest(),'exact original resolver snapshot')
    return {'path':str(p),'lstat':{'mode':s.st_mode,'uid':s.st_uid,'gid':s.st_gid,'inode':s.st_ino,'device':s.st_dev},'readlink':os.readlink(p) if p.is_symlink() else None,'resolved':str(resolved),'target':{'mode':t.st_mode,'uid':t.st_uid,'gid':t.st_gid,'nlink':t.st_nlink},'config':pin(resolved)},data

def metadata():
    import ssl
    context=ssl.create_default_context();need(context.verify_mode==ssl.CERT_REQUIRED and context.check_hostname,'original TLS verification')
    try:dns,_=resolv()
    except OSError as error:dns={'error':safe_error(error),'resolvLink':os.readlink('/etc/resolv.conf') if pathlib.Path('/etc/resolv.conf').is_symlink() else None}
    return {'dns':dns,'defaultSslPaths':ssl.get_default_verify_paths()._asdict(),'sslVerifyMode':int(context.verify_mode),'sslCheckHostname':context.check_hostname,'caStats':context.cert_store_stats(),'proxyEnvironmentPresence':{k:k in os.environ for k in ['HTTP_PROXY','HTTPS_PROXY','ALL_PROXY','NO_PROXY','http_proxy','https_proxy','all_proxy','no_proxy']}}

def probe(root,phase):
    import ssl,zlib
    need(os.geteuid()==0 and sys.version_info[:3]==(3,12,11) and zlib.ZLIB_RUNTIME_VERSION=='1.3.1','actual pinned root interpreter')
    record={'phase':phase,'startedAt':now(),'metadata':metadata(),'acceptance':False}
    m=load(root/'materials.diagnostic.py');m.safe_error=safe_error
    try:
        body=m.request(URL,time.monotonic()+9,2*1024**2);copy(root/(phase+'.response.json'),body)
        value=json.loads(body);need(value['id']==388328382 and value['tag_name']=='v0.2.3' and value['immutable'] is True and value['draft'] is False,'original selected legacy metadata')
        record.update(status='actual-original-metadata-get-success',response=pin(root/(phase+'.response.json')))
    except BaseException as error:
        detail=safe_error(error)
        if isinstance(error,RuntimeError):
            try:detail=json.loads(str(error))
            except ValueError:pass
        record.update(status='actual-original-metadata-get-failure',error=detail)
    record['finishedAt']=now();save(root/(phase+'.json'),record)

def run_probe(root,phase):
    argv=[sys.executable,'-I','-B',str(root/'diagnose.py'),'probe',str(root),phase];started=now()
    with (root/(phase+'.stdout')).open('xb') as out,(root/(phase+'.stderr')).open('xb') as err:
        child=subprocess.Popen(argv,stdin=subprocess.DEVNULL,stdout=out,stderr=err)
        try:code=child.wait(timeout=10);timed_out=False
        except subprocess.TimeoutExpired:child.kill();code=child.wait(timeout=2);timed_out=True
    save(root/(phase+'.owner.json'),{'argv':argv,'pid':child.pid,'startedAt':started,'finishedAt':now(),'timedOut':timed_out,'exitCode':code,'reaped':True,'stdout':pin(root/(phase+'.stdout')),'stderr':pin(root/(phase+'.stderr'))})
    return json.loads((root/(phase+'.json')).read_bytes()) if (root/(phase+'.json')).exists() else {'phase':phase,'status':'probe-result-unavailable','timedOut':timed_out,'exitCode':code}

def namespace(root):
    need(os.geteuid()==0,'trusted namespace root');subprocess.run(['/usr/bin/mount','--make-rprivate','/'],check=True,timeout=5)
    copy(root/'NAMESPACE_BEFORE.mountinfo',pathlib.Path('/proc/self/mountinfo').read_bytes())
    dns,data=resolv();copy(root/'DNS.original.conf',data);save(root/'DNS_ORIGINAL.json',dns)
    results=[run_probe(root,'baseline')]
    runtime=root/'namespace-run';runtime.mkdir(mode=0o755);os.chown(runtime,65534,65534)
    subprocess.run(['/usr/bin/mount','--bind',str(runtime),'/run'],check=True,timeout=5)
    results.append(run_probe(root,'overlay'));restoration={'status':'not-applicable-original-target-outside-run'}
    target=pathlib.Path(dns['resolved']);rel=target.relative_to('/run') if target.is_relative_to('/run') else None
    if rel is not None:
        need(rel.parts and '..' not in rel.parts and not target.exists(),'only hidden original ordinary DNS target')
        need(dns['lstat']['uid']==0 and 0<=dns['target']['uid']<1000 and dns['target']['nlink']==1 and not dns['target']['mode']&0o022,'original root-configured system resolver file')
        current=pathlib.Path('/run')
        for component in rel.parts[:-1]:
            current=current/component;current.mkdir(mode=0o755);need(not current.is_symlink() and current.stat().st_uid==0,'fresh private resolver directories')
        copy(target,data,stat.S_IMODE(dns['target']['mode']));need(target.read_bytes()==data,'restore exact provided resolver bytes only')
        restoration={'status':'original-provided-config-byte-exactly-restored','target':pin(target),'original':dns['config']}
        results.append(run_probe(root,'restored'))
    copy(root/'NAMESPACE_AFTER.mountinfo',pathlib.Path('/proc/self/mountinfo').read_bytes())
    save(root/'OBSERVATIONS.json',{'status':'transport-diagnostic-observations-complete','results':results,'restoration':restoration,'acceptance':False,'meaning':'Individual baseline/overlay/restored outcomes retained; no runtime/CI/qualification verdict or automatic retry.'})

def bootstrap(operator,baseline,expected):
    need(os.geteuid()==0 and re.fullmatch('[a-f0-9]{64}',expected),'root authenticated bootstrap')
    env={'PATH':'/usr/bin:/bin','HOME':'/nonexistent','LC_ALL':'C','GIT_CONFIG_NOSYSTEM':'1','GIT_CONFIG_GLOBAL':'/dev/null','GIT_NO_REPLACE_OBJECTS':'1'}
    def git(repo,*args):return subprocess.check_output(['/usr/bin/git','-c','safe.directory='+str(repo),'--no-replace-objects','-C',str(repo),*args],env=env,timeout=10)
    head=os.environ['GITHUB_SHA'];need(re.fullmatch('[a-f0-9]{40}',head) and head==os.environ['SELECTED_OPERATOR'],'actual diagnostic operator')
    need(git(operator,'rev-parse','HEAD').decode().strip()==head and git(baseline,'rev-parse','HEAD').decode().strip()==BASE and git(baseline,'rev-parse','HEAD^{tree}').decode().strip()==TREE,'exact diagnostic and seed carrier Git selections')
    own=git(operator,'show',head+':operator/dns-tls/diagnose.py');need(hashlib.sha256(own).hexdigest()==expected,'independently selected diagnostic source')
    run,attempt=os.environ['GITHUB_RUN_ID'],os.environ['GITHUB_RUN_ATTEMPT'];need(re.fullmatch('[1-9][0-9]*',run) and attempt=='1','one actual diagnostic attempt')
    root=pathlib.Path('/workspace/scratch/M6_410_DNS_TLS_DIAGNOSTIC_'+run+'_'+attempt)
    for p in [root.parent.parent,root.parent]:
        p.mkdir(mode=0o755,exist_ok=True);s=p.lstat();need(stat.S_ISDIR(s.st_mode) and s.st_uid==0 and not s.st_mode&0o022 and not p.is_symlink(),'root workspace')
    root.mkdir(mode=0o700);copy(root/'diagnose.py',own)
    started=time.monotonic();save(root/'INTENT.json',{'startedAt':now(),'operator':head,'baselineOperator':BASE,'sourceProductExecuted':False,'run':run,'attempt':attempt,'limits':{'wholeSeconds':120,'probeSeconds':10,'maximumRequests':3},'scope':'Dedicated mount namespace transport diagnosis only. No Engine, Docker, product, guest or approval.'})
    before=pathlib.Path('/proc/self/mountinfo').read_bytes();copy(root/'PARENT_BEFORE.mountinfo',before)
    seed=git(baseline,'show',BASE+':operator/remote-runtime/payload/runtime-seed.tar.gz');need(len(seed)==30859757 and hashlib.sha256(seed).hexdigest()==SEED_SHA,'whole approved compatible seed')
    module=git(baseline,'show',BASE+':operator/remote-runtime/modules/materials.py');need(hashlib.sha256(module).hexdigest()==MATERIAL_SHA,'exact original material transport module')
    copy(root/'seed.tar.gz',seed);copy(root/'materials.original.py',module)
    old=b"        except (OSError, http.client.HTTPException): raise RuntimeError('bounded HTTPS acquisition failed') from None"
    new=b"        except OSError as error: raise RuntimeError(json.dumps(safe_error(error))) from None\n        except http.client.HTTPException as error: raise RuntimeError(json.dumps(safe_error(error))) from None"
    need(module.count(old)==1,'exact two diagnostic exception clauses only');copy(root/'materials.diagnostic.py',module.replace(old,new).replace(b'for hop in range(5):',b'for hop in range(1):'))
    m=load(root/'materials.original.py');selected=m.unpack_seed(str(root/'seed.tar.gz'),str(root/'seed'),SEED_SHA,MANIFEST_SHA)
    python=selected/'host-python/bin/python3.12';need(pin(python)['sha256']=='f944c0c6ee99cda936c67316992eb8c2105a62a7052c91347b16dcdeefab7278','actual approved interpreter')
    save(root/'PINNED_INPUTS.json',{'seedArchive':pin(root/'seed.tar.gz'),'seedManifest':pin(selected/'MANIFEST.json'),'interpreter':pin(python),'originalMaterialModule':pin(root/'materials.original.py'),'diagnosticMaterialModule':pin(root/'materials.diagnostic.py')})
    copy(root/'SEED_MANIFEST.json',(selected/'MANIFEST.json').read_bytes())
    environment=dict(os.environ);environment.update(LD_LIBRARY_PATH=str(selected/'compression/lib'),PYTHONDONTWRITEBYTECODE='1')
    argv=['/usr/bin/unshare','--mount','--',str(python),'-I','-B',str(root/'diagnose.py'),'namespace',str(root)]
    with (root/'namespace.stdout').open('xb') as out,(root/'namespace.stderr').open('xb') as err:
        child=subprocess.Popen(argv,env=environment,stdin=subprocess.DEVNULL,stdout=out,stderr=err,start_new_session=True)
        epoch=pathlib.Path('/proc/'+str(child.pid)+'/stat').read_text().rsplit(')',1)[1].split()[19]
        timed_out=False
        try:code=child.wait(timeout=max(1,120-(time.monotonic()-started)))
        except subprocess.TimeoutExpired:
            need(pathlib.Path('/proc/'+str(child.pid)+'/stat').read_text().rsplit(')',1)[1].split()[19]==epoch,'same namespace owner');os.killpg(child.pid,signal.SIGKILL);code=child.wait(timeout=5);timed_out=True
    after=pathlib.Path('/proc/self/mountinfo').read_bytes();copy(root/'PARENT_AFTER.mountinfo',after)
    relevant=lambda b:[v for v in b.decode().splitlines() if v.split(' - ')[0].split()[4] in ['/run','/sys/fs/cgroup']]
    need(relevant(before)==relevant(after),'parent run/cgroup mount metadata unchanged')
    save(root/'RESULT.json',{'status':'transport-diagnostic-owner-complete','exitCode':code,'reaped':True,'timedOut':timed_out,'pid':child.pid,'epoch':epoch,'parentRunAndCgroupMountMetadataUnchanged':True,'wholeSeconds':time.monotonic()-started,'observations':pin(root/'OBSERVATIONS.json') if (root/'OBSERVATIONS.json').exists() else None,'acceptance':False})
    need(code==0 and not timed_out,'diagnostic observation owner failed; preserve raw evidence')

if __name__=='__main__':
    if sys.argv[1]=='bootstrap':bootstrap(pathlib.Path(sys.argv[2]),pathlib.Path(sys.argv[3]),sys.argv[4])
    elif sys.argv[1]=='namespace':namespace(pathlib.Path(sys.argv[2]))
    elif sys.argv[1]=='probe':probe(pathlib.Path(sys.argv[2]),sys.argv[3])
    else:raise ValueError('closed diagnostic mode')
