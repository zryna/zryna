"""Manifest-bound fixed draft-only inert raw asset transport. No publication or guest execution."""
import argparse, base64, gzip, hashlib, http.client, io, json, os, pathlib, re, shutil, stat, subprocess, sys, tarfile, time, urllib.parse, zlib

HERE=pathlib.Path(__file__).resolve().parent
PLAN_RAW=(HERE/'plan.json').read_bytes()
assert 1<=len(PLAN_RAW)<=1048576
PLAN=json.loads(PLAN_RAW)
# Source/release selectors belong to the complete separately reviewed manifest;
# its exact SHA must also be bound by the manual operator dispatch.
PINNED_RELEASE=PLAN['releaseId'];PINNED_TAG=PLAN['tag'];PINNED_SOURCE=PLAN['sourceCommit']
assert type(PINNED_RELEASE) is int and 0<PINNED_RELEASE<10**15
assert re.fullmatch('[a-f0-9]{40}',PINNED_SOURCE) and re.fullmatch('[a-f0-9]{40}',PLAN['sourceTree'])
assert PINNED_TAG=='m6-raw-evidence-'+PINNED_SOURCE+'-1' and PLAN['repository']=='zryna/zryna'
assert 1<=len(PLAN['assets'])<=64 and len({p['name'] for p in PLAN['assets']})==len(PLAN['assets'])
for p in PLAN['assets']:
 assert re.fullmatch('[A-Za-z0-9][A-Za-z0-9._-]{0,127}',p['name'])
 assert type(p['bytes']) is int and 0<p['bytes']<=2147483648 and re.fullmatch('[a-f0-9]{64}',p['sha256'])
for b in PLAN['blobs']:
 assert b['path'].startswith(('payload/','lib/')) and all(x not in ['','..','.'] for x in b['path'].split('/'))
 assert b['outputName'] is None or b['outputName'] in {p['name'] for p in PLAN['assets']}
PINS={p['name']:p for p in PLAN['assets']}
sha=lambda b:hashlib.sha256(b).hexdigest()
DEADLINE=time.monotonic()+2400
TOKEN=None
LOCAL=None
ROOT=None

def check(condition,message):
 if not condition:raise RuntimeError(message)
def hashfile(path):
 h=hashlib.sha256();n=0
 with pathlib.Path(path).open('rb') as stream:
  while block:=stream.read(65536):h.update(block);n+=len(block)
 return n,h.hexdigest()
def verify(path,pin):check(hashfile(path)==(pin['bytes'],pin['sha256']),'asset identity mismatch: '+pathlib.Path(path).name)
def save(name,value):
 with (ROOT/name).open('x') as stream:json.dump(value,stream,indent=2);stream.write('\n')
def guard():check(time.monotonic()<DEADLINE,'whole fixed transport deadline exceeded')

def connection(url,method='GET',*,accept='application/vnd.github+json',body=None,length=None,auth=False):
 guard();u=urllib.parse.urlsplit(url)
 check(u.scheme=='https' and not u.username and not u.password and u.port in [None,443] and not u.fragment,'HTTPS transport required')
 host=u.hostname
 allowed=host in ['api.github.com','uploads.github.com','release-assets.githubusercontent.com','storage.googleapis.com','registry.npmjs.org'] or host.endswith('.blob.core.windows.net') or host.endswith('.actions.githubusercontent.com')
 check(allowed,'unapproved transport host')
 check(not auth or host in ['api.github.com','uploads.github.com'],'credential boundary')
 check(method=='GET' or method=='POST' and host=='uploads.github.com' and u.path==f'/repos/zryna/zryna/releases/{PINNED_RELEASE}/assets','closed read-or-draft-asset API only')
 check(method=='GET' or urllib.parse.parse_qs(u.query).get('name',[None])[0] in PINS,'closed output asset only')
 c=http.client.HTTPSConnection(host,timeout=min(120,max(1,int(DEADLINE-time.monotonic()))))
 headers={'Accept':accept,'User-Agent':'zryna-m6-fixed-draft-transport-v1','X-GitHub-Api-Version':'2022-11-28','Connection':'close'}
 if auth:check(TOKEN is not None,'Actions credential required');headers['Authorization']='Bearer '+TOKEN
 if body is not None:
  headers['Content-Type']='application/octet-stream';headers['Content-Length']=str(length)
 try:
  c.request(method,u.path+('?' +u.query if u.query else ''),body=body,headers=headers)
  return c,c.getresponse()
 except BaseException:
  c.close();raise

def api(path):
 check(path.startswith('/repos/zryna/zryna/'),'fixed repository API required')
 c,r=connection('https://api.github.com'+path,auth=True)
 try:
  check(r.status==200,'GitHub metadata HTTP '+str(r.status));raw=r.read(5*1024*1024+1);check(len(raw)<=5*1024*1024,'bounded GitHub JSON');return json.loads(raw)
 finally:r.close();c.close()

def download(url,out,pin,*,auth=False,local_key=None,algorithm='sha256'):
 check(not out.exists(),'create-only acquisition')
 if LOCAL is not None:
  check(local_key in LOCAL,'missing fixed local source '+str(local_key));source=pathlib.Path(LOCAL[local_key])
  with source.open('rb') as i,out.open('xb') as o:shutil.copyfileobj(i,o,65536)
 else:
  c,r=connection(url,accept='application/octet-stream',auth=auth)
  try:
   if r.status in [301,302,303,307,308]:
    target=r.getheader('Location');check(target is not None,'missing bounded redirect');r.close();c.close()
    c,r=connection(target,accept='application/octet-stream',auth=False)
   check(r.status==200,'asset acquisition HTTP '+str(r.status))
   bound=pin.get('bytes',8*1024*1024)
   n=0
   with out.open('xb') as stream:
    while b:=r.read(65536):guard();n+=len(b);check(n<=bound,'download first extra byte');stream.write(b)
  finally:r.close();c.close()
 if algorithm=='sha256':verify(out,pin)
 else:check(hashlib.sha512(out.read_bytes()).hexdigest()==pin['sha512'],'npm archive SHA512 mismatch')
 return out

def copy_checked(source,out,pin):
 check(not out.exists(),'create-only material output')
 with source.open('rb') as i,out.open('xb') as o:shutil.copyfileobj(i,o,65536)
 verify(out,pin)

def zip_read(path,suffix,maximum):
 import zipfile
 with zipfile.ZipFile(path) as a:
  matches=[i for i in a.infolist() if (i.filename==suffix.removeprefix('/') or i.filename.endswith(suffix)) and not i.is_dir()]
  check(len(matches)==1,'unique pinned ZIP member required')
  member=matches[0];check(member.file_size<=maximum and member.flag_bits&1==0,'bounded ordinary ZIP member')
  mode=member.external_attr>>16;check(stat.S_IFMT(mode) in [0,stat.S_IFREG],'ordinary ZIP member type')
  raw=a.read(member);check(len(raw)==member.file_size,'member length');return raw

def put_checked(out,raw,pin):
 check(len(raw)==pin['bytes'] and sha(raw)==pin['sha256'],'selected member byte identity')
 with out.open('xb') as stream:stream.write(raw)

def tar_member_map(path):
 a=tarfile.open(path,'r:gz');members=a.getmembers();check(len(members)<=1024,'finite source archive members')
 check(bool(members),'nonempty source archive');root=members[0].name.split('/')[0]
 seen=set()
 for m in members:
  check(m.name not in seen and not m.name.startswith('/') and '..' not in m.name.split('/'),'bounded canonical source member')
  check(m.name==root or m.name.startswith(root+'/'),'one exact source archive root')
  check(m.isdir() or m.isfile(),'ordinary source tar topology');seen.add(m.name)
 return a,members

def tar_read(a,members,rel,pin):
 root=members[0].name.split('/')[0]
 matches=[m for m in members if m.name==root+'/'+rel and m.isfile()]
 check(len(matches)==1 and matches[0].size==pin['bytes'],'unique bounded source tar member')
 raw=a.extractfile(matches[0]).read(pin['bytes']+1)
 check(len(raw)==pin['bytes'] and sha(raw)==pin['sha256'],'source tar member digest');return raw

def small_tar(out,entries):
 with out.open('xb') as raw:
  with gzip.GzipFile(fileobj=raw,filename='',mtime=0,mode='wb',compresslevel=6) as compressed:
   with tarfile.open(fileobj=compressed,mode='w|',format=tarfile.USTAR_FORMAT) as archive:
    for rel,path,mode in entries:
     n,_=hashfile(path);m=tarfile.TarInfo(rel);m.size=n;m.mode=mode;m.uid=m.gid=m.mtime=0
     with path.open('rb') as stream:archive.addfile(m,stream)

def prepare():
 assets=ROOT/'assets';assets.mkdir();acquired=ROOT/'acquired';acquired.mkdir()
 # Review all Git-delivered small bytes and image transports before use.
 for blob in PLAN['blobs']:
  path=HERE/blob['path'];verify(path,blob)
  if blob['kind'] in ['compression-library','compression-license']:continue
  out=assets/blob['outputName']
  if blob['kind']=='copy':copy_checked(path,out,PINS[out.name])
  elif blob['kind']=='gunzip':
   n=0
   with gzip.open(path,'rb') as i,out.open('xb') as o:
    while b:=i.read(65536):n+=len(b);check(n<=PINS[out.name]['bytes'],'image first extra byte');o.write(b)
   verify(out,PINS[out.name])
  else:raise RuntimeError('closed blob kind')
 check(zlib.ZLIB_RUNTIME_VERSION=='1.3.1','fixed inert compressor library version')
 # Whole original archives, never reconstructed from inner files.
 for name,source in PLAN['sourceArtifacts'].items():
  if 'id' in source:
   if LOCAL is None:
    m=api('/repos/zryna/zryna/actions/artifacts/'+str(source['id']))
    check(m['id']==source['id'] and m['name']==source['name'] and not m['expired'] and m['size_in_bytes']==source['bytes'] and m['digest']=='sha256:'+source['sha256'],'original Actions artifact metadata')
    check(m['workflow_run']==source['workflowRun'],'original Actions artifact workflow identity')
   download('https://api.github.com/repos/zryna/zryna/actions/artifacts/'+str(source['id'])+'/zip',assets/name,source,auth=True,local_key=str(source['id']))
  else:
   if LOCAL is None:
    m=api('/repos/zryna/zryna/actions/jobs/'+str(source['jobId']));check(m['run_id']==source['runId'] and m['conclusion']=='success','owning shared-job identity')
   download('https://api.github.com/repos/zryna/zryna/actions/jobs/'+str(source['jobId'])+'/logs',assets/name,source,auth=True,local_key='job-'+str(source['jobId']))
 # Selected setup payloads retain their actual original container/archive bytes.
 for platform in ['linux','windows']:
  setup_name='zryna-setup-0.1.0-candidate.3-x86_64-'+('unknown-linux-gnu.tar.gz' if platform=='linux' else 'pc-windows-msvc.zip')
  put_checked(assets/setup_name,zip_read(assets/('setup-artifact-'+platform+'-original.zip'),'/'+setup_name,PINS[setup_name]['bytes']),PINS[setup_name])
  if platform=='linux':
   a,ms=tar_member_map(assets/setup_name)
   for rel,name in [('setup.json','setup-manifest-linux.json'),('bin/zryna-language-server','zryna-language-server-linux'),('editor/zryna-0.5.0.vsix','zryna-0.5.0-linux.vsix')]:put_checked(assets/name,tar_read(a,ms,rel,PINS[name]),PINS[name])
   a.close()
  else:
   for rel,name in [('setup.json','setup-manifest-windows.json'),('bin/zryna-language-server.exe','zryna-language-server-windows.exe'),('editor/zryna-0.5.0.vsix','zryna-0.5.0-windows.vsix')]:put_checked(assets/name,zip_read(assets/setup_name,'/'+rel,PINS[name]['bytes']),PINS[name])
 put_checked(assets/'authenticated-docs-manifest.json',zip_read(assets/'docs-artifact-original.zip','/manifest.json',PINS['authenticated-docs-manifest.json']['bytes']),PINS['authenticated-docs-manifest.json'])
 legacy_pin=PLAN['legacy']
 if LOCAL is None:
  m=api('/repos/zryna/zryna/releases/'+str(legacy_pin['immutableReleaseId']));check(m['immutable'] is True and not m['draft'] and m['tag_name']==legacy_pin['tag'],'original immutable legacy release')
  subject=next(x for x in m['assets'] if x['id']==legacy_pin['id']);check(subject['digest']=='sha256:'+legacy_pin['sha256'] and subject['size']==legacy_pin['bytes'],'original legacy subject')
 legacy=download('https://api.github.com/repos/zryna/zryna/releases/assets/'+str(legacy_pin['id']),acquired/'legacy.tar.gz',legacy_pin,auth=True,local_key='legacy')
 chrome=download(PLAN['chrome']['url'],assets/'chrome-linux64-153.0.8010.12.zip',PLAN['chrome'],local_key='chrome')
 playwright=download(PLAN['playwright']['url'],acquired/'playwright.tgz',PLAN['playwright'],local_key='playwright',algorithm='sha512')
 la,lm=tar_member_map(legacy);pa,pm=tar_member_map(playwright)
 import zipfile
 ca=zipfile.ZipFile(chrome);check(len(ca.infolist())<=512,'bounded original browser archive')
 source_maps={}
 for role in ['compiler','evaluator']:
  manifest=json.loads((assets/(role+'-materials-manifest.json')).read_bytes());root=ROOT/('sources-'+role);root.mkdir()
  for item in manifest['files']:
   rel=item['path'];out=root/rel;out.parent.mkdir(parents=True,exist_ok=True)
   if rel.startswith('browser/'):
    name=rel.removeprefix('browser/');m=ca.getinfo(name);check(m.file_size==item['bytes'] and not m.flag_bits&1,'pinned browser member length');raw=ca.read(m)
   elif rel.startswith('playwright/'):
    raw=tar_read(pa,pm,rel.removeprefix('playwright/'),item)
   else:raw=tar_read(la,lm,rel,item)
   put_checked(out,raw,item);out.chmod(0o500 if item['mode']&0o111 else 0o400)
  source_maps[role]=(root,manifest)
  document={'format':'zryna.playground-preparation-plan.v1','version':1,'kind':'materials','role':role,'directories':manifest['directories'],'files':[{**item,'sourcePath':str(root/item['path'])} for item in manifest['files']],'bytes':sum(x['bytes'] for x in manifest['files']),'config':None}
  raw_plan=json.dumps(document,separators=(',',':'),ensure_ascii=True).encode();plan_path=ROOT/(role+'-relocated-emission-plan.json');plan_path.write_bytes(raw_plan)
  sys.path.insert(0,str(HERE.parent/'examples/playground/restricted/daemon/preparation'))
  from emit import run
  out=assets/(role+'-materials.tar.gz')
  with out.open('xb') as raw_out:
   with gzip.GzipFile(fileobj=raw_out,filename='',mtime=0,mode='wb',compresslevel=6) as compressed:receipt=run(str(plan_path),len(raw_plan),sha(raw_plan),compressed)
  original=PLAN['materialTarPins'][role]
  check(receipt['archiveBytes']==original['archiveBytes'] and receipt['archiveSha256']==original['archiveSha256'],'original deterministic material tar identity')
  verify(out,PINS[out.name]);save(role+'-actual-emission.json',receipt)
 la.close();pa.close();ca.close()
 root,manifest=source_maps['compiler']
 entries=[(x,root/x,0o555 if x.endswith('/node') else 0o444) for x in ['runtime/node/bin/node','licenses/node-LICENSE']]
 small_tar(assets/'node-22.22.1-linux-x64.tar.gz',entries)
 entries=[(x['path'],root/x['path'],x['mode']) for x in manifest['files'] if x['path'].startswith('lib/zryna/bootstrap/') or x['path'].startswith('licenses/typescript')]
 small_tar(assets/'signed-legacy-provider-6.0.2.tar.gz',entries)
 check({p.name for p in assets.iterdir()}==set(PINS),'exact closed manifest outputs')
 for name,pin in PINS.items():verify(assets/name,pin)
 save('PREPARED.json',{'status':'all-exact-asset-bytes-prepared','assets':list(PINS.values()),'compressionLibrary':zlib.ZLIB_RUNTIME_VERSION,'sourceCommit':PINNED_SOURCE,'scope':'Inert material regeneration/acquisition only; no guest execution or qualification/publication assertion.'})
 return assets

def draft_state():
 release=api('/repos/zryna/zryna/releases/'+str(PINNED_RELEASE))
 check(release['id']==PINNED_RELEASE and release['tag_name']==PINNED_TAG and release['draft'] is True and release.get('immutable') is not True,'fixed private draft identity required')
 check(release['target_commitish']==PINNED_SOURCE,'fixed draft target source required')
 assets=api('/repos/zryna/zryna/releases/'+str(PINNED_RELEASE)+'/assets?per_page=100')
 check(isinstance(assets,list) and len(assets)<=len(PINS),'bounded fixed draft assets')
 check(len({x['name'] for x in assets})==len(assets) and all(x['name'] in PINS for x in assets),'unknown/duplicate draft assets refused')
 return {x['name']:x for x in assets}

def verify_existing(asset,root):
 pin=PINS[asset['name']]
 check(asset['state']=='uploaded' and asset['size']==pin['bytes'] and asset['digest']=='sha256:'+pin['sha256'],'existing asset immutable byte identity')
 out=root/(str(asset['id'])+'-'+asset['name'])
 download('https://api.github.com/repos/zryna/zryna/releases/assets/'+str(asset['id']),out,pin,auth=True)
 return {'id':asset['id'],'name':asset['name'],'bytes':pin['bytes'],'sha256':pin['sha256'],'actualReadback':True}

def upload(assets):
 readbacks=ROOT/'asset-readbacks';readbacks.mkdir();results=[]
 existing=draft_state()
 save('INITIAL_DRAFT_ASSETS.json',[{'id':x['id'],'name':x['name'],'size':x['size'],'digest':x['digest']} for x in existing.values()])
 for name,pin in PINS.items():
  guard();current=draft_state()
  if name in current:results.append(verify_existing(current[name],readbacks));continue
  # A lost upload response remains uncertain; never retry this POST. Later operator
  # invocations can resume only after fresh listing and exact complete asset readback.
  verify(assets/name,pin)
  url='https://uploads.github.com/repos/zryna/zryna/releases/'+str(PINNED_RELEASE)+'/assets?name='+urllib.parse.quote(name,safe='')
  with (assets/name).open('rb') as body:c,r=connection(url,'POST',body=body,length=pin['bytes'],auth=True)
  try:
   check(r.status==201,'draft asset upload HTTP '+str(r.status));raw=r.read(262145);check(len(raw)<=262144,'bounded upload receipt');actual=json.loads(raw)
  finally:r.close();c.close()
  check(actual['name']==name and actual['size']==pin['bytes'] and actual['digest']=='sha256:'+pin['sha256'],'actual upload metadata digest')
  save('UPLOADED-'+name+'.json',{'id':actual['id'],'name':name,'bytes':actual['size'],'sha256':pin['sha256'],'state':actual['state']})
  latest=draft_state();check(name in latest and latest[name]['id']==actual['id'],'fresh actual uploaded asset listing')
  results.append(verify_existing(latest[name],readbacks))
 final=draft_state();check(set(final)==set(PINS),'all exact draft assets required')
 save('RESULT.json',{'status':'all-private-draft-assets-uploaded-and-independently-read-back','releaseId':PINNED_RELEASE,'tag':PINNED_TAG,'sourceCommit':PINNED_SOURCE,'assets':results,'draft':True,'published':False,'scope':'Fixed asset uploads only. No release edits/publication/tag mutation/overwrites or compiler qualification claim.'})

def main():
 global ROOT,TOKEN,LOCAL
 parser=argparse.ArgumentParser();parser.add_argument('--prepare-local');parser.add_argument('--local-attempt',type=int,default=1);args=parser.parse_args()
 if args.prepare_local:
  check(os.environ.get('GITHUB_ACTIONS')!='true','local preparation is never an Actions upload mode')
  check(1<=args.local_attempt<=9,'bounded fresh local packaging attempt')
  LOCAL=json.loads(pathlib.Path(args.prepare_local).read_bytes());ROOT=pathlib.Path('/tmp/M6_410_RAW_UPLOAD_INERT_PREPARATION_'+PINNED_SOURCE+'_RUN'+str(args.local_attempt))
 else:
  check(os.environ.get('GITHUB_ACTIONS')=='true' and os.environ.get('GITHUB_REPOSITORY')=='zryna/zryna','fixed native Actions owner required')
  check(os.environ.get('GITHUB_EVENT_NAME')=='workflow_dispatch','manual reviewed operator dispatch only')
  check(os.environ.get('GITHUB_SHA')==os.environ.get('APPROVED_OPERATOR_COMMIT'),'reviewed operator revision required')
  approved_sha=os.environ.get('APPROVED_PLAN_SHA256','')
  check(re.fullmatch('[a-f0-9]{64}',approved_sha) and sha(PLAN_RAW)==approved_sha,'complete reviewed manifest SHA required')
  check(PLAN.get('transportApproval',{}).get('approved') is True,'transport manifest is not approved for upload')
  check(PINNED_SOURCE!='5d79dba18117a7ff41ff08d328aa94838c0fce9c','known service connection-limit failure baseline is frozen; no upload dispatch')
  TOKEN=os.environ.get('GITHUB_TOKEN');check(bool(TOKEN),'native Actions token required')
  ROOT=pathlib.Path(os.environ['RUNNER_TEMP'])/'m6-fixed-draft-transport'
 ROOT.mkdir(mode=0o700)
 check(subprocess.run(['git','rev-parse','HEAD:examples/playground/restricted/daemon/preparation'],cwd=HERE.parent,capture_output=True,text=True,check=True).stdout.strip()==subprocess.run(['git','rev-parse',PINNED_SOURCE+':examples/playground/restricted/daemon/preparation'],cwd=HERE.parent,capture_output=True,text=True,check=True).stdout.strip(),'original emitter source unchanged')
 assets=prepare()
 if LOCAL is None:upload(assets)
 else:save('RESULT.json',{'status':'all-exact-bytes-validated-by-inert-local-preparation','uploads':False,'sourceCommit':PINNED_SOURCE,'assets':len(PINS)})
 print(json.dumps({'status':'complete','assets':len(PINS),'uploaded':LOCAL is None}))

if __name__=='__main__':main()
