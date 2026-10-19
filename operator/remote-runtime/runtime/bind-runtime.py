"""Preparation-only literal port of reviewed mature operators; never runs a guest."""
import difflib
import importlib.util
import json
import os
from pathlib import Path
import re
import sys

HERE = Path(__file__).absolute().parent
spec = importlib.util.spec_from_file_location('runtime_support', HERE / 'runtime_support.py')
s = importlib.util.module_from_spec(spec)
spec.loader.exec_module(s)

JOBS = [
 ('parity','m6-410-run-actual-http-parity-20261018.mjs','M6_410_ACTUAL_CANDIDATE_HTTP_PARITY','actual-nineteen-cli-http-browser-comparison-matched',360),
 ('plain-keyboard',None,'M6_410_PLAIN_KEYBOARD','actual-ordinary-chrome-keyboard-and-contrast-pass',90),
 ('original-ui-owner','m6-410-run-actual-ui-20261018.mjs','M6_410_ACTUAL_CANDIDATE_UI','actual-initial-abort-corpus-ax-ui-pass',360),
 ('http-boundaries','m6-410-run-actual-http-boundaries-20261018.mjs','M6_410_ACTUAL_CANDIDATE_HTTP_BOUNDARIES','actual-http-boundary-cohort-pass',360),
 ('session-isolation','m6-410-run-actual-session-isolation-20261018.mjs','M6_410_ACTUAL_CANDIDATE_SESSION_ISOLATION','actual-source-session-and-contained-browser-isolation-pass',360),
 ('browser-faults','m6-410-run-real-browser-faults-v2-20261018.mjs','M6_410_ACTUAL_BROWSER_FAULTS','actual-browser-watchdog-cancellation-resource-faults-pass',360),
 ('compiler-owner-faults','m6-410-run-real-compiler-owner-faults-20261018.mjs','M6_410_ACTUAL_COMPILER_OWNER_FAULTS','actual-compiler-whole-operation-timeout-and-cancellation-pass',360),
 ('compiler-resources','m6-410-run-real-compiler-resource-faults-20261018.mjs','M6_410_ACTUAL_COMPILER_RESOURCE_FAULTS','actual-current-compiler-resource-faults-pass',360),
 ('pipe-boundaries','m6-410-run-real-pipe-boundaries-20261018.mjs','M6_410_ACTUAL_PIPE_BOUNDARIES','actual-compiler-stdout-stderr-first-extra-boundaries-pass',360),
 ('storage-boundaries','m6-410-run-real-storage-boundaries-20261018.mjs','M6_410_ACTUAL_STORAGE_BOUNDARIES','actual-original-compiler-evaluator-storage-boundaries-pass',360),
 ('material-denials','material-denial-operator.mjs','M6_410_ACTUAL_MATERIAL_DENIAL','actual-material-provenance-and-pre-source-audit-denials-pass',360),
 ('role-kernel-policy',None,'M6_410_KERNEL_POLICY_ACTUAL','two-owned-new-source-role-policy-probes-pass',60),
 ('host-death-and-owned-recovery','m6-410-run-real-host-death-v2-20261018.mjs','M6_410_ACTUAL_HOST_DEATH','actual-host-death-poison-and-operator-recovery-pass',360)]


def bind_runtime(context_path, manifest_path):
    ctx, manifest, composition = s.context_inputs(context_path, manifest_path)
    s.need(manifest['approval']['phase'] == 'qualification', 'private qualification only')
    runtime, deps = ctx['runtime'], ctx['runtime']['dependencies']
    s.exact(runtime['template'], ['directory','manifest'])
    template = Path(runtime['template']['directory'])
    catalogue = s.decode(s.ordinary(s.checked(runtime['template']['manifest'])))
    s.need(catalogue['actualFutureSource'] is None and catalogue['approvalGranted'] is False and
           catalogue['newSourceEvidence'] is False, 'historical inert template carrier only')
    # Read and check the whole supplied carrier, including all historical streams.
    source_changes=[]
    for row in catalogue['files']:
        raw = s.ordinary(template / row['carrierPath'])
        s.need(len(raw) == row['bytes'] and s.sha(raw) == row['sha256'], 'whole original template/oracle bytes')
        if row['carrierPath'].startswith('source-reference/'):
            target = Path(ctx['source']['path']) / row['carrierPath'].removeprefix('source-reference/')
            actual=s.ordinary(target)
            if actual != raw:
                allowed=['examples/playground/restricted/daemon/compiler-publication-constants.mjs',
                         'examples/playground/restricted/daemon/compiler-publication-sbom-graph.mjs']
                s.need(row['carrierPath'].removeprefix('source-reference/') in allowed and b'compiler.2' in raw and
                       actual==raw.replace(b'compiler.2',b'compiler.3'), 'only exact reviewed565 tag substitutions allowed')
                source_changes.append({'path':str(target),'historicalSha256':row['sha256'],'actual':s.pin(target),
                    'diff':''.join(difflib.unified_diff(raw.decode().splitlines(True),actual.decode().splitlines(True)))})
    prepared = s.decode(s.ordinary(composition / 'RESULT.json'))
    observations = s.decode(s.ordinary(deps['inputObservations']['path']))
    s.need(prepared['status'] == 'prepared-unapproved' and prepared['source'] == ctx['source'] and
           observations['status'] == 'actual-runtime-preparation-observations-not-approved' and
           observations['source'] == ctx['source'] and observations['selection'] == prepared['selection'], 'honest actual preparation inputs')
    descriptor = s.decode(s.ordinary(composition / 'reviewed-launcher.candidate.json'))
    approval = s.decode(s.ordinary(composition / 'approval.candidate.json'))
    s.need(approval['cpus'] == [0,1] and approval['engine_uid'] == 65534 and
           descriptor['source']['commit'] == ctx['source']['commit'] and descriptor['source']['tree'] == ctx['source']['tree'] and
           descriptor['compiler']['sha256'] == approval['compiler_executable_sha256'], 'actual candidate and unchanged two-CPU authority')
    source = ctx['source']['commit']
    operators, output = Path(runtime['operatorRoot']), Path(runtime['outputRoot'])
    operators.mkdir(mode=0o700)
    caller_view = composition/'caller-view'; caller_operators = caller_view/'runtime-operators'
    view = s.decode(s.ordinary(composition/'CALLER_VIEW.json')); s.need(view['root']==str(caller_view),'closed readonly caller projection')
    result_roots={prefix:output/('trusted'if name=='role-kernel-policy'else'caller')/root_name for name,_,prefix,_,_ in JOBS for root_name in[prefix+'_'+source+'_RUN1']}
    originals = [r for r in catalogue['files'] if r['carrierPath'].startswith('templates/') and
                 r['carrierPath'] != 'templates/fixed-imports/upload-response-binding.py' and
                 r['carrierPath'] != 'templates/keyboard/context.json']
    replacements = {r['originalPath']:str(operators / r['carrierPath'].removeprefix('templates/')) for r in originals}
    replacements.update({
      '/workspace/scratch/M6_410_LAUNCHER_CPU34_PREPARATION_'+s.OLD_COMMIT+'_RUN1':str(composition),
      '/workspace/scratch/M6_410_RUNTIME_COHORT_OPERATORS_'+s.OLD_COMMIT+'_RUN2_TAIL1':str(operators/'runtime'),
      '/workspace/scratch/m6-410-compose-daemon-policy-20261018.py':str(operators/'fixed-imports/m6-410-compose-daemon-policy-20261018.py'),
      '/workspace/scratch/m6-410-fixed-storage-boundaries-20261018.mjs':deps['storageWatcher']['path'],
      '/workspace/scratch/M6_410_HOST_TOOLS_20261018_RUN2/POLICY_CAPTURED_CLOSURE.json':deps['toolsClosure']['path'],
      '/workspace/scratch/M6_410_MATERIAL_COMPOSITION_20261018_RUN1/sources/browser/chrome-linux64/chrome':deps['browser']['path'],
      '/workspace/scratch/M6_410_MATERIAL_COMPOSITION_20261018_RUN1/sources/playwright/index.mjs':deps['playwrightEntry']['path'],
      '/workspace/scratch/M6_410_RESOURCE_PROBE_PREPARATION_20261018_RUN1':deps['probe']['directory'],
      '/workspace/scratch/M6_410_KERNEL_PRECREATION_CONFIG_DIAGNOSIS_dcef77a79782e91016b9e3e8d528cf21850b9d41_RUN1/EXACT_IMAGE_CONFIG_BLOB.json':deps['probe']['configBlob']['path'],
      '/workspace/scratch/M6_410_CLI_COMPARISON_SELECTION_'+s.OLD_COMMIT+'_RUN1/SELECTED19.json':str(operators/'SELECTED19.json')})
    root_names = {prefix:prefix+'_'+source+'_RUN1' for _,_,prefix,_,_ in JOBS}
    for row in originals:
        text = s.ordinary(template/row['carrierPath']).decode()
        for match in re.finditer(r'/workspace/scratch/(M6_410_(?:ACTUAL_[A-Z_]+|PLAIN_KEYBOARD|KERNEL_POLICY_ACTUAL)_'+s.OLD_COMMIT+r'_RUN[0-9]+)', text):
            old = match.group(1)
            prefix = old.split('_'+s.OLD_COMMIT)[0]
            s.need(prefix in root_names, 'known original observation root')
            replacements[match.group(0)] = str(result_roots[prefix])
            replacements[old] = root_names[prefix]
    inputs = [s.pin(context_path), s.pin(manifest_path), ctx['candidateReview'], ctx['frozenMain'], runtime['template']['manifest'],
              s.pin(composition/'RESULT.json'), s.pin(deps['inputObservations']['path']),
              s.pin(composition/'CALLER_VIEW.json'), s.pin(composition/'CALLER_PERMISSION_MANIFEST.json'), s.pin(HERE/'caller-view.py')]
    inputs.append(s.pin('/usr/bin/python3.12'))
    provider_module=s.pin(HERE/'host-file-profile.py');s.need(provider_module['bytes']==13151 and provider_module['sha256']=='85f7ddc89cd4e3b3c54a60d5281d44d3a30dc15b1f5f843eba2e8fc288751b5d','exact original-host-profile provider generator');inputs.append(provider_module)
    for name in ['daemon-mountinfo.raw','observer-mountinfo.raw','HOST_METADATA.json','HOST_FILE_PROFILE.json','original-owner_mounts.py','owner_mounts.py','PROVENANCE.json']:
        inputs.append(s.pin(composition/'host-file-provider'/name))
    inputs.append(s.pin(composition/'HOST_FILE_PROVIDER_SELECTION.json'))
    logging=s.pin(HERE/'safe-logging-owner_main.py');s.need(logging['bytes']==13162 and logging['sha256']=='5df1a1d4a9f30f7e3edb87ba24a0468b52f07f2dcf3274592a0a4ddfecba8b5d','exact reviewed118-enum logging subject');inputs.append(logging)
    logging=s.pin(HERE/'ui-owner-failure-enums.json');s.need(logging['bytes']==46491 and logging['sha256']=='5060c0e0cb5b86b569bc19c4a5f7e3bb2525a95cff49765c8d5589afbc4f13ae','exact reviewed118-enum logging subject');inputs.append(logging)  # Fixed trusted native owner only; all other Python roles retain PBS.
    for key in ['storageWatcher','toolsClosure','browser','playwrightEntry']:
        inputs.append(s.pin(deps[key]['path']))
    for key in ['source','imageMetadata','configBlob']:
        inputs.append(s.pin(deps['probe'][key]['path']))
    diffs, files = [], []
    def emit(relative, raw):
        path = operators/relative
        path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        with path.open('xb') as stream:
            stream.write(raw)
        path.chmod(0o400)
        files.append(s.pin(path))
    for row in originals:
        relative = row['carrierPath'].removeprefix('templates/')
        original = s.ordinary(template/row['carrierPath'])
        if relative.endswith(('.mjs','.py')):
            text = original.decode()
            if relative == 'kernel/run-policy-probes.py':
                imports="sys.path.insert(0,'/workspace/scratch/M6_410_UPLOAD_RESPONSE_FIX_RUNTIME_PREPARATION_20261018_RUN1')\nfrom binding import select,ordinary,sha\n"
                s.need(text.count(imports)==1,'exact historical kernel import block')
                text=text.replace(imports,'')
                start, end = text.index('import argparse\n'), text.index('approval_raw=ordinary')
                header = '''import argparse
def ordinary(path):
    before=pathlib.Path(path).stat();assert before.st_uid==0 and not before.st_mode&0o022
    raw=pathlib.Path(path).read_bytes();assert len(raw)==before.st_size;return raw
def sha(raw):return hashlib.sha256(raw).hexdigest()
parser=argparse.ArgumentParser();parser.add_argument('--candidate-review',required=True);args=parser.parse_args()
context=json.loads(ordinary(pathlib.Path(CONTEXT_PATH)))
assert args.candidate_review==context['candidateReview']['path']
assert sha(ordinary(pathlib.Path(args.candidate_review)))==context['candidateReview']['sha256']
composition=pathlib.Path(context['prepare']['root'])
prepared=json.loads(ordinary(composition/'RESULT.json'));selection=prepared['selection']
observations=json.loads(ordinary(pathlib.Path(context['runtime']['dependencies']['inputObservations']['path'])))
assert prepared['status']=='prepared-unapproved' and prepared['source']==context['source']
assert selection['source']['commit']==context['source']['commit'] and selection['source']['tree']==context['source']['tree']
assert observations['status']=='actual-runtime-preparation-observations-not-approved' and observations['source']==context['source'] and observations['selection']==selection
'''.replace('CONTEXT_PATH',repr(str(context_path)))
                text = text[:start]+header+text[end:]
                old="assert probe_state['Config']==canonical_config\n"
                s.need(text.count(old)==1,'original retained CLI1.51 Config guard')
                text=text.replace(old,'# Retained IMAGE is a genuine fresh original Engine API1.49 response; validate closed17 below.\n')
                old='expected_full_config={**canonical_config,**api149_defaults}\n'
                s.need(text.count(old)==1,'original closed17 projection')
                text=text.replace(old,old+"assert set(probe_state['Config'])==set(expected_full_config),'Closed retained actual API1.49 config required'\n"+
                    "for key,value in expected_full_config.items():\n"+
                    "    actual=probe_state['Config'][key];assert type(actual) is type(value) and actual==value,('RetainedConfig',key)\n")
            if relative=='runtime/m6-410-run-actual-http-boundaries-20261018.mjs':
                anchor=' async function compilation(id,revision,source,rawBody,hold=false){'
                diagnostic=""" async function retainUnexpected(response,id,path){
  if(response.status===200)return;
  const raw=await bytes(response,2048);save(id+'.unexpected-body.bin',raw);
  save(id+'.unexpected-response.json',{id,path,status:response.status,bodyBytes:raw.length,bodySha256:sha(raw)});
 }
"""
                s.need(text.count(anchor)==1,'original compile helper')
                text=text.replace(anchor,diagnostic+anchor)
                old='source}));assert.equal(response.status,200,id);'
                s.need(text.count(old)==1,'original compile status oracle')
                text=text.replace(old,"source}));await retainUnexpected(response,id,'/compile');assert.equal(response.status,200,id);")
                old="const response=await fetchRaw('/evaluate',JSON.stringify(selection));assert.equal(response.status,200);"
                s.need(text.count(old)==1,'original evaluate status oracle')
                text=text.replace(old,"const response=await fetchRaw('/evaluate',JSON.stringify(selection));await retainUnexpected(response,'recovery-evaluate','/evaluate');assert.equal(response.status,200);")
                old='}catch(error){result.failure='
                s.need(text.count(old)==1,'original failure record')
                text=text.replace(old,'}catch(error){result.rows=rows;result.failure=')
            local_replacements = replacements if relative=='kernel/run-policy-probes.py' else {old:new.replace(str(composition),str(caller_view)) for old,new in replacements.items()}
            for old,new in sorted(local_replacements.items(),key=lambda item:len(item[0]),reverse=True):
                text = text.replace(old,new)
            text = text.replace(s.OLD_COMMIT,source).replace(s.OLD_TREE,ctx['source']['tree'])
            chosen_uid = 0 if relative=='kernel/run-policy-probes.py' else s.CALLER_UID
            for old,new in [('process.getuid(),1000','process.getuid(),'+str(chosen_uid)),('os.getuid()==1000','os.getuid()=='+str(chosen_uid)),
                            ("ps['Uid'].split()!=['1000']*4","ps['Uid'].split()!=["+repr(str(chosen_uid))+"]*4"),("'3-4'","'0-1'")]:
                text = text.replace(old,new)
            if relative == 'keyboard/owner.mjs':
                old="assert(root==='/workspace/scratch/M6_410_PLAIN_KEYBOARD_'+source.commit+'_RUN6');"
                s.need(old in text, 'original keyboard fixed root guard')
                text=text.replace(old,'assert(root==='+json.dumps(str(result_roots['M6_410_PLAIN_KEYBOARD']))+');')
            if relative=='keyboard/worker.mjs':
                old="assert(root.startsWith('/workspace/scratch/M6_410_')&&prep.startsWith('/workspace/scratch/M6_410_'));"
                s.need(text.count(old)==1,'original keyboard path guards')
                text=text.replace(old,'assert(root==='+json.dumps(str(result_roots['M6_410_PLAIN_KEYBOARD']))+
                                  '&&prep==='+json.dumps(str(caller_view))+');')
            # Fixed observers use the same actually selected physical Python, not PATH guesses.
            text=text.replace("spawn('python3',","spawn("+json.dumps(manifest['files']['python']['path'])+",")
            if relative=='runtime/material-denial-operator.mjs':
                old="spawnSync('python3',"
                s.need(text.count(old)==1,'original material policy translator interpreter')
                text=text.replace(old,'spawnSync('+json.dumps(manifest['files']['python']['path'])+',')
            if relative=='trusted-owner/trusted-ui-owner-client.mjs':
                old=": 'python3', env);"
                s.need(text.count(old)==1,'original trusted Linux owner interpreter')
                text=text.replace(old,': '+json.dumps('/usr/bin/python3.12')+', env);')
            leftovers=re.findall(r'/workspace/scratch/[A-Za-z0-9_./-]+',text)
            s.need(all(p.startswith((str(composition)+'/',str(operators)+'/',str(output)+'/',
                       str(Path(deps['probe']['directory']))+'/',str(Path(deps['storageWatcher']['path']).parent)+'/',
                       str(Path(deps['toolsClosure']['path']).parent)+'/',str(Path(deps['browser']['path']).parent)+'/',
                       str(Path(deps['playwrightEntry']['path']).parent)+'/')) or p in [str(composition),str(operators/'runtime'),deps['probe']['directory'],manifest['files']['python']['path']]
                       for p in leftovers), 'unported historical scratch dependency')
            entries={('keyboard/owner.mjs'if name=='plain-keyboard'else'runtime/'+script):(index,name) for index,(name,script,_,_,_) in enumerate(JOBS,1) if name!='role-kernel-policy'}
            if relative in entries:
                index,name=entries[relative]; identity_path=str(output/'caller'/'.identities'/(f'{index:02}-'+name+'.node.json'))
                guard=r"""{const f=await import('node:fs'),h=await import('node:crypto'),u=CALLER_UID,p=IDENTITY_PATH;
if(process.env.M6_410_CALLER_IDENTITY_PATH!==p)throw Error('Fixed caller identity path');
const raw=f.readFileSync('/proc/self/status','utf8'),keys=['Uid','Gid','Groups','CapInh','CapPrm','CapEff','CapBnd','CapAmb','NoNewPrivs'],status={};
if(Buffer.byteLength(raw)>65536)throw Error('Bounded caller identity');for(const key of keys){const match=raw.match(new RegExp('^'+key+':[\\t ]*(.*)$','m'));if(!match)throw Error('Missing identity '+key);status[key]=match[1].trim();}
const groups=status.Groups?status.Groups.split(/\s+/).map(Number):[];
if(![process.getuid(),process.geteuid(),process.getgid(),process.getegid()].every(x=>x===u)||groups.length||status.Uid.split(/\s+/).some(x=>Number(x)!==u)||status.Gid.split(/\s+/).some(x=>Number(x)!==u)||keys.slice(3,8).some(k=>BigInt('0x'+status[k])!==0n)||status.NoNewPrivs!=='1')throw Error('Actual nonroot zero-capability NNP caller required');
f.writeFileSync(p,JSON.stringify({pid:process.pid,parentPid:process.ppid,uid:process.getuid(),euid:process.geteuid(),gid:process.getgid(),egid:process.getegid(),groups,nodeGetgroups:process.getgroups(),status,statusSha256:h.createHash('sha256').update(raw).digest('hex'),rawStatus:raw})+'\n',{flag:'wx',mode:0o400});}
""".replace('CALLER_UID',str(s.CALLER_UID)).replace('p=IDENTITY_PATH;','p='+json.dumps(identity_path)+';')
                text=guard+text
            diffs.append({'carrierPath':row['carrierPath'],'originalSha256':row['sha256'],
                          'diff':''.join(difflib.unified_diff(original.decode().splitlines(True),text.splitlines(True),fromfile=row['originalPath'],tofile=str(operators/relative)))})
            emit(relative,text.encode())
        else:
            emit(relative,original)
    selector=s.decode(s.ordinary(template/'oracles/SELECTED19.json'))
    history_roots={}
    for row in catalogue['files']:
        if row['role']=='historical-original-selected-CLI-stream':
            history_roots[str(Path(row['originalPath']).parent)]=str((template/row['carrierPath']).parent)
    for row in selector['rows']:
        original_root=row['captureRoot'];s.need(original_root in history_roots,'all original selected captures transferred')
        row['originalCaptureRoot']=original_root;row['captureRoot']=history_roots[original_root]
    selector.update(integration={'commit':source,'tree':ctx['source']['tree']},
                    transferredHistoricalSelection=s.pin(template/'oracles/SELECTED19.json'),
                    comparisonScope='New integration labels comparison only. All original CLI timestamps/source/owner/failed-batch history remain historical; no new CLI execution.')
    emit('SELECTED19.json',(json.dumps(selector,sort_keys=True,indent=2)+'\n').encode())
    emit('keyboard/context.json',(json.dumps({'source':{'commit':source,'tree':ctx['source']['tree']},
          'prep':str(caller_view),'root':str(result_roots['M6_410_PLAIN_KEYBOARD'])},indent=2)+'\n').encode())
    final_helper = s.ordinary(HERE/'readonly-confirmation.py')
    s.need(s.sha(final_helper) == '0d5d9ceaeddd40f9294bc81e3e35c32ae76e37acf8040d686e34cd577f8ecbb4', 'exact reviewed final read-only helper sibling')
    for name in ['runtime_support.py','run-cohorts.py','readonly-confirmation.py']:
        emit('runner/'+name,s.ordinary(HERE/name))
    # Copy only fixed static operator inputs to readonly caller view, never privileged runner/kernel code.
    for item in list(files):
        src=Path(item['path']); rel=src.relative_to(operators)
        if rel.parts[0]in['runner','kernel']:continue
        target=caller_operators/rel;target.parent.mkdir(parents=True,exist_ok=True,mode=0o755)
        with target.open('xb')as out:out.write(s.ordinary(src))
        target.chmod(0o444);files.append(s.pin(target))
    jobs=[]
    for name,script,prefix,status,seconds in JOBS:
        if name=='plain-keyboard': argv=[manifest['files']['node']['path'],str(caller_operators/'keyboard/owner.mjs'),str(caller_operators/'keyboard/context.json')]
        elif name=='role-kernel-policy': argv=[manifest['files']['python']['path'],'-B',str(operators/'kernel/run-policy-probes.py'),'--candidate-review',ctx['candidateReview']['path']]
        else: argv=[manifest['files']['node']['path'],str(caller_operators/'runtime'/script)]
        jobs.append({'name':name,'argv':argv,'result':str(result_roots[prefix]/'RESULT.json'),'status':status,'seconds':seconds,'role':'trusted-probe'if name=='role-kernel-policy'else'nonroot-caller'})
    s.save(operators/'PORT_DIFFS.json',{'sourceChanges':source_changes,'operatorChanges':diffs});files.append(s.pin(operators/'PORT_DIFFS.json'))
    approvals=[]
    for pair in manifest['approval']['approved']:
        candidate=s.pin(composition/pair['candidate']);inputs.append(candidate)
        approvals.append({**candidate,'path':str(composition/pair['output'])})
    s.save(operators/'BOUND_MANIFEST.json',{'format':'zryna.m6-remote-runtime-binding.v1','version':1,'phase':'qualification',
           'source':ctx['source'],'files':files,'inputs':inputs,'approvals':approvals,'jobs':jobs,
           'interpreters':{key:manifest['files'][key] for key in ['node','python']},'fullQualification':False,
           'caller':{'uid':s.CALLER_UID,'gid':s.CALLER_UID,'supplementaryGroups':[]},
           'callerView':{'root':str(caller_view),'approvals':[{k:row[k] for k in['canonical','view']}for row in view['approvals']]},
           'finalConfirmation':s.pin(operators/'runner'/'readonly-confirmation.py'),
           'provenance':'Corrected mature18/keyboardRUN6/pipeRUN4; historical CLI19 and trusted probe remain explicit old immutable inputs, never new source execution evidence.'})
    return s.pin(operators/'BOUND_MANIFEST.json')


if __name__=='__main__':
    s.need(len(sys.argv)==3,'bind-runtime.py ACTUAL_CONTEXT ACTUAL_MANIFEST')
    print(json.dumps(bind_runtime(sys.argv[1],sys.argv[2])))
