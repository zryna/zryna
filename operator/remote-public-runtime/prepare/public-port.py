"""Inert literal public-operator port; no authentication, approval or guest launch."""
import difflib
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import pathlib

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


OLD='b3be654c452ea51f2c136a75d2eb89995eb4bd66'
TREE='3fa38d7b62b06a64ac5385d610be164f9b1961b8'


def need(value,reason):
    if not value:raise ValueError(reason)


def exact(value,keys):need(type(value) is dict and set(value)==set(keys),'closed public port record')


def raw(path):
    path=Path(path);need(path.is_absolute() and path==Path(os.path.normpath(path)),'absolute port input')
    for p in [*reversed(path.parents),path]:
        s=p.lstat();need(s.st_uid==0 and not s.st_mode&0o022 and not stat.S_ISLNK(s.st_mode),'root-owned ordinary input ancestry')
    s=path.lstat();need(stat.S_ISREG(s.st_mode) and s.st_nlink==1 and s.st_size<=1024**3,'bounded single-link input')
    b=path.read_bytes();t=path.lstat();need((s.st_dev,s.st_ino,s.st_size,s.st_mtime_ns,s.st_ctime_ns)==(t.st_dev,t.st_ino,t.st_size,t.st_mtime_ns,t.st_ctime_ns),'stable whole input');return b


def pin(path):
    b=raw(path);return {'path':str(path),'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()}


def checked(p):exact(p,['path','bytes','sha256']);need(pin(p['path'])==p,'selected whole input');return p['path']


def port_public(c,m):
    need(os.geteuid()==0 and m['approval']['phase']=='public','trusted unapproved public preparation')
    runtime=c['runtime'];exact(runtime,['template','publicTemplate','operatorRoot','outputRoot','toolsClosure'])
    root=Path(m['approval']['root']);baseline=Path(c['prepare']['root']);operators=Path(runtime['operatorRoot']);output=Path(runtime['outputRoot'])
    need(baseline==root/'baseline' and operators==root/'operators' and output==Path(m['hostRoot'])/'public-observations','fixed separate roots')
    need(runtime['toolsClosure']=={'path':str(baseline/'OWNER_TOOLS.json')},'actual baseline tools closure')
    operators.mkdir(mode=0o755)
    files=[];diffs=[]
    def save(name,b):
        target=operators/name
        with target.open('xb') as stream:stream.write(b)
        target.chmod(0o444);files.append(pin(target))
    for key in ['template','publicTemplate']:exact(runtime[key],['directory','manifest'])
    historical=Path(runtime['template']['directory']);catalog=json.loads(raw(checked(runtime['template']['manifest'])))
    need(catalog['actualFutureSource'] is None and catalog['approvalGranted'] is False and catalog['newSourceEvidence'] is False,'historical inert CLI carrier')
    references=[]
    for row in catalog['files']:
        b=raw(historical/row['carrierPath']);need(len(b)==row['bytes'] and hashlib.sha256(b).hexdigest()==row['sha256'],'whole historical carrier')
        if row['carrierPath'].startswith('source-reference/'):
            name=row['carrierPath'].removeprefix('source-reference/');actual=raw(Path(c['source']['path'])/name)
            if b!=actual:
                need(name in ['examples/playground/restricted/daemon/compiler-publication-constants.mjs','examples/playground/restricted/daemon/compiler-publication-sbom-graph.mjs'] and b'compiler.2' in b and actual==b.replace(b'compiler.2',b'compiler.3'),'only reviewed tag substitution in original source references')
                references.append({'path':name,'historicalSha256':row['sha256'],'actual':pin(Path(c['source']['path'])/name)})
    selector=json.loads(raw(historical/'oracles/SELECTED19.json'));roots={}
    for row in catalog['files']:
        if row['role']=='historical-original-selected-CLI-stream':roots[str(Path(row['originalPath']).parent)]=str((historical/row['carrierPath']).parent)
    for row in selector['rows']:
        old=row['captureRoot'];need(old in roots,'complete original historical capture transfer');row['originalCaptureRoot']=old;row['captureRoot']=roots[old]
    selector.update(integration={k:c['source'][k] for k in ['commit','tree']},transferredHistoricalSelection=pin(historical/'oracles/SELECTED19.json'),comparisonScope='New source comparison only; original source, timestamps, CLI owner and failed batch remain historical. No fresh CLI execution.')
    save('SELECTED19.json',(json.dumps(selector,sort_keys=True,indent=2)+'\n').encode())
    template=Path(runtime['publicTemplate']['directory']);public=json.loads(raw(checked(runtime['publicTemplate']['manifest'])))
    need(public['format']=='zryna.m6-remote-public-template.v1' and public['version']==1 and public['actualFutureSource'] is None and public['acceptance'] is False and public['historicalSource']=={'commit':OLD,'tree':TREE},'actual original public templates without acceptance claim')
    names=['m6-410-public-publication-state-20261018.mjs']+['m6-410-run-public-'+n+'-20261018.mjs' for n in ['parity','ui','boundaries','isolation','documentation']]
    need([r['path'] for r in public['files']]==names,'exact state and five original helpers')
    old_ops='/workspace/scratch/M6_410_PUBLIC_COORDINATOR_OPERATORS_'+OLD+'_RUN2'
    replacements={old_ops:str(operators),'/workspace/scratch/M6_410_PUBLIC_LAUNCHER_PREPARATION_'+OLD+'_RUN2':str(root/'caller-view'),'/workspace/scratch/M6_410_CLI_COMPARISON_SELECTION_'+OLD+'_RUN1/SELECTED19.json':str(operators/'SELECTED19.json'),'/workspace/scratch/M6_410_HOST_TOOLS_20261018_RUN2/POLICY_CAPTURED_CLOSURE.json':str(root/'caller-view/OWNER_TOOLS.json')}
    prefixes=['M6_410_ACTUAL_PUBLISHED_HTTP_PARITY','M6_410_ACTUAL_PUBLISHED_UI','M6_410_ACTUAL_PUBLISHED_HTTP_BOUNDARIES','M6_410_ACTUAL_PUBLISHED_SESSION_ISOLATION','M6_410_ACTUAL_PUBLISHED_DOCUMENTATION_EXERCISE']
    for prefix in prefixes:replacements['/workspace/scratch/'+prefix+'_'+OLD+'_RUN2']=str(output/(prefix+'_'+c['source']['commit']+'_RUN1'))
    for row in public['files']:
        b=raw(template/row['path']);need(len(b)==row['bytes'] and hashlib.sha256(b).hexdigest()==row['sha256'],'whole original public operator')
        text=b.decode()
        # Retain unexpected real policy-error bytes before the unchanged original HTTP oracle.
        if row['path']=='m6-410-run-public-boundaries-20261018.mjs':
            anchor=' async function compilation(id,revision,source,rawBody,hold=false){'
            need(text.count(anchor)==1,'original compile helper')
            text=text.replace(anchor," async function retainUnexpected(response,id,path){if(response.status===200)return;const raw=await bytes(response,2048);save(id+'.unexpected-body.bin',raw);save(id+'.unexpected-response.json',{id,path,status:response.status,bodyBytes:raw.length,bodySha256:sha(raw)});}\n"+anchor)
            old='source}));assert.equal(response.status,200,id);';need(text.count(old)==1,'original compile status oracle');text=text.replace(old,"source}));await retainUnexpected(response,id,'/compile');assert.equal(response.status,200,id);")
            old="const response=await fetchRaw('/evaluate',JSON.stringify(selection));assert.equal(response.status,200);";need(text.count(old)==1,'original eval status oracle');text=text.replace(old,"const response=await fetchRaw('/evaluate',JSON.stringify(selection));await retainUnexpected(response,'recovery-evaluate','/evaluate');assert.equal(response.status,200);")
            old='}catch(error){result.failure=';need(text.count(old)==1,'original failure record');text=text.replace(old,'}catch(error){result.rows=rows;result.failure=')
        for old,new in sorted(replacements.items(),key=lambda p:len(p[0]),reverse=True):text=text.replace(old,new)
        text=text.replace(OLD,c['source']['commit']).replace(TREE,c['source']['tree'])
        leftovers=re.findall(r'/workspace/scratch/[A-Za-z0-9_./-]+',text)
        need(all(p.startswith((str(root)+'/',str(output)+'/')) or p==str(root) for p in leftovers),'no unported historical scratch dependency')
        save(row['path'],text.encode());diffs.append({'path':row['path'],'originalSha256':row['sha256'],'diff':''.join(difflib.unified_diff(b.decode().splitlines(True),text.splitlines(True)))})
    save('PORT_DIFFS.json',(json.dumps({'source':c['source'],'sourceReferences':references,'changes':diffs},indent=2)+'\n').encode())
    save('BOUND_PUBLIC.json',(json.dumps({'format':'zryna.m6-remote-public-binding.v1','version':1,'source':c['source'],'caller':{'uid':1000,'gid':1000,'supplementaryGroups':[]},'callerView':str(root/'caller-view'),'files':files,'originalTemplates':runtime['publicTemplate']['manifest'],'historicalComparison':runtime['template']['manifest'],'scope':'Source/path port with bounded HTTP failure retention only; not authentication, approval or execution.'},indent=2)+'\n').encode())
