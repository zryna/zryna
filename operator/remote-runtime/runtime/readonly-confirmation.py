"""Post-thirteen operator metadata only: GET requests, no guest, audit or review authority."""
import datetime
import importlib.util
import os
from pathlib import Path
import re
import sys

spec=importlib.util.spec_from_file_location('final_confirmation_support',Path(__file__).with_name('runtime_support.py'))
s=importlib.util.module_from_spec(spec)
spec.loader.exec_module(s)


def identical(actual,expected):
    s.need(type(actual) is type(expected),'same JSON value type')
    if type(actual) is dict:
        s.need(set(actual)==set(expected),'same JSON object keys')
        for key in expected:identical(actual[key],expected[key])
    elif type(actual) is list:
        s.need(len(actual)==len(expected),'same JSON list size')
        for a,b in zip(actual,expected):identical(a,b)
    else:s.need(actual==expected,'same JSON value')


def confirm(manifest_path):
    manifest=s.decode(s.ordinary(manifest_path));root=Path(manifest['approval']['root'])
    ctx,manifest,root=s.context_inputs(root/'REMOTE_RUNTIME_CONTEXT.json',manifest_path)
    s.need(manifest['approval']['phase']=='qualification','qualification-only post-runtime metadata')
    operators=Path(ctx['runtime']['operatorRoot']);bound=s.decode(s.ordinary(operators/'BOUND_MANIFEST.json'))
    s.need(bound['source']==ctx['source'] and bound['phase']=='qualification','exact actual binding')
    s.checked(bound['finalConfirmation']);s.need(Path(bound['finalConfirmation']['path'])==Path(__file__).absolute(),'exact pinned final helper')
    journal=Path(ctx['runtime']['outputRoot'])/'execution';proofs=[]
    names=['parity','plain-keyboard','original-ui-owner','http-boundaries','session-isolation','browser-faults',
           'compiler-owner-faults','compiler-resources','pipe-boundaries','storage-boundaries','material-denials',
           'role-kernel-policy','host-death-and-owned-recovery']
    s.need([j['name'] for j in bound['jobs']]==names,'exact original thirteen jobs')
    for index,job in enumerate(bound['jobs'],1):
        path=journal/(f'{index:02}-'+job['name']+'.COMPLETE.json');row=s.decode(s.ordinary(path))
        s.need(row['name']==job['name'] and row['argv']==job['argv'] and row['owningCommandPassed'] is True and
               row['exitCode']==0 and not row.get('timedOut') and row['observedStatus']==job['status'],
               'all thirteen owning commands actually passed before any GET')
        s.need(s.result_pin(row['result']['path'])==row['result'],'whole actual output result bytes unchanged');s.need(row['result']['path']==job['result'],'actual expected original result')
        s.need(s.decode(s.result_read(job['result']))['status']==job['status'],'actual original status retained')
        for key in ['stdout','stderr']:s.checked(row[key])
        proofs.append(s.pin(path))
    prepared=s.decode(s.ordinary(root/'PREPARATION.json'));inventory={r['path']:r for r in prepared['files']}
    def approved_pin(path):
        relative=str(Path(path).relative_to(root));p=s.pin(path);row=inventory[relative]
        s.need((p['bytes'],p['sha256'])==(row['bytes'],row['sha256']),'current bytes match actual approved preparation');return p
    actual=s.decode(s.ordinary(root/'RESULT.json'))
    s.need(actual['status']=='prepared-unapproved' and actual['source']==ctx['source'],'actual preparation baseline remains unapproved metadata')
    candidate=s.decode(s.ordinary(ctx['candidateReview']['path']))
    s.need(candidate['status']=='official-two-replica-candidate-acquired-and-reassembled' and candidate['source']=={k:ctx['source'][k] for k in ['commit','tree']},'actual candidate source')
    reviewed=s.decode(s.ordinary(root/'reviewed-launcher.candidate.json'))
    selected_binary=s.pin(candidate['compilerPath'])
    s.need(all(selected_binary[k]==candidate['compiler'][k] for k in ['bytes','sha256']),'actual selected compiler binary remains exact')
    compiler=approved_pin(root/'compiler.bin')
    s.need(all(compiler[k]==candidate['compiler'][k]==reviewed['compiler'][k] for k in ['bytes','sha256']),'actual whole candidate compiler remains exact')
    files={name:approved_pin(root/name) for name in ['reviewed-launcher.candidate.json','source-composition.json','complete-policy.candidate.json']}
    for pair in manifest['approval']['approved']:
        selected=approved_pin(root/pair['candidate']);p=s.pin(root/pair['output'])
        s.need((p['bytes'],p['sha256'])==(selected['bytes'],selected['sha256']) and
               (root/pair['output']).stat().st_mode&0o777==0o400,'actual approval is unchanged exact0400 candidate bytes')
        files[pair['output']]=p
    s.need({k:reviewed['source'][k] for k in ['commit','tree']}=={k:ctx['source'][k] for k in ['commit','tree']},'same actual reviewed source')
    s.need(reviewed['compiler']['path']==str(root/'compiler.bin'),'same actual retained candidate path')
    ui_approval=s.decode(s.ordinary(root/'ui-approval.json'))
    approval=s.decode(s.ordinary(root/'approval.json'));s.need(approval['engine_uid']==65534 and approval['cpus']==[0,1],'unchanged peer and two approved CPUs')
    s.need(approval['composition_sha256']==reviewed['compositionSha256'],'same approved source composition')
    s.git_source(ctx['source'])
    result_root=Path(ctx['runtime']['outputRoot'])/'final-confirmation';result_root.mkdir(mode=0o700)
    started=datetime.datetime.now(datetime.timezone.utc).isoformat()
    inputs=[s.pin(manifest_path),s.pin(root/'REMOTE_RUNTIME_CONTEXT.json'),ctx['candidateReview'],ctx['frozenMain'],
            s.pin(root/'PREPARATION.json'),s.pin(operators/'BOUND_MANIFEST.json'),approved_pin(root/'RESULT.json'),selected_binary,compiler,*files.values(),*proofs]
    state={'format':'zryna.m6-operator-final-readonly-confirmation.v1','version':1,'source':ctx['source'],
           'startedAt':started,'engineMutations':0,'guestExecutions':0,'fullQualification':False,'independentReview':False,
           'inputs':inputs,'compiler':{k:compiler[k] for k in ['bytes','sha256']},'approvedFiles':files,
           'scope':'Trusted preparer/runtime-owner GET-only final metadata. No new full material audit, guest cleanup action, volume creation or independent68 review is asserted.'}
    try:
        sys.path.insert(0,ctx['source']['path']+'/examples/playground/restricted/daemon/preparation')
        from engine import Engine,decode
        from install import volume_identity
        engine=Engine(65534,seconds=30);raws=[];state['requests']=raws
        def get(name,path):
            raw,_=engine.request('GET',path,statuses=(200,))
            target=result_root/(name+'.raw.json')
            with target.open('xb') as out:out.write(raw)
            target.chmod(0o400);raws.append({'method':'GET','path':path,'status':200,'body':s.pin(target)})
            return decode(raw)
        images={}
        for role in ['compiler','evaluator','ui']:
            image=actual['images'][role]['image'];s.need(re.fullmatch('sha256:[a-f0-9]{64}',image),'exact actual image ID')
            if role=='ui':s.need(ui_approval['image']==image,'same exact approved UI image')
            else:s.need(approval[role+'_image_sha256']==image.removeprefix('sha256:'),'same exact approved native-role image')
            baseline_path=root/(role+'-IMAGE_INSPECT.json');inputs.append(approved_pin(baseline_path))
            before=s.decode(s.ordinary(baseline_path));current=get(role+'-IMAGE_INSPECT','/images/'+image+'/json')
            s.need(current['Id']==before['Id']==image,'same actual image identity')
            for key in ['Id','Config','RootFS','Os','Architecture']:identical(current[key],before[key])
            s.need(current['Os']=='linux' and current['Architecture']=='amd64','original selected image platform')
            images[role]={key:current[key] for key in ['Id','Config','RootFS','Os','Architecture']}
        materials={};sealed=[]
        for role in ['compiler','evaluator']:
            selected=actual['materials'][role];name=selected['name'];s.need(re.fullmatch('zryna410-materials-'+role+'-[a-f0-9]{32}',name),'exact fresh sealed material name')
            s.need(selected['failure'] is None and selected['holderRemoved'] is True,'original population success remains explicit')
            expected=approval[role+'_materials'];s.need(all(selected[k]==expected[k] for k in ['name','created_at','nonce','root_sha256','files','bytes']),'same approved six-field material binding')
            baseline_path=root/(role+'-VOLUME_INSPECT.json');inputs.append(approved_pin(baseline_path));before=s.decode(s.ordinary(baseline_path))
            current=get(role+'-VOLUME_INSPECT','/volumes/'+name)
            volume_identity(current,name,selected['nonce'],selected['root_sha256'],selected['created_at'])
            for key in ['Name','CreatedAt','Driver','Scope','Labels','Options','Mountpoint']:identical(current[key],before[key])
            materials[role]=current;sealed.append(name)
        containers=get('CONTAINERS_ALL','/containers/json?all=1');s.need(type(containers) is list and containers==[],'no current containers on task-private Engine')
        volumes=get('VOLUMES_ALL','/volumes');s.need(type(volumes) is dict and type(volumes.get('Volumes')) is list,'whole actual global volume list')
        rows=volumes['Volumes'];s.need(len(rows)==2 and sorted(row['Name'] for row in rows)==sorted(sealed),'exactly two sealed volumes; no task-private volumes remain')
        for role in ['compiler','evaluator']:
            current=next(row for row in rows if row['Name']==materials[role]['Name'])
            for key in ['Name','CreatedAt','Driver','Scope','Labels','Options','Mountpoint']:identical(current[key],materials[role][key])
        s.git_source(ctx['source']);s.need(s.pin(root/'compiler.bin')==compiler,'actual compiler unchanged after final GETs')
        for name,p in files.items():s.need(s.pin(root/name)==p,'actual approval/composition unchanged after GETs')
        state.update(status='actual-post-thirteen-readonly-metadata-confirmed',sourceCleanConfirmed=True,currentImages=images,currentSealedVolumeMetadata=materials,
                     currentContainerList=containers,currentVolumeList=volumes,requests=raws,
                     mandatoryRuntimeAudits='Per-operation mandatory full audits remain original native evidence. Final GET metadata does not perform or claim a fresh full material audit.')
        return_path=result_root/'RESULT.json'
        state['finishedAt']=datetime.datetime.now(datetime.timezone.utc).isoformat();s.save(return_path,state);return s.pin(return_path)
    except BaseException as error:
        state.update(status='final-readonly-confirmation-failed',failure={'type':type(error).__name__,'message':str(error)},
                     finishedAt=datetime.datetime.now(datetime.timezone.utc).isoformat())
        s.save(result_root/'FAILURE.json',state);raise
