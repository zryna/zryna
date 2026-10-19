"""One source-authenticated, unapproved preparation phase; activated only by host owner."""
import hashlib,importlib.util,json,os,pathlib,re,stat,subprocess,sys
sys.dont_write_bytecode=True


def require(value,reason):
    if not value:raise ValueError(reason)


def pairs(items):
    result={}
    for k,v in items:require(k not in result,'duplicate JSON field');result[k]=v
    return result


def read(path):return json.loads(pathlib.Path(path).read_bytes(),object_pairs_hook=pairs,parse_constant=lambda _:require(False,'nonfinite JSON'))


def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module


def bootstrap(path,expected):
    p=pathlib.Path(path)
    for a in [*reversed(p.parents),p]:
        s=a.lstat();require(s.st_uid==0 and not s.st_mode&0o022 and not stat.S_ISLNK(s.st_mode),'root-owned no-link manifest ancestry')
    b=p.read_bytes();require(0<len(b)<=65536 and hashlib.sha256(b).hexdigest()==expected,'manifest selected identity')
    m=read(p);cp=m['files']['common'];c=pathlib.Path(cp['path'])
    for a in [*reversed(c.parents),c]:
        s=a.lstat();require(s.st_uid==0 and not s.st_mode&0o022 and not stat.S_ISLNK(s.st_mode),'root-owned common module ancestry')
    b=c.read_bytes();require(len(b)==cp['bytes'] and hashlib.sha256(b).hexdigest()==cp['sha256'],'common wholepin')
    common=load('remote_prepare_common',c);value,_=common.read_manifest(str(p),expected,c.parent);return common,value


def main(manifest_path):
    require(os.geteuid()==0 and len(sys.argv)==2,'one manifest argument and trusted operator UID0')
    common,m=bootstrap(manifest_path,os.environ['M6_REMOTE_MANIFEST_SHA256']);pin=common.check_pin
    require(m['hostRoot']==os.environ['M6_REMOTE_HOST_ROOT'],'owned host root')
    require(all(os.environ.get(k)==v for k,v in [('GITHUB_RUN_ID',m['run']['id']),('GITHUB_RUN_ATTEMPT',m['run']['attempt']),('GITHUB_SHA',m['approval']['operatorCommit'])]),'actual hosting selectors')
    c=read(pin(m['files']['context']));common.exact(c,['format','version','source','frozenMain','candidateReview','seed','prepare','runtime'])
    require(c['format']=='zryna.m6-remote-runtime-context.v1' and c['version']==1 and c['source']==m['source'],'context/source identity')
    p=c['prepare'];common.exact(p,['root','workRoot','materialsModule','plansModule','compositionModule','policyModule','runtimeBinder','runtimeBinderSupport','runtimeRunner','probeModule','probeExport','probeConfigBlob','probeSource'])
    common.exact(c['seed'],['directory','manifest']);pin(c['seed']['manifest'])
    require(c['seed']['manifest']['path']==str(pathlib.Path(c['seed']['directory'])/'MANIFEST.json'),'selected seed manifest path')
    module_names=['materialsModule','plansModule','compositionModule','policyModule','runtimeBinder','runtimeBinderSupport','runtimeRunner','probeModule']
    require(pathlib.Path(p['runtimeBinderSupport']['path'])==pathlib.Path(p['runtimeBinder']['path']).parent/'runtime_support.py' and pathlib.Path(p['runtimeRunner']['path'])==pathlib.Path(p['runtimeBinder']['path']).parent/'run-cohorts.py','selected binder sibling executable closure')
    for name in [*module_names,'probeExport','probeConfigBlob','probeSource']:pin(p[name])
    runtime_support=load('selected_runtime_support',p['runtimeBinderSupport']['path'])
    require(runtime_support.CALLER_UID==1000,'ROOT must select actual preflight-supported fixed nonroot caller UID before activation')
    require(m['approval']['phase']=='qualification' and m['approval']['source']=={k:c['source'][k] for k in ['commit','tree']},'candidate-only source-bound phase')
    require(m['approval']['run']=={k:int(v) for k,v in m['run'].items()},'actual approval run selectors')
    require(m['approval']['prepared']=='PREPARATION.json' and m['approval']['approved']==[{'candidate':'approval.candidate.json','output':'approval.json'},{'candidate':'ui-approval.candidate.json','output':'ui-approval.json'}],'original candidate approval paths')
    require(p['root']==m['approval']['root'] and p['workRoot']==m['hostRoot']+'/preparation-work','fresh fixed work and approval roots')
    root=pathlib.Path(p['root']);work=pathlib.Path(p['workRoot'])
    require(root.is_relative_to(m['hostRoot']) and root!=work and not root.is_relative_to(work) and not work.is_relative_to(root),'separate small approval and large work roots')
    for target in [root,work]:common.trusted(str(target.parent),True);target.mkdir(mode=0o700)
    require(c['runtime']['operatorRoot']==str(root/'runtime-operators'),'bound helpers inside complete approval inventory')
    probe=c['runtime']['dependencies']['probe'];require(probe['directory']==str(root/'probe') and probe['imageMetadata']=={'path':str(root/'probe/IMAGE.json')} and probe['configBlob']=={'path':str(root/'probe/EXACT_IMAGE_CONFIG_BLOB.json')} and probe['source']==p['probeSource'],'fresh trusted probe result paths')
    require(c['runtime']['dependencies']['inputObservations']=={'path':str(root/'INPUT_OBSERVATIONS.json')},'own actual observations path')
    source=c['source'];repo=pathlib.Path(source['path']);identity={k:source[k] for k in ['commit','tree']}
    def git(*args):return subprocess.run(['git','--no-replace-objects',*args],cwd=repo,stdin=subprocess.DEVNULL,capture_output=True,timeout=30,check=True).stdout
    def clean_source():
        require(git('rev-parse','HEAD').decode().strip()==source['commit'] and git('rev-parse','HEAD^{tree}').decode().strip()==source['tree'],'actual source commit/tree')
        require(git('status','--porcelain=v1','--untracked-files=all')==b'','actual untouched clean source checkout')
    clean_source();frozen=read(pin(c['frozenMain']));require(all(frozen[k]==source[k] for k in ['commit','tree']),'actual frozen source record')
    candidate=read(pin(c['candidateReview']));require(candidate['status']=='official-two-replica-candidate-acquired-and-reassembled' and candidate['source']==identity,'actual reviewed two-replica source candidate')
    compiler=common.file_pin(candidate['compilerPath']);require(compiler['bytes']==candidate['compiler']['bytes'] and compiler['sha256']==candidate['compiler']['sha256'] and compiler['bytes']<=134217728,'actual compiler binary whole bytes')
    def save(path,value):
        b=(json.dumps(value,indent=2,sort_keys=True)+'\n').encode()
        with pathlib.Path(path).open('xb') as stream:stream.write(b)
        pathlib.Path(path).chmod(0o400)
    def copy(src,dst):
        with pathlib.Path(src).open('rb') as a,pathlib.Path(dst).open('xb') as b:
            while chunk:=a.read(65536):b.write(chunk)
        pathlib.Path(dst).chmod(0o400)
    try:
        save(root/'INTENT.json',{'source':identity,'candidate':c['candidateReview'],'compiler':compiler,'seed':c['seed'],'scope':'Fresh source preparation only; no approval, guest execution, publication or acceptance.'})
        copy(compiler['path'],root/'compiler.bin');(root/'compiler.bin').chmod(0o500);require(common.file_pin(root/'compiler.bin')['sha256']==compiler['sha256'],'retained actual candidate')
        materials_module=load('remote_materials',p['materialsModule']['path']);materials=materials_module.build_materials(str(repo),c['seed']['directory'],str(work/'material-build'))
        save(root/'MATERIAL_BUILD.json',materials)
        plans_module=load('remote_plans',p['plansModule']['path']);plans=plans_module.build_runtime_plans(source,candidate,c['seed']['directory'],materials,str(work/'runtime-plans'),m['files']['node']['path'])
        save(root/'RUNTIME_PLAN_SELECTION.json',plans)
        deps=c['runtime']['dependencies'];require(deps['toolsClosure']=={'path':str(root/'OWNER_TOOLS.json')},'generated original UI owner-tools closure')
        seed=pathlib.Path(c['seed']['directory']);historic=read(seed/'metadata/host-python-policy.current.json')
        python=common.file_pin(seed/'host-python/bin/python3.12')
        require(python==m['files']['python'],'selected root interpreter is exact approved seed host Python')
        tools={'python':{**python,'ownerUid':0},'files':[],'runtime':{'version':'3.12','files':[]}}
        names=['engine.py','plan.py','owner_main.py','owner_relay.py','owner_lifecycle.py','owner_attach.py','owner_config.py','owner_identity.py','owner_audit.py','owner_kernel.py','owner_proc.py','owner_cgroup.py','owner_mounts.py']
        for name in names:
            base='preparation' if name in ['engine.py','plan.py'] else 'ui-qualification'
            path=repo/'examples/playground/restricted/daemon'/base/name
            tools['files'].append({'name':name,**common.file_pin(path),'ownerUid':0})
        for row in historic['runtime']['files']:
            path=seed/'host-python'/row['name'];common.trusted(str(path));b=path.read_bytes()
            require(len(b)==row['bytes'] and hashlib.sha256(b).hexdigest()==row['sha256'],'whole approved seed Python runtime leaf')
            tools['runtime']['files'].append({**row,'path':str(path),'ownerUid':0})
        save(root/'OWNER_TOOLS.json',tools)
        require(deps['browser']=={'path':materials['evaluator']['sourceDirectory']+'/browser/chrome-linux64/chrome'} and deps['playwrightEntry']=={'path':materials['evaluator']['sourceDirectory']+'/playwright/index.mjs'},'actual reconstructed material browser/playwright paths')
        sys.path.insert(0,str(repo/'examples/playground/restricted/daemon/preparation'))
        from engine import Engine
        from install import import_runtime,populate_materials,volume_identity
        from ui_image import import_image
        images={};volumes={}
        for role in ['compiler','evaluator','ui']:
            selected=plans['plans'][role];operation=import_image if role=='ui' else import_runtime
            result=operation(Engine(65534,seconds=300),selected['path'],selected['planBytes'],selected['planSha256']);save(root/(role+'-image.json'),result)
            require(result['failure'] is None and re.fullmatch('sha256:[a-f0-9]{64}',result['image'] or ''),'original image import succeeded')
            images[role]=result;save(root/(role+'-IMAGE_INSPECT.json'),Engine(65534,seconds=30).json('GET','/images/'+result['image']+'/json'))
        for role in ['compiler','evaluator']:
            row=materials[role];selected=row['plan'];result=populate_materials(Engine(65534,seconds=300),images[role]['image'],selected['path'],selected['bytes'],selected['sha256'],row['manifest']['sha256'])
            save(root/(role+'-volume.json'),result);require(result['failure'] is None and result['holderRemoved'] is True,'original never-started volume population succeeded')
            volumes[role]=result
        engine=Engine(65534,seconds=60);containers=engine.json('GET','/containers/json?all=1');save(root/'CONTAINERS_AFTER_PREPARATION.json',containers)
        require(type(containers) is list,'bounded actual container observations')
        for role,v in volumes.items():
            actual=engine.json('GET','/volumes/'+v['name']);save(root/(role+'-VOLUME_INSPECT.json'),actual)
            volume_identity(actual,v['name'],v['nonce'],v['root_sha256'],v['created_at'])
            require(all(x.get('Id')!=v['holderId'] and all(t.get('Name')!=v['name'] for t in x.get('Mounts',[])) for x in containers),'no remaining holder or mounted aliases of prepared material volume')
        probe_module=load('remote_probe',p['probeModule']['path']);probe_result=probe_module.prepare_probe(c,Engine,pin,save)
        selection={'source':identity,'compiler':{'bytes':compiler['bytes'],'sha256':compiler['sha256']},'candidateReview':c['candidateReview'],'frozenMain':c['frozenMain']}
        actual={'status':'prepared-unapproved','source':source,'selection':selection,'images':images,'materials':volumes,'materialBuild':materials,'plans':plans,'probe':probe_result,'workRoot':str(work)}
        save(root/'RESULT.json',actual)
        save(root/'INPUT_OBSERVATIONS.json',{'status':'actual-runtime-preparation-observations-not-approved','source':source,'selection':selection,'images':images,'materials':volumes,'probe':probe_result,'scope':'Operator-owned fresh API/source observations, not independent review or guest mounted full material audit. No guest has started; original runtime audit remains mandatory.'})
        prefix=root/'COMPOSITION_COMMAND';argv=[m['files']['node']['path'],p['compositionModule']['path'],str(manifest_path)]
        save(str(prefix)+'.intent.json',{'argv':argv,'cwd':str(repo),'timeoutSeconds':60})
        with pathlib.Path(str(prefix)+'.stdout').open('xb') as out,pathlib.Path(str(prefix)+'.stderr').open('xb') as err:
            completed=subprocess.run(argv,cwd=repo,stdin=subprocess.DEVNULL,stdout=out,stderr=err,timeout=60)
        save(str(prefix)+'.json',{'returncode':completed.returncode});require(completed.returncode==0,'original candidate composition command succeeded')
        copy(m['files']['context']['path'],root/'REMOTE_RUNTIME_CONTEXT.json')
        view_helper=pathlib.Path(p['runtimeBinder']['path']).parent/'caller-view.py'
        view_helper_pin=common.file_pin(str(view_helper))
        require(view_helper_pin=={'path':str(view_helper),'bytes':9426,'sha256':'c3b2dc8351a1e9828049199dceb4b1e010e3f3af0b20d586b3660151923e3a63'},'exact root-owned nofollow caller view helper sibling')
        view_module=load('selected_caller_view',view_helper);view_module.prepare_view(c,m,materials,plans,runtime_support.CALLER_UID)
        require(common.file_pin(str(view_helper))==view_helper_pin,'caller view helper source unchanged after use')
        binder=load('remote_runtime_binder',p['runtimeBinder']['path']);binder.bind_runtime(str(root/'REMOTE_RUNTIME_CONTEXT.json'),str(manifest_path))
        # Host-native ownership smoke uses actual bound client before any protected PREP inventory.
        smoke_helper=pathlib.Path(p['runtimeBinder']['path']).parent/'trusted-owner-preflight.mjs'
        smoke_pin=common.file_pin(str(smoke_helper));require(smoke_pin=={'path':str(smoke_helper),'bytes':5784,'sha256':'319a64544ad0a69d78a370e5592d4da558cf676db77f7e3b6e6db015df765a11'},'exact root-owned nofollow trusted owner smoke sibling')
        provider=common.file_pin('/usr/bin/python3.12');provider_stat=pathlib.Path(provider['path']).lstat();require(provider_stat.st_nlink==1,'ordinary single-link system Python provider')
        copy(provider['path'],root/'trusted-owner-provider-python3.12')
        smoke_work=work/'trusted-owner-smoke';smoke_work.mkdir(mode=0o700);os.chown(smoke_work,1000,1000)
        smoke_argv=['/usr/bin/setpriv','--reuid=1000','--regid=1000','--clear-groups','--inh-caps=-all','--bounding-set=-all','--ambient-caps=-all','--no-new-privs',m['files']['node']['path'],str(smoke_helper),str(repo),str(root/'caller-view/runtime-operators/trusted-owner/trusted-ui-owner-client.mjs'),str(smoke_work),json.dumps(provider,separators=(',',':'))]
        smoke_env={'PATH':str(pathlib.Path(m['files']['node']['path']).parent)+':/usr/sbin:/usr/bin:/sbin:/bin','HOME':'/nonexistent','LANG':'C.UTF-8','LC_ALL':'C.UTF-8','TZ':'UTC','PYTHONDONTWRITEBYTECODE':'1','LD_LIBRARY_PATH':c['seed']['directory']+'/compression/lib','GIT_CONFIG_COUNT':'1','GIT_CONFIG_KEY_0':'safe.directory','GIT_CONFIG_VALUE_0':'/workspace/zryna','GIT_CONFIG_NOSYSTEM':'1','GIT_CONFIG_GLOBAL':'/dev/null','GIT_NO_REPLACE_OBJECTS':'1'}
        retained=[];retained_bytes=0;smoke_destination=root/'trusted-owner-preflight';smoke_destination.mkdir(mode=0o700)
        try:
            smoke_command=common.run_owned(smoke_argv,str(repo),60,root/'TRUSTED_OWNER_SMOKE',smoke_env)
        finally:
            # Never infer reap from finally entry: only the actual trusted command journal permits caller body reads.
            smoke_journal_path=root/'TRUSTED_OWNER_SMOKE.json'
            smoke_journal=read(common.check_pin(common.file_pin(smoke_journal_path))) if smoke_journal_path.exists() else {}
            smoke_reaped=smoke_journal.get('reaped') is True and type(smoke_journal.get('returncode')) is int and 'reapError' not in smoke_journal
            if not smoke_reaped:
                save(root/'TRUSTED_OWNER_SMOKE_RETENTION.json',{'files':[],'bytes':0,'notRead':True,'reason':'actual-owned-smoke-command-reap-unconfirmed','commandJournal':common.file_pin(smoke_journal_path) if smoke_journal_path.exists() else None,'scope':'Caller evidence bodies were not read; missing/reapError/unknown exit remains a preparation failure.'})
                require(False,'actual owned smoke command reap/returncode required before reading caller evidence')
            for item in sorted(smoke_work.rglob('*')):
                before=item.lstat();require(not stat.S_ISLNK(before.st_mode),'no linked smoke evidence')
                if stat.S_ISDIR(before.st_mode):continue
                require(stat.S_ISREG(before.st_mode) and before.st_uid==1000 and before.st_nlink==1 and not before.st_mode&0o022 and before.st_size<=2097152,'bounded single-link actual caller smoke output')
                raw=item.read_bytes();after=item.lstat();require(all(getattr(before,k)==getattr(after,k)for k in ['st_dev','st_ino','st_mode','st_uid','st_gid','st_nlink','st_size','st_mtime_ns','st_ctime_ns']),'stable complete smoke output')
                retained_bytes+=len(raw);require(len(retained)<64 and retained_bytes<=16777216,'bounded full smoke retention')
                destination=smoke_destination/item.relative_to(smoke_work);destination.parent.mkdir(parents=True,exist_ok=True,mode=0o700)
                with destination.open('xb')as output:output.write(raw)
                destination.chmod(0o400);retained.append({'originalPath':str(item),'originalUid':before.st_uid,'originalGid':before.st_gid,'originalMode':stat.S_IMODE(before.st_mode),'retained':{'path':str(destination),'bytes':len(raw),'sha256':hashlib.sha256(raw).hexdigest()}})
            save(root/'TRUSTED_OWNER_SMOKE_RETENTION.json',{'files':retained,'bytes':retained_bytes,'scope':'Whole actual caller outputs, including partial failures; copying to root0/0400 grants no success or cleanup verdict.'})
        smoke_result=read(smoke_destination/'RESULT.json');require(smoke_result['status']=='original-system-python-owner-native-feature-and-true-cleanup-smoke-passed-not-runtime-qualification' and smoke_result['provider']==provider and smoke_result['cleanupConfirmed'] is True,'actual original owner/native feature smoke required')
        require(smoke_result['nativeFeatures']['pidfdOpenSelf'] is True and smoke_result['nativeFeatures']['pidfdSignalZero'] is True and smoke_result['nativeFeatures']['nativeChildSubreaper']==1,'actual supported pidfd/signal/subreaper features')
        owner_record=read(smoke_destination/'proof/true.json')
        for dependency in smoke_result['dependencies']:
            selected={k:dependency[k]for k in ['path','bytes','sha256']}
            if pathlib.Path(selected['path']).is_relative_to(smoke_work):
                require(selected['path']==owner_record['pins']['helperCapture']['captured']['path'] and selected['bytes']==owner_record['pins']['helper']['bytes'] and selected['sha256']==owner_record['pins']['helper']['sha256'],'only original byte-exact caller helper capture is nonroot-owned')
            else:require(common.file_pin(selected['path'])==selected,'root-owned ordinary whole actual system runtime dependency')
        require(common.file_pin(provider['path'])==provider and common.file_pin(str(smoke_helper))==smoke_pin,'provider and smoke code unchanged')
        save(root/'TRUSTED_OWNER_PROVIDER.json',{'provider':provider,'retainedBinary':common.file_pin(root/'trusted-owner-provider-python3.12'),'originalMode':stat.S_IMODE(provider_stat.st_mode),'actualNativeFeatures':smoke_result['nativeFeatures'],'actualPackageVersions':smoke_result['packageVersions'],'dependencies':smoke_result['dependencies'],'ownerReady':smoke_result['ready'],'complete':smoke_result['complete'],'scope':'Trusted owner host runtime observation only; root PBS, guest materials and source execution unchanged.'})
        bound_path=root/'runtime-operators/BOUND_MANIFEST.json';bound=read(bound_path)
        bound['inputs'].extend([smoke_pin,common.file_pin(root/'TRUSTED_OWNER_PROVIDER.json'),common.file_pin(root/'TRUSTED_OWNER_SMOKE_RETENTION.json')])
        # BOUND_MANIFEST was created by the binder in this unapproved preparation only; replace before the first protected inventory.
        bound_path.unlink();save(bound_path,bound)
        clean_source();pin(m['files']['context']);pin(c['frozenMain']);pin(c['candidateReview'])
        for name in module_names:pin(p[name])
        require(common.file_pin(candidate['compilerPath'])==compiler,'actual candidate retained unchanged')
        files=[];total=0
        for path in sorted(root.rglob('*')):
            common.trusted(str(path),path.is_dir())
            if path.is_dir():continue
            s=path.lstat();require(stat.S_ISREG(s.st_mode) and s.st_nlink==1 and s.st_size<=134217728,'bounded complete ordinary preparation member')
            b=path.read_bytes();after=path.stat();require((s.st_ino,s.st_dev,s.st_size,s.st_mtime_ns,s.st_ctime_ns)==(after.st_ino,after.st_dev,after.st_size,after.st_mtime_ns,after.st_ctime_ns),'stable whole preparation member')
            relative=str(path.relative_to(root));require(relative not in ['PREPARATION.json','approval.json','ui-approval.json'],'no existing approval or inventory')
            files.append({'path':relative,'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()});total+=len(b)
        require(2<=len(files)<=2048 and total<=536870912,'complete original gate inventory bounds')
        prepared={'format':'zryna.m6-remote-preparation.v1','version':1,'phase':'qualification','source':identity,'operatorCommit':m['approval']['operatorCommit'],'run':m['approval']['run'],'files':files}
        save(root/'PREPARATION.json',prepared)
        save(pathlib.Path(m['hostRoot'])/'REMOTE_PREPARED.json',{'status':'prepared-unapproved','preparation':common.file_pin(root/'PREPARATION.json'),'files':len(files),'bytes':total,'source':identity,'scope':'Candidate-only. Separate protected-tag gate and original independent reviews remain required; no guest execution.'})
    except BaseException as error:
        if not (root/'FAILURE.json').exists():save(root/'FAILURE.json',{'status':'preparation-failed','source':identity,'type':type(error).__name__,'message':str(error)[:1024],'cleanup':'Exact original returned image/volume/holder identities retained. No uncertain object force-removal or approval; host owner retains isolated daemon cleanup.'})
        raise


if __name__=='__main__':main(pathlib.Path(sys.argv[1]))
