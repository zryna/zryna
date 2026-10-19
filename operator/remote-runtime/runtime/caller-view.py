"""Root-owned readonly caller carriers; never approve or execute a guest/source caller."""
import hashlib,json,os,pathlib,stat,subprocess

def make_sources_readable(context,materials,plans):
    assert os.geteuid()==0
    root=pathlib.Path(context['prepare']['root']);work=pathlib.Path(context['prepare']['workRoot']);view=root/'caller-view'
    assert root.is_absolute()and work.is_absolute()and root.resolve()==root and work.resolve()==work
    fields=['st_dev','st_ino','st_mode','st_nlink','st_size','st_uid','st_gid','st_mtime_ns','st_ctime_ns']
    same=lambda s:tuple(getattr(s,k)for k in fields)
    def pin(p):
        p=pathlib.Path(p);assert p.resolve()==p;s=p.lstat();assert stat.S_ISREG(s.st_mode)and s.st_nlink==1 and s.st_uid==0 and not s.st_mode&0o022
        h=hashlib.sha256();n=0
        with p.open('rb')as f:
            while b:=f.read(65536):h.update(b);n+=len(b)
        assert n==s.st_size and same(s)==same(p.lstat());return{'path':str(p),'bytes':n,'sha256':h.hexdigest()}
    def save(p,value):
        raw=(json.dumps(value,indent=2)+'\n').encode()
        with p.open('xb')as f:f.write(raw)
        p.chmod(0o444);return pin(p)
    # Only root-owned selected source parents become searchable, never caller-writable.
    permissions=[]
    def searchable(p):
        p=pathlib.Path(p);assert p.is_relative_to(work)or p==root
        chain=[]
        while p==root or p.is_relative_to(work):
            chain.append(p)
            if p in[root,work]:break
            p=p.parent
        for p in reversed(chain):
            s=p.lstat();assert stat.S_ISDIR(s.st_mode)and not p.is_symlink()and s.st_uid==0 and not s.st_mode&0o022
            if stat.S_IMODE(s.st_mode)!=0o755:
                p.chmod(0o755);permissions.append({'path':str(p),'kind':'directory','uid':0,'originalMode':stat.S_IMODE(s.st_mode),'newMode':0o755})
    source_rows={}
    def selected_source(row):
        previous=source_rows.get(row['sourcePath'])
        assert previous is None or all(previous[k]==row[k]for k in['bytes','sha256','mode'])
        source_rows[row['sourcePath']]=row
    for role in['compiler','evaluator']:
        selected=materials[role]['plan'];assert pin(selected['path'])==selected;plan=json.loads(pathlib.Path(selected['path']).read_bytes())
        for row in plan['files']:selected_source(row)
    for role in['compiler','evaluator','ui']:
        selected=plans['plans'][role];p=pathlib.Path(selected['path']);actual=pin(p);assert(actual['bytes'],actual['sha256'])==(selected['planBytes'],selected['planSha256'])
        for row in json.loads(p.read_bytes())['files']:selected_source(row)
    for name,row in sorted(source_rows.items()):
        p=pathlib.Path(name);assert p.is_relative_to(work)or p.is_relative_to(context['seed']['directory'])
        before=pin(p);assert(before['bytes'],before['sha256'])==(row['bytes'],row['sha256']);mode=0o555 if row['mode']==0o555 else 0o444;assert row['mode']in[0o444,0o555]
        if p.is_relative_to(work):searchable(p.parent)
        else:
            for ancestor in[p.parent,*p.parents]:
                if str(ancestor)=='/':break
                s=ancestor.lstat();assert stat.S_ISDIR(s.st_mode)and s.st_uid==0 and not s.st_mode&0o022 and s.st_mode&0o005==0o005
        original=stat.S_IMODE(p.lstat().st_mode);p.chmod(mode);after=pin(p);assert after==before
        permissions.append({'path':name,'kind':'selected-source','uid':0,'originalMode':original,'newMode':mode,'wholePin':after})
    searchable(root);return permissions

def prepare_view(context,manifest,materials,plans,caller_uid):
    assert os.geteuid()==0 and caller_uid==1000
    root=pathlib.Path(context['prepare']['root']);work=pathlib.Path(context['prepare']['workRoot']);view=root/'caller-view'
    assert str(root)==manifest['approval']['root'] and work==pathlib.Path(manifest['hostRoot'])/'preparation-work'
    fields=['st_dev','st_ino','st_mode','st_nlink','st_size','st_uid','st_gid','st_mtime_ns','st_ctime_ns']
    same=lambda s:tuple(getattr(s,k)for k in fields)
    def pin(p):
        p=pathlib.Path(p);assert p.resolve()==p;s=p.lstat();assert stat.S_ISREG(s.st_mode)and s.st_nlink==1 and s.st_uid==0 and not s.st_mode&0o022
        h=hashlib.sha256();n=0
        with p.open('rb')as f:
            while b:=f.read(65536):h.update(b);n+=len(b)
        assert n==s.st_size and same(s)==same(p.lstat());return{'path':str(p),'bytes':n,'sha256':h.hexdigest()}
    def save(p,value):
        raw=(json.dumps(value,indent=2)+'\n').encode()
        with p.open('xb')as f:f.write(raw)
        p.chmod(0o444);return pin(p)
    # Only root-owned selected source parents become searchable, never caller-writable.
    permissions=[]
    def searchable(p):
        p=pathlib.Path(p);assert p.is_relative_to(work)or p==root
        chain=[]
        while p==root or p.is_relative_to(work):
            chain.append(p)
            if p in[root,work]:break
            p=p.parent
        for p in reversed(chain):
            s=p.lstat();assert stat.S_ISDIR(s.st_mode)and not p.is_symlink()and s.st_uid==0 and not s.st_mode&0o022
            if stat.S_IMODE(s.st_mode)!=0o755:
                p.chmod(0o755);permissions.append({'path':str(p),'kind':'directory','uid':0,'originalMode':stat.S_IMODE(s.st_mode),'newMode':0o755})
    permissions=make_sources_readable(context,materials,plans)
    searchable(root);view.mkdir(mode=0o755);copies=[]
    def copy(source,name,mode=0o444):
        source=pathlib.Path(source);before=pin(source);target=view/name
        with source.open('rb')as a,target.open('xb')as b:
            while chunk:=a.read(65536):b.write(chunk)
        target.chmod(mode);actual=pin(target);assert pin(source)==before and(actual['bytes'],actual['sha256'])==(before['bytes'],before['sha256']);copies.append({'original':before,'view':actual,'mode':mode});return actual
    for name in['compiler.bin','loader.js','candidate-receipt.json','source-composition.json','complete-policy.candidate.json','policy-input.candidate.json','OWNER_TOOLS.json']:
        copy(root/name,name,0o555 if name=='compiler.bin'else 0o444)
    approvals=[]
    for pair in manifest['approval']['approved']:
        projected=copy(root/pair['candidate'],pair['output'])
        approvals.append({'canonical':str(root/pair['output']),'view':projected,'candidate':pin(root/pair['candidate'])})
    original_path=root/'reviewed-launcher.candidate.json';original=pin(original_path);reviewed=json.loads(original_path.read_bytes())
    for entry,name in[(reviewed['compiler'],'compiler.bin'),(reviewed['template'],'loader.js'),(reviewed['approval'],'approval.json'),(reviewed['compilerPublication']['receipt'],'candidate-receipt.json')]:
        actual=pin(view/name);assert(entry['bytes'],entry['sha256'])==(actual['bytes'],actual['sha256']);entry['path']=str(view/name);entry['ownerUid']=0
    projected=save(view/'reviewed-launcher.candidate.json',reviewed)
    # The original API explicitly excludes physical paths/ownerUid from composition identity.
    code="""import assert from 'node:assert/strict';import{readFileSync}from'node:fs';import{pathToFileURL}from'node:url';import{createHash}from'node:crypto';
const[root,view,source]=process.argv.slice(1),{composition,validatePolicy}=await import(pathToFileURL(source+'/examples/playground/restricted/daemon/launcher-policy.mjs'));
const a=JSON.parse(readFileSync(root+'/reviewed-launcher.candidate.json')),b=JSON.parse(readFileSync(view+'/reviewed-launcher.candidate.json'));validatePolicy(a);validatePolicy(b);
const x=Buffer.from(JSON.stringify(composition(a))),y=Buffer.from(JSON.stringify(composition(b)));assert(x.equals(y));assert(x.equals(readFileSync(root+'/source-composition.json')));assert.equal(a.compositionSha256,b.compositionSha256);console.log(JSON.stringify({sameOriginalComposition:true,compositionSha256:createHash('sha256').update(x).digest('hex')}));"""
    argv=[manifest['files']['node']['path'],'--input-type=module','-e',code,str(root),str(view),context['source']['path']]
    save(root/'CALLER_COMPOSITION_INVARIANT.intent.json',{'argv':argv,'scope':'Original composition()/validatePolicy() on static descriptors only; no launcher or guest execution.'})
    with(root/'CALLER_COMPOSITION_INVARIANT.stdout').open('xb')as out,(root/'CALLER_COMPOSITION_INVARIANT.stderr').open('xb')as err:
        result=subprocess.run(argv,stdin=subprocess.DEVNULL,stdout=out,stderr=err,timeout=15,check=False)
    assert result.returncode==0 and pin(original_path)==original
    proof=json.loads((root/'CALLER_COMPOSITION_INVARIANT.stdout').read_bytes());assert proof['sameOriginalComposition']is True and proof['compositionSha256']==reviewed['compositionSha256']
    save(root/'CALLER_PERMISSION_MANIFEST.json',{'source':context['source'],'callerUid':caller_uid,'callerGid':caller_uid,'changes':permissions,'copies':copies,'originalDescriptor':original,'projectedDescriptor':projected,'compositionInvariant':proof,'canonicalActualApprovals':'Remain absent until original protected-tag gate creates root0/0400 outputs; static view copies are candidates only and must equal actual outputs before launch.','scope':'Root-owned readonly projection only. No source bytes changed, no600MBcopies, no approval/guest/runtime acceptance.'})
    record={'root':str(view),'approvals':approvals,'descriptor':projected,'permissionManifest':pin(root/'CALLER_PERMISSION_MANIFEST.json')}
    save(root/'CALLER_VIEW.json',record);return record
