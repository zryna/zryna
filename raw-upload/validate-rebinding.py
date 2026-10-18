"""Read-only local source/pin cross-check. Never uploads, edits a plan or grants qualification."""
import argparse, hashlib, json, pathlib, re, tarfile

REQUIRED=['frozenMain','fullCiAdmission','candidateReview','nativeReview','installedSupplementReview','editorSetupReview','readerReview','documentationReview','compilerRuntimePlan','compilerImageMetadata','evaluatorImageMetadata','compilerImageExport','evaluatorImageExport','durablePins','uploadPlan','transportPlan','rawDraft']
BASELINE='5d79dba18117a7ff41ff08d328aa94838c0fce9c'
IMMUTABLE_REUSE={'compiler-materials','evaluator-materials','compiler-materials-archive','evaluator-materials-archive','daemon-supervisor-linux','compiler-release','node-runtime','provider','chrome-linux','chrome-linux-inventory','legacy-release-envelope','legacy-release-signature'}
sha=lambda b:hashlib.sha256(b).hexdigest()
def need(value,reason):
 if not value:raise ValueError(reason)
def hashfile(path):
 h=hashlib.sha256();n=0
 with pathlib.Path(path).open('rb') as f:
  while b:=f.read(65536):h.update(b);n+=len(b)
 return n,h.hexdigest()
def read(pin):
 need(isinstance(pin,dict) and set(pin)=={'path','bytes','sha256'},'complete actual input file pin required')
 need(type(pin['bytes']) is int and 0<pin['bytes']<=2*1024*1024 and re.fullmatch('[a-f0-9]{64}',pin['sha256']),'bounded pinned JSON input')
 p=pathlib.Path(pin['path']);need(p.is_absolute() and p.is_file() and not p.is_symlink(),'ordinary actual pinned input')
 need(hashfile(p)==(pin['bytes'],pin['sha256']),'actual input digest: '+p.name)
 return json.loads(p.read_bytes())
def source(row,commit,tree,label):need(row.get('commit')==commit and row.get('tree')==tree,label+' must select actual new integration')

def validate(selection):
 missing=[key for key in REQUIRED if not selection.get('inputs',{}).get(key)]
 if missing:return {'status':'pending-actual-inputs','missing':missing,'admitted':False}
 data={key:read(selection['inputs'][key]) for key in REQUIRED if key not in ['compilerImageExport','evaluatorImageExport']}
 frozen=data['frozenMain'];commit,tree=frozen['commit'],frozen['tree']
 need(re.fullmatch('[a-f0-9]{40}',commit) and re.fullmatch('[a-f0-9]{40}',tree) and commit!=BASELINE,'known bad baseline is not a new integration')
 need(frozen['status']=='actual-reviewed-publication-fix-merged-and-new-main-frozen','actual parent freeze required')
 ci=data['fullCiAdmission'];source(ci['source'],commit,tree,'full CI');source(ci['main'],commit,tree,'protected main')
 need(ci['ci']['headSha']==commit and ci['ci']['headTree']==tree and ci['ci']['conclusion']=='success','actual successful exact-source CI admission')
 candidate=data['candidateReview'];source(candidate['source'],commit,tree,'compiler candidate')
 need(candidate['status']=='official-two-replica-candidate-acquired-and-reassembled','actual independently reassembled candidate required')
 need(hashfile(candidate['compilerPath'])==(candidate['compiler']['bytes'],candidate['compiler']['sha256']),'actual selected compiler binary')
 native=data['nativeReview'];need(native['sourceMain']==commit and native['tree']==tree,'native execution source cannot be relabeled')
 for platform in ['LINUX','WINDOWS']:
  supplement=data['installedSupplementReview'][platform];source(supplement['source'],commit,tree,'installed supplement '+platform)
  need(supplement['fullMainCiRun']==ci['ci']['runId'],'installed supplement same full CI')
  source(data['editorSetupReview'][platform]['source'],commit,tree,'installed editor/setup '+platform)
 reader=data['readerReview'];source(reader['source'],commit,tree,'native read-only input corpus')
 need(reader['status']=='both-native-readonly-captures-independently-reviewed','actual native reader review')
 docs=data['documentationReview'];source(docs['expected']['source'],commit,tree,'authenticated documentation')
 need(docs['status'] in ['official-docs-authenticated-and-source-bundle-identical','official-docs-authenticated-and-vendored-bytes-identical'],'original authenticated docs review required')
 runtime_plan=data['compilerRuntimePlan'];app=next(x for x in runtime_plan['files'] if x['path']=='app/compiler')
 need((app['bytes'],app['sha256'])==(candidate['compiler']['bytes'],candidate['compiler']['sha256']),'runtime image plan must contain actual newly selected candidate')
 image_proofs=[]
 for role in ['compiler','evaluator']:
  metadata=data[role+'ImageMetadata'];export_pin=selection['inputs'][role+'ImageExport'];need(hashfile(export_pin['path'])==(export_pin['bytes'],export_pin['sha256']),'actual image export identity')
  need(export_pin['bytes']<=128*1024*1024,'bounded inert image export')
  with tarfile.open(export_pin['path'],'r:') as archive:
   members=archive.getmembers();need(len(members)<=512 and sum(x.size for x in members)<=128*1024*1024,'bounded image archive')
   need(all(x.isfile() or x.isdir() for x in members),'image ordinary topology')
   names=[x.name for x in members];need(len(set(names))==len(names),'no duplicate image members')
   need(all(not n.startswith('/') and all(c not in ['', '.', '..'] for c in n.split('/')) for n in names),'canonical image members')
   by={x.name:x for x in members}
   for member in members:
    if member.isfile() and member.name.startswith('blobs/sha256/'):
     need(sha(archive.extractfile(member).read())==member.name.rsplit('/',1)[1],'every image content address')
   config_name='blobs/sha256/'+metadata['image'].removeprefix('sha256:')
   config=json.load(archive.extractfile(by[config_name]))
   need(config['config']==metadata['config'] and config['rootfs']['diff_ids']==metadata['rootFs']['Layers'],'actual approved runtime image config/rootFS')
   image_proofs.append({'role':role,'configSha256':metadata['image'].removeprefix('sha256:'),'exportSha256':export_pin['sha256']})
 pins=data['durablePins'];roles={x['role']:x for x in pins};need(len(roles)==len(pins),'unique actual durable roles')
 for platform in ['linux','windows']:
  installed=data['editorSetupReview'][platform.upper()]
  need(roles['server-'+platform]['sha256']==installed['serverSha256'],'actual platform server variant')
  extension=roles['extension' if platform=='linux' else 'extension-windows'];need(extension['sha256']==installed['vsixSha256'],'actual platform installed VSIX; no cross-OS byte substitution')
  need(roles['setup-manifest-'+platform]['sha256']==installed['manifestSha256'],'actual same-source setup manifest')
 need(roles['docs-bundle']['sha256']==docs['expected']['manifestSha256'],'docs role is actual manifest, never whole artifact ZIP')
 for role in ['server-linux','server-windows','extension','extension-windows','setup-linux','setup-windows','setup-manifest-linux','setup-manifest-windows','docs-bundle','durable-material-provenance','original-acquisition-provenance']:
  need(roles[role]['source']['kind']=='git' and roles[role]['source']['revision']==commit+':'+tree,'fresh source-bound material provenance: '+role)
 reuse={x['role']:x for x in selection.get('componentReuse',[])}
 for role,pin in roles.items():
  if pin['source']['kind']=='git' and pin['source']['revision'].startswith(BASELINE+':'):
   need(role in IMMUTABLE_REUSE and role in reuse,'old source artifact requires explicit immutable-component reuse: '+role)
   row=reuse[role];need(row.get('sha256')==pin['sha256'] and row.get('bytes')==pin['bytes'] and row.get('originalSourceRevision')==pin['source']['revision'] and row.get('reason'),'honest unchanged component identity/provenance: '+role)
 plan=data['transportPlan'];uploads=data['uploadPlan'];draft=data['rawDraft']
 need(plan['sourceCommit']==commit and plan['sourceTree']==tree,'whole transport plan new source')
 need(plan['tag']=='m6-raw-evidence-'+commit+'-1' and plan['tag']==uploads['tag'],'new exact raw tag')
 need(draft['id']==plan['releaseId'] and draft['draft'] is True and draft.get('immutable') is not True and draft['tag_name']==plan['tag'] and draft['target_commitish']==commit,'actual new private draft target/id')
 need(plan['releaseId']!=416355512,'old private baseline draft cannot be adopted as new source')
 outputs={x['name']:x for x in uploads['assets']};need(len(outputs)==len(uploads['assets']),'closed unique upload outputs')
 need({x['name']:(x['bytes'],x['sha256']) for x in plan['assets']}=={n:(x['bytes'],x['sha256']) for n,x in outputs.items()},'complete transport and durable-upload pins must agree')
 for name,row in outputs.items():
  need(hashfile(row['localPath'])==(row['bytes'],row['sha256']),'actual local upload byte identity '+name)
  need(row['futureUrl']=='https://github.com/zryna/zryna/releases/download/'+plan['tag']+'/'+name,'actual new retention URL selection')
 for name,artifact in plan['sourceArtifacts'].items():
  metadata_pin=selection.get('originalMetadata',{}).get(name);need(metadata_pin,'actual original metadata input required: '+name);metadata=read(metadata_pin)
  if 'id' in artifact:
   need(metadata['id']==artifact['id'] and metadata['digest']=='sha256:'+artifact['sha256'] and metadata['size_in_bytes']==artifact['bytes'] and not metadata['expired'] and metadata['workflow_run']==artifact['workflowRun'],'original artifact id/hash/head remains authentic: '+name)
   if name.startswith(('setup-artifact-','editor-artifact-')):head,run=commit,ci['ci']['runId']
   elif name.startswith('native-reader-'):head,run=reader['operator'],reader['runId']
   elif name.startswith('native-'):head,run=native['workflowOperator'],native['run']
   elif name.startswith('installed-supplement-'):
    platform='WINDOWS' if '-windows-' in name else 'LINUX';row=data['installedSupplementReview'][platform];head,run=row['operator'],row['actualHostedRun']
   elif name.startswith('docs-'):head,run=commit,docs['expected']['docsRunId']
   else:raise ValueError('unreviewed original artifact family '+name)
   need(metadata['workflow_run']['head_sha']==head and metadata['workflow_run']['id']==run,'owning workflow head/run is distinct from selected payload source: '+name)
  else:need(metadata['id']==artifact['jobId'] and metadata['run_id']==ci['ci']['runId'] and metadata['conclusion']=='success','actual newly selected canonical shared-job identity')
 return {'status':'local-source-and-material-bindings-consistent-pending-separate-operator-review','admitted':False,'source':{'commit':commit,'tree':tree},'ciRun':ci['ci']['runId'],'candidateRun':candidate['runId'],'compiler':candidate['compiler'],'images':image_proofs,'outputs':len(outputs),'reuseRoles':sorted(reuse),'scope':'Read-only local consistency check only. Original authenticators, actual runtime qualification, independent operator review and real upload/readback remain required. No opaque capability or publication approval granted.'}

def main():
 parser=argparse.ArgumentParser();parser.add_argument('selection');parser.add_argument('--output',required=True);args=parser.parse_args()
 selection=json.loads(pathlib.Path(args.selection).read_bytes())
 try:result=validate(selection)
 except Exception as error:result={'status':'rejected-source-or-material-binding','admitted':False,'error':str(error)}
 with pathlib.Path(args.output).open('x') as out:json.dump(result,out,indent=2);out.write('\n')
 print(json.dumps(result));raise SystemExit(1 if result['status'].startswith('rejected') else 0)

if __name__=='__main__':main()
