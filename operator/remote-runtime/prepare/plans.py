"""Small actual-source runtime-plan adapter around the original preparation APIs."""
import hashlib,json,pathlib,re,struct,subprocess,sys


def build_runtime_plans(source,candidate,seed,materials,output,node):
    repo=pathlib.Path(source['path']);seed=pathlib.Path(seed);root=pathlib.Path(output)
    root.mkdir(mode=0o700);sys.path.insert(0,str(repo/'examples/playground/restricted/daemon/preparation'))
    from compose import candidate as compose_candidate,encode,verify_sources
    import ui_plan
    sha=lambda b:hashlib.sha256(b).hexdigest()
    origins=[]
    def put(p,b,mode=0o400):
        p.parent.mkdir(parents=True,exist_ok=True,mode=0o700)
        with p.open('xb') as f:f.write(b)
        p.chmod(mode)
    def dump(p,v):put(p,(json.dumps(v,indent=2)+'\n').encode())
    def source_bytes(rel):
        b=(repo/rel).read_bytes()
        actual=subprocess.run(['git','--no-replace-objects','show',source['commit']+':'+rel],cwd=repo,capture_output=True,check=True,timeout=10).stdout
        assert b==actual,'Actual selected source blob differs: '+rel
        return b
    def item(dest,data,origin,mode=0o444):
        p=root/'sources'/dest;put(p,data,0o500 if mode==0o555 else 0o400)
        v={'path':dest,'sourcePath':str(p),'bytes':len(data),'sha256':sha(data),'mode':mode};origins.append({**v,'origin':origin});return v
    binary=pathlib.Path(candidate['compilerPath']).read_bytes()
    assert len(binary)==candidate['compiler']['bytes'] and sha(binary)==candidate['compiler']['sha256']
    compiler=item('app/compiler',binary,{'actualOfficialCandidate':candidate['compilerPath'],'sourceCommit':source['commit']},0o555)
    license=item('licenses/zryna-LICENSE',source_bytes('LICENSE'),{'git':'LICENSE'})
    # Historical supervisor bytes are reusable only while all actual source inputs remain identical.
    provenance=json.loads((seed/'metadata/ubuntu-static-source-scope.original.json').read_bytes())
    for row in provenance['sources']:
        rel=str(pathlib.PurePosixPath(row['path']).relative_to('/workspace/zryna'));b=source_bytes(rel)
        assert len(b)==row['bytes'] and sha(b)==row['sha256'],'Seed supervisor source changed; fresh build required'
    supervisors={}
    for name,leaf in [('supervisor','daemon-supervisor'),('ui-supervisor','ui-supervisor')]:
        p=seed/'supervisors'/leaf;supervisors[name]=item(name,p.read_bytes(),{'historicalSeedSupervisor':str(p),'originalSourceMain':provenance['sourceMain']},0o555)
    closure=json.loads((seed/'metadata/runtime-library-closure.original.json').read_bytes())
    runtime=[]
    for f in closure['files']:
        p=seed/'runtime-closure'/f['path'];b=p.read_bytes();assert len(b)==f['bytes'] and sha(b)==f['sha256']
        runtime.append({**f,'sourcePath':str(p)})
    manifests={role:item('manifests/'+role+'.json',pathlib.Path(materials[role]['manifest']['path']).read_bytes(),{'actualMaterialManifest':materials[role]['manifest']}) for role in ['compiler','evaluator']}
    pins=source_bytes('crates/zryna-backend-webassembly/src/wit_world_audit/pins.rs').decode()
    matches=re.findall(r'pin\(\s*(?:ROOT_PACKAGE|"[^"]+")\s*,\s*"([^"]+)"\s*,\s*"([a-f0-9]{64})"\s*,\s*\)',pins);assert len(matches)==34
    digest=hashlib.sha256(b'ZRYNA-PINNED-WIT-SOURCES\0\x01');wit=[]
    for path,h in sorted(matches):
        rel=path if path.startswith('spec/') else 'crates/zryna-backend-webassembly/tests/wit-world-audit-v1/dependencies/'+path.removeprefix('wasi/')
        data=source_bytes(rel);assert sha(data)==h;p=path.encode();digest.update(struct.pack('<Q',len(p)));digest.update(p);digest.update(struct.pack('<Q',len(data)));digest.update(data);wit.append({'path':path,'source':rel,'bytes':len(data),'sha256':h})
    wit_sha=digest.hexdigest();dump(root/'WIT34_IDENTITY.json',{'sha256':wit_sha,'sources':wit})
    template=source_bytes('crates/zryna-driver/src/browser_component/loader.js');template_item=item('resources/loader.js',template,{'git':'crates/zryna-driver/src/browser_component/loader.js'})
    downloads=json.loads((seed/'metadata/downloads.json').read_bytes())['inputs'];chrome=[x for x in downloads if 'chrome' in x['name']];assert len(chrome)==1
    identity={'version':1,'compilerSha256':sha(binary),'bindingTemplateSha256':sha(template),'witSha256':wit_sha,'browserArchiveSha256':chrome[0]['sha256']}
    identity_item=item('resources/evaluation-identity.json',json.dumps(identity,separators=(',',':')).encode(),{'actualCandidateBoundEvaluationIdentity':True})
    def manifest(role):return {**manifests[role],'path':'resources/materials-manifest.json'}
    compiler_libs={'ld-linux-x86-64.so.2','libc.so.6','libdl.so.2','libgcc_s.so.1','libm.so.6','libpthread.so.0','libstdc++.so.6'}
    compiler_runtime=[f for f in runtime if f['path']=='lib64/ld-linux-x86-64.so.2' or f['path'].startswith('lib/') and pathlib.PurePosixPath(f['path']).name in compiler_libs or f['path'] in ['licenses/runtime/ubuntu24/libc6/copyright','licenses/runtime/ubuntu24/libstdc++6/copyright','licenses/runtime/ubuntu24/libgcc-s1/copyright']]
    plans={'compiler':compose_candidate('runtime','compiler',[compiler,license,supervisors['supervisor'],manifest('compiler'),*compiler_runtime])}
    evaluation=[compiler,license,supervisors['supervisor'],manifest('evaluator'),*runtime,template_item,identity_item]
    eval_sources={'app/evaluate.mjs':'daemon/evaluation-entry.mjs',**{'app/'+n+'.mjs':n+'.mjs' for n in ['worker','protocol','limits','json','report','executable','evaluation']},**{'app/daemon/'+n+'.mjs':'daemon/'+n+'.mjs' for n in ['evaluate','evaluation-input','evaluation-page','evaluation-server']}}
    for dest,rel in eval_sources.items():evaluation.append(item(dest,source_bytes('examples/playground/restricted/'+rel),{'git':'examples/playground/restricted/'+rel}))
    plans['evaluator']=compose_candidate('runtime','evaluator',evaluation)
    ui=[compiler,license,supervisors['supervisor'],supervisors['ui-supervisor'],manifest('evaluator'),*runtime]
    base='examples/playground/restricted/daemon/ui-qualification/'
    for dest in sorted(ui_plan.REQUIRED):
        if dest.startswith('app/') and dest!='app/compiler':
            rel=base+('inner.mjs' if dest=='app/ui-driver.mjs' else pathlib.PurePosixPath(dest).name);ui.append(item(dest,source_bytes(rel),{'git':rel}))
    ui.append(item('etc/fonts/fonts.conf',source_bytes('examples/playground/restricted/daemon/preparation/ui-fonts.conf'),{'git':'examples/playground/restricted/daemon/preparation/ui-fonts.conf'}))
    for n in ['NimbusSans-Regular.otf','NimbusMonoPS-Regular.otf']:ui.append(item('resources/fonts/'+n,(seed/'fonts'/n).read_bytes(),{'historicalSeedFont':str(seed/'fonts'/n)}))
    ui.append(item('licenses/fonts/fonts-urw-base35-copyright',(seed/'licenses/fonts-urw-base35-copyright').read_bytes(),{'historicalSeedLicense':True}))
    code='import {planBytes} from '+json.dumps(str(repo/'examples/playground/restricted/daemon/ui-qualification/plan.mjs'))+';process.stdout.write(planBytes())'
    fixed=subprocess.run([node,'--input-type=module','-e',code],capture_output=True,timeout=10,check=True).stdout
    ui.append(item('resources/ui-plan.json',fixed,{'actualSourceFixedUiPlan':True}));plans['ui']=ui_plan.candidate(ui)
    results={}
    for role,plan in plans.items():
        if role!='ui':verify_sources(plan);raw,pin=encode(plan)
        else:
            from handles import Sources
            s=Sources()
            try:
                for f in plan['files']:s.verify(f)
                s.revalidate_files()
            finally:s.close()
            raw=json.dumps(plan,separators=(',',':'),ensure_ascii=True).encode()
        put(root/(role+'-plan.json'),raw);results[role]={'path':str(root/(role+'-plan.json')),'planBytes':len(raw),'planSha256':sha(raw),'files':len(plan['files']),'bytes':plan['bytes']}
    dump(root/'PLAN_PINS.json',results);dump(root/'SOURCE_ORIGINS.json',origins)
    identity.update({'sourceCommit':source['commit'],'sourceTree':source['tree'],'compilerBytes':len(binary),'uiPlanSha256':sha(fixed),'scope':'Fresh selected source/candidate plan bytes; supervisors/libraries/fonts retain original historical seed provenance; no approval or guest execution.'})
    dump(root/'IDENTITY.json',identity)
    return {'root':str(root),'plans':results,'identity':identity}
