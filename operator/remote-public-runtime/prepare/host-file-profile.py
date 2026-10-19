"""Prepare one exact root-reviewed host-file observer provider; no Engine/guest calls."""
import ast,hashlib,json,os,re,stat,time
from pathlib import Path
C='71fd69d99a39c22bdf262d1655a032aa76221f3e'
T='351da60bd4037e0f96c286045c8943f41fa99b03'
ORIGINAL_SHA='50f2503d9877bcabcf6cd388aa8cf7c8782e0ffafcf790c54c29d71b07780aa8'
FIELDS=('st_dev','st_ino','st_mode','st_nlink','st_uid','st_gid','st_size','st_mtime_ns','st_ctime_ns')
def need(value,reason):
    if not value:raise ValueError(reason)
def state(s):return {k:getattr(s,k)for k in FIELDS}
def canonical(value):
    return type(value)is str and (value=='/'or bool(re.fullmatch(r'/(?:[A-Za-z0-9_.@+-]+/)*[A-Za-z0-9_.@+-]+',value)))and all(x not in['.','..']for x in value.split('/'))
def valid_source(filesystem,source):
    return (filesystem=='overlay'and source=='overlay')or(filesystem in['ext4','xfs','btrfs']and type(source)is str and source.startswith('/dev/')and canonical(source)and bool(re.fullmatch(r'/dev/(?:[A-Za-z0-9_.+-]+/)*[A-Za-z0-9_.+-]+',source)))
def beneath(path,parent):return path==parent or path.startswith(parent.rstrip('/')+'/')
def ordinary(path,directory=False):
    p=Path(path);need(p.is_absolute()and canonical(str(p)),'canonical absolute trusted path')
    for ancestor in[p,*p.parents]:
        s=ancestor.lstat();need(not stat.S_ISLNK(s.st_mode)and s.st_uid==0 and not s.st_mode&0o022,'root-owned no-link trusted ancestry')
    s=p.lstat();need(stat.S_ISDIR(s.st_mode)if directory else stat.S_ISREG(s.st_mode)and s.st_nlink==1,'ordinary trusted source')
    return p
def pin(path):
    p=ordinary(path);before=state(p.lstat());h=hashlib.sha256();count=0
    with p.open('rb')as stream:
        while block:=stream.read(65536):count+=len(block);need(count<=128*1024**2,'bounded whole provider input');h.update(block)
    need(state(p.lstat())==before and count==before['st_size']and count>0,'stable whole provider input')
    return {'path':str(p),'bytes':count,'sha256':h.hexdigest()}
def kernel(path,maximum):
    fd=os.open(path,os.O_RDONLY|os.O_NOFOLLOW|os.O_CLOEXEC)
    try:
        before=state(os.fstat(fd));need(stat.S_ISREG(before['st_mode'])and before['st_uid']==0,'actual root process metadata')
        out=bytearray()
        while True:
            block=os.read(fd,min(16384,maximum+1-len(out)))
            if not block:break
            out.extend(block);need(len(out)<=maximum,'kernel metadata first extra byte')
        need(state(os.fstat(fd))==before,'stable kernel descriptor');return bytes(out)
    finally:os.close(fd)
def process(pid,argv,binary):
    raw=kernel('/proc/'+str(pid)+'/stat',16384);fields=raw.decode('ascii').rsplit(')',1)[1].split();need(len(fields)>=20 and fields[0]not in['Z','X','x']and fields[19].isdigit(),'live exact task-owned daemon epoch')
    actual=kernel('/proc/'+str(pid)+'/cmdline',16384);need(actual==b'\0'.join(x.encode('ascii')for x in argv)+b'\0','exact private daemon command')
    status=kernel('/proc/'+str(pid)+'/status',16384).decode('ascii');rows=dict(x.split(':',1)for x in status.splitlines()if ':'in x)
    need(rows['Uid'].split()==rows['Gid'].split()==['0']*4 and os.readlink('/proc/'+str(pid)+'/exe')==binary,'root exact selected daemon binary')
    return {'identity':{'pid':pid,'epoch':fields[19],'argv':argv,'uid':rows['Uid'].split(),'gid':rows['Gid'].split(),'exe':binary,'commandSha256':hashlib.sha256(actual).hexdigest()},'statusSha256':hashlib.sha256(status.encode()).hexdigest()}
def covering(raw,container_directory):
    need(raw.endswith(b'\n')and len(raw)<=262144,'whole bounded mount table');rows=[];ids=set()
    for line in raw.decode('ascii').splitlines():
        left,separator,right=line.partition(' - ');a,b=left.split(),right.split()
        need(separator and len(a)>=6 and len(b)==3 and len(rows)<512 and re.fullmatch('[1-9][0-9]{0,9}',a[0])and re.fullmatch('[0-9]{1,10}',a[1]),'complete mount identity row')
        ident,parent=int(a[0]),int(a[1]);need(ident not in ids and ident!=parent and canonical(a[4])and re.fullmatch('[0-9]{1,10}:[0-9]{1,10}',a[2]),'unique canonical mount identity')
        ids.add(ident);rows.append({'id':ident,'parentId':parent,'target':a[4],'root':a[3],'device':a[2],'filesystem':b[0],'source':b[1],'options':a[5].split(','),'superOptions':b[2].split(',')})
    need(not any(row['target'].startswith(container_directory+'/')for row in rows),'no deeper Docker container-file submounts')
    candidates=[row for row in rows if beneath(container_directory,row['target'])];need(candidates,'actual covering Docker storage mount')
    length=max(len(x['target'])for x in candidates);group=[x for x in candidates if len(x['target'])==length];need(len({x['target']for x in group})==1,'unique longest mount coordinate')
    tops=[x for x in group if not any(y['parentId']==x['id']for y in group)];need(len(tops)==1,'unique effective same-target stack')
    chosen=tops[0];seen=set();cursor=chosen
    while True:
        need(cursor['id']not in seen,'acyclic mount stack');seen.add(cursor['id']);parents=[x for x in group if x['id']==cursor['parentId']]
        if not parents:break
        need(len(parents)==1,'unique stack ancestry');cursor=parents[0]
    need(len(seen)==len(group)and canonical(chosen['root'])and chosen['filesystem']in['overlay','ext4','xfs','btrfs'],'closed supported backing mount profile')
    need(valid_source(chosen['filesystem'],chosen['source']),'closed canonical backing filesystem source')
    relative=container_directory[len(chosen['target'].rstrip('/')):].lstrip('/')
    coordinate=chosen['root'].rstrip('/')+('/'+relative if relative else '')or'/'
    need(canonical(coordinate),'exact filesystem-root coordinate without normalization')
    return chosen,coordinate

def project(raw,profile):
    need(set(profile)=={'containerRoot','device','filesystem','source'}and canonical(profile['containerRoot'])and re.fullmatch('[0-9]{1,10}:[0-9]{1,10}',profile['device'])and profile['filesystem']in['overlay','ext4','xfs','btrfs'],'closed exact provider literal')
    need(valid_source(profile['filesystem'],profile['source']),'closed canonical projected backing filesystem source')
    need(hashlib.sha256(raw.encode()).hexdigest()==ORIGINAL_SHA,'exact original source bytes for projection')
    oldroot="base['root'] == '/var/lib/docker/containers/' + cid + '/' + target.rsplit('/', 1)[1]";newroot="base['root'] == HOST_FILE_PROFILE['containerRoot'] + '/' + cid + '/' + target.rsplit('/', 1)[1]"
    oldfs="base['filesystem'] == 'overlay' and base['source'] == 'overlay' and";newfs="base['filesystem'] == HOST_FILE_PROFILE['filesystem'] and base['source'] == HOST_FILE_PROFILE['source'] and base['device'] == HOST_FILE_PROFILE['device'] and"
    need(raw.count(oldroot)==raw.count(oldfs)==1,'exact narrow original predicate shape');generated=raw.replace(oldroot,newroot).replace(oldfs,newfs)
    constant='HOST_FILE_PROFILE = '+repr(profile)+'\n';generated=generated.replace('import re\n','import re\n\n'+constant,1)
    restored=generated.replace(constant,'').replace(newroot,oldroot).replace(newfs,oldfs)
    need(ast.dump(ast.parse(restored),include_attributes=False)==ast.dump(ast.parse(raw),include_attributes=False),'all original condition/control AST except selected profile and added device retained')
    return generated

def generate(manifest,candidate_root):
    """Called only by authenticated root prepare before caller view/PREPARATION/tag."""
    started=time.monotonic();need(os.geteuid()==0,'root-only observer provider generation')
    m=manifest;host=ordinary(m['hostRoot'],True);root=ordinary(candidate_root,True)
    need(m['source']=={'path':'/workspace/zryna','commit':C,'tree':T}and root==host/'public-composition/baseline'and m['approval']['phase']=='public'and m['approval']['root']==str(host/'public-composition')and str(host)=='/workspace/scratch/M6_410_EPHEMERAL_HOST_'+m['run']['id']+'_'+m['run']['attempt'],'exact selected source/public baseline host')
    info_path=host/'engine-info.stdout';info_pin=pin(info_path);info=json.loads(info_path.read_bytes());data=str(host/'data');containers=data+'/containers';directory=ordinary(containers,True)
    need(info['DockerRootDir']==data and info['Driver']=='overlay2'and info['ServerVersion']=='28.4.0','actual selected original Engine storage identity')
    directory_before=state(directory.lstat());binary=str(host/'engine-bin/dockerd');binary_path=ordinary(binary);binary_stat=state(binary_path.lstat());binary_pin=pin(binary_path)
    pid_path=host/'dockerd.pid';pid_pin=pin(pid_path);pid_raw=pid_path.read_bytes();need(re.fullmatch(rb'[1-9][0-9]{0,9}\n?',pid_raw),'exact owned daemon pidfile');pid=int(pid_raw)
    nonce=m['run']['id']+'-'+m['run']['attempt'];argv=[binary,'--host=unix://'+str(host/'engine.sock'),'--group=nogroup','--config-file='+str(host/'daemon.json'),'--data-root='+data,'--exec-root='+str(host/'exec'),'--pidfile='+str(pid_path),'--bridge=none','--iptables=false','--ip6tables=false','--ip-forward=false','--ip-masq=false','--containerd='+str(host/'containerd.sock'),'--containerd-namespace=m6-runtime-'+nonce,'--containerd-plugins-namespace=m6-runtime-plugins-'+nonce,'--exec-opt=native.cgroupdriver=cgroupfs']
    before=process(pid,argv,binary);daemon_raw=kernel('/proc/'+str(pid)+'/mountinfo',262144);observer_raw=kernel('/proc/self/mountinfo',262144)
    daemon,coordinate=covering(daemon_raw,containers);observer,other_coordinate=covering(observer_raw,containers)
    keys=['root','target','device','filesystem','source','superOptions']
    need(all(daemon[k]==observer[k]for k in keys)and coordinate==other_coordinate,'same exact daemon/observer storage coordinates')
    need(daemon['device']==str(os.major(directory_before['st_dev']))+':'+str(os.minor(directory_before['st_dev'])),'actual directory backing device')
    after=process(pid,argv,binary)
    need(kernel('/proc/'+str(pid)+'/mountinfo',262144)==daemon_raw and kernel('/proc/self/mountinfo',262144)==observer_raw and after['identity']==before['identity'],'same source mount tables/stable daemon identity; live status hashes are observations only')
    need(state(directory.lstat())==directory_before and state(binary_path.lstat())==binary_stat and pin(binary_path)==binary_pin and pin(info_path)==info_pin and pin(pid_path)==pid_pin and time.monotonic()-started<10,'stable actual provider acquisition before PREP')
    out=root/'host-file-provider';out.mkdir(mode=0o755)
    def put(name,raw,mode=0o400):
        path=out/name
        with path.open('xb')as f:f.write(raw)
        path.chmod(mode);return pin(path)
    def save(name,value):return put(name,(json.dumps(value,sort_keys=True,indent=2)+'\n').encode())
    proof=[put('daemon-mountinfo.raw',daemon_raw),put('observer-mountinfo.raw',observer_raw)]
    proof.append(save('HOST_METADATA.json',{'source':m['source'],'daemonBefore':before,'daemonAfter':after,'statusHashMeaning':'Separate whole live status snapshot hashes; changing RSS/context-switch counters are not identity disagreement. PID/epoch/UID/GID/exe/exactargv/cmdline wholehash must remain equal.','daemonBinary':binary_pin,'pidfile':pid_pin,'engineInfo':info_pin,'directory':{'path':containers,'before':directory_before,'after':state(directory.lstat())},'daemonCoveringMount':daemon,'observerCoveringMount':observer,'observerPid':os.getpid(),'scope':'Actual same backing coordinates; different observer /run/cgroup mounts are not equated.'}))
    profile={'containerRoot':coordinate,'device':daemon['device'],'filesystem':daemon['filesystem'],'source':daemon['source']}
    proof.append(save('HOST_FILE_PROFILE.json',{'format':'zryna.m6-reviewed-host-file-profile.v1','source':m['source'],'profile':profile,'approval':False,'acceptance':False,'coordinateEquation':'mountRoot + relative(DockerRootDir/containers,mountTarget); append original64hex CID and one fixed hostname/hosts/resolv.conf basename'}))
    original=Path(m['source']['path'])/'examples/playground/restricted/daemon/ui-qualification/owner_mounts.py';original_pin=pin(original);need(original_pin['sha256']==ORIGINAL_SHA,'exact original E owner mount observer')
    raw=original.read_text();generated=project(raw,profile)
    proof.append(put('original-owner_mounts.py',raw.encode()))
    selected=put('owner_mounts.py',generated.encode(),0o444);proof.append(selected)
    provenance=save('PROVENANCE.json',{'format':'zryna.m6-host-file-observer-provider.v1','source':m['source'],'original':original_pin,'generated':selected,'profile':profile,'proofs':proof,'originalOtherElevenHelpersUnchanged':True,'separatelySelectedLogger':{'helper':pin(Path(__file__).parent/'safe-logging-owner_main.py'),'enumMap':pin(Path(__file__).parent/'ui-owner-failure-enums.json'),'meaning':'Previously reviewed118 fixed-enum failure logging only; original condition/control/cleanup unchanged, original five public helpers remain unchanged. Map historical diagnostic labels do not label this run.'},'acceptance':False,'scope':'Separately selected operator helper; only exact host-file root/fs/source projection plus device equality. Original null tmpfs/same-device/flags/ancestry/protected mounts/UID/proc/budgets unchanged. No source product edit, ambient profile read, guest, Engine call or approval.'})
    return {'helper':selected,'provenance':provenance,'profile':profile,'inputs':proof+[provenance]}
