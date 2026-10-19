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
    c=read(pin(m['files']['context']));common.exact(c,['format','version','source','frozenMain','candidateReview','seed','prepare','publication','runtime','documentation'])
    require(c['format']=='zryna.m6-remote-public-context.v1' and c['version']==1 and c['source']==m['source'],'context/source identity')
    p=c['prepare'];common.exact(p,['root','workRoot','materialsModule','plansModule','compositionModule','policyModule','publicPort','hostFileProvider'])
    common.exact(c['seed'],['directory','manifest']);pin(c['seed']['manifest'])
    require(c['seed']['manifest']['path']==str(pathlib.Path(c['seed']['directory'])/'MANIFEST.json'),'selected seed manifest path')
    module_names=['materialsModule','plansModule','compositionModule','policyModule','publicPort','hostFileProvider']
    for name in module_names:pin(p[name])
    require(m['approval']['phase']=='public' and m['approval']['source']=={k:c['source'][k] for k in ['commit','tree']},'candidate-only source-bound phase')
    require(m['approval']['run']=={k:int(v) for k,v in m['run'].items()},'actual approval run selectors')
    require(m['approval']['prepared']=='PREPARATION.json' and m['approval']['approved']==[{'candidate':'approval.public.candidate.json','output':'approval.json'},{'candidate':'ui-approval.candidate.json','output':'ui-approval.json'}],'original candidate approval paths')
    require(p['root']==m['approval']['root']+'/baseline' and p['workRoot']==m['hostRoot']+'/preparation-work','fresh fixed work and approval roots')
    root=pathlib.Path(p['root']);work=pathlib.Path(p['workRoot'])
    require(root.is_relative_to(m['hostRoot']) and root!=work and not root.is_relative_to(work) and not work.is_relative_to(root),'separate small approval and large work roots')
    outer=pathlib.Path(m['approval']['root']);common.trusted(str(outer.parent),True);outer.mkdir(mode=0o700)
    for target in [root,work]:common.trusted(str(target.parent),True);target.mkdir(mode=0o700)
    require(c['runtime']['operatorRoot']==str(outer/'operators'),'public helpers inside complete approval inventory')
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
        deps={'toolsClosure':c['runtime']['toolsClosure']};require(deps['toolsClosure']=={'path':str(root/'OWNER_TOOLS.json')},'generated original UI owner-tools closure')
        seed=pathlib.Path(c['seed']['directory']);historic=read(seed/'metadata/host-python-policy.current.json')
        python=common.file_pin(seed/'host-python/bin/python3.12')
        require(python==m['files']['python'],'selected root interpreter is exact approved seed host Python')
        provider_path=pathlib.Path(pin(p['hostFileProvider']))
        require(provider_path.name=='host-file-profile.py','exact independently selected public provider module')
        provider_module=load('reviewed_public_host_file_profile',provider_path)
        provider=provider_module.generate(m,root)
        pin(p['hostFileProvider'])
        selected_logging={}
        for name,count,digest in [('safe-logging-owner_main.py',13162,'5df1a1d4a9f30f7e3edb87ba24a0468b52f07f2dcf3274592a0a4ddfecba8b5d'),('ui-owner-failure-enums.json',46491,'5060c0e0cb5b86b569bc19c4a5f7e3bb2525a95cff49765c8d5589afbc4f13ae')]:
            original=provider_path.parent/name;before=common.file_pin(original)
            require(before['bytes']==count and before['sha256']==digest,'exact separately selected safe118 logger/map')
            target=root/'host-file-provider'/name;copy(original,target);target.chmod(0o444)
            retained=common.file_pin(target)
            require((retained['bytes'],retained['sha256'])==(count,digest) and common.file_pin(original)==before,'whole readonly safe logging selection retained')
            selected_logging[name]={'original':before,'retained':retained}
        provider['selectedLogging']=selected_logging
        save(root/'HOST_FILE_PROVIDER_SELECTION.json',provider)
        tools={'python':{**python,'ownerUid':0},'files':[],'runtime':{'version':'3.12','files':[]}}
        names=['engine.py','plan.py','owner_main.py','owner_relay.py','owner_lifecycle.py','owner_attach.py','owner_config.py','owner_identity.py','owner_audit.py','owner_kernel.py','owner_proc.py','owner_cgroup.py','owner_mounts.py']
        for name in names:
            base='preparation' if name in ['engine.py','plan.py'] else 'ui-qualification'
            path=repo/'examples/playground/restricted/daemon'/base/name
            if name=='owner_main.py':path=pathlib.Path(selected_logging['safe-logging-owner_main.py']['retained']['path'])
            if name=='owner_mounts.py':
                path=pathlib.Path(provider['helper']['path'])
                require(common.file_pin(path)==provider['helper'],'actual whole generated public host-file observer')
            tools['files'].append({'name':name,**common.file_pin(path),'ownerUid':0})
        for row in historic['runtime']['files']:
            path=seed/'host-python'/row['name'];common.trusted(str(path));b=path.read_bytes()
            require(len(b)==row['bytes'] and hashlib.sha256(b).hexdigest()==row['sha256'],'whole approved seed Python runtime leaf')
            tools['runtime']['files'].append({**row,'path':str(path),'ownerUid':0})
        save(root/'OWNER_TOOLS.json',tools)
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
        selection={'source':identity,'compiler':{'bytes':compiler['bytes'],'sha256':compiler['sha256']},'candidateReview':c['candidateReview'],'frozenMain':c['frozenMain']}
        actual={'status':'prepared-unapproved','source':source,'selection':selection,'images':images,'materials':volumes,'materialBuild':materials,'plans':plans,'workRoot':str(work)}
        save(root/'RESULT.json',actual)
        save(root/'INPUT_OBSERVATIONS.json',{'status':'actual-runtime-preparation-observations-not-approved','source':source,'selection':selection,'images':images,'materials':volumes,'scope':'Operator-owned fresh API/source observations, not independent review or guest mounted full material audit. No guest has started; original runtime audit remains mandatory.'})
        prefix=root/'COMPOSITION_COMMAND';argv=[m['files']['node']['path'],p['compositionModule']['path'],str(manifest_path)]
        save(str(prefix)+'.intent.json',{'argv':argv,'cwd':str(repo),'timeoutSeconds':60})
        with pathlib.Path(str(prefix)+'.stdout').open('xb') as out,pathlib.Path(str(prefix)+'.stderr').open('xb') as err:
            completed=subprocess.run(argv,cwd=repo,stdin=subprocess.DEVNULL,stdout=out,stderr=err,timeout=60)
        save(str(prefix)+'.json',{'returncode':completed.returncode});require(completed.returncode==0,'original candidate composition command succeeded')
        copy(m['files']['context']['path'],root/'REMOTE_RUNTIME_CONTEXT.json')
        porter=load('remote_public_port',p['publicPort']['path']);permissions=porter.make_sources_readable(c,materials,plans);outer.chmod(0o755)
        save(root/'CALLER_SOURCE_PERMISSIONS.json',{'source':source,'callerUid':1000,'callerGid':1000,'changes':permissions,'scope':'Root-owned original byte-identical inputs made readable before public authentication/PREP; no caller-writable source or guest execution.'});porter.port_public(c,m)
        clean_source();pin(m['files']['context']);pin(c['frozenMain']);pin(c['candidateReview'])
        for name in module_names:pin(p[name])
        require(common.file_pin(candidate['compilerPath'])==compiler,'actual candidate retained unchanged')
        save(pathlib.Path(m['hostRoot'])/'PUBLIC_BASELINE_PREPARED.json',{'status':'baseline-prepared-unapproved','source':identity,'baseline':str(root),'scope':'No public authentication, protected approval, PREPARATION inventory or guest execution. Same live Node coordinator owns subsequent phases.'})
    except BaseException as error:
        if not (root/'FAILURE.json').exists():save(root/'FAILURE.json',{'status':'preparation-failed','source':identity,'type':type(error).__name__,'message':str(error)[:1024],'cleanup':'Exact original returned image/volume/holder identities retained. No uncertain object force-removal or approval; host owner retains isolated daemon cleanup.'})
        raise


if __name__=='__main__':main(pathlib.Path(sys.argv[1]))
