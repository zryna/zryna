"""Pure translation of selected Rust config/policy inputs; not an approval API."""
import hashlib, json

ENV = ['PATH=/materials/runtime/node/bin','LC_ALL=C.UTF-8','LANG=C.UTF-8',
       'TMPDIR=/tmp','HOME=/nonexistent','TZ=UTC','SOURCE_DATE_EPOCH=0']

def container(record, compile, holder):
    memory = 16777216 if holder else 536870912 if compile else 1073741824
    tasks = 2 if holder else 32 if compile else 256
    seccomp = json.loads(record['compiler_seccomp_json'] if holder or compile else record['evaluator_seccomp_json'])
    # serde_json Value display has sorted object keys; the source files are canonical.
    seccomp_text = json.dumps(seccomp, separators=(',', ':'), sort_keys=True)
    mounts = [{'Type':'volume','Source':'input','Target':'/input','ReadOnly':not holder,'VolumeOptions':{'NoCopy':True}},
              {'Type':'volume','Source':'output','Target':'/output','ReadOnly':holder,'VolumeOptions':{'NoCopy':True}}]
    if not holder:
        material = record['compiler_materials'] if compile else record['evaluator_materials']
        mounts.append({'Type':'volume','Source':material['name'],'Target':'/materials','ReadOnly':True,'VolumeOptions':{'NoCopy':True}})
    host = {'ReadonlyRootfs':True,'NetworkMode':'none','CapDrop':['ALL'],
        'Privileged':False,'Init':False,'IpcMode':'none','CgroupnsMode':'private',
        'Runtime':'runc','PidMode':'','UTSMode':'','UsernsMode':'',
        'OomKillDisable':False,'ShmSize':67108864,'PidsLimit':tasks,
        'Memory':memory,'MemorySwap':memory,'CpuPeriod':100000,
        'CpuQuota':10000 if holder else 200000,'CpusetCpus':','.join(map(str,record['cpus'])),
        'Tmpfs':{'/tmp':'rw,nosuid,nodev,noexec,size='+str(1048576 if holder else 67108864 if compile else 134217728)+',mode=1777',
            '/dev':'rw,nosuid,noexec,dev,size=1048576,mode=0755',
            '/dev/shm':'rw,nosuid,nodev,noexec,size='+str(1048576 if compile or holder else 67108864)+',mode=1777'},
        'Ulimits':[{'Name':'core','Soft':0,'Hard':0},{'Name':'fsize','Soft':268435456,'Hard':268435456},{'Name':'nofile','Soft':512,'Hard':512}],
        'LogConfig':{'Type':'none','Config':{}},'AutoRemove':False,
        'RestartPolicy':{'Name':'no','MaximumRetryCount':0},
        'SecurityOpt':['no-new-privileges:true','seccomp='+seccomp_text],
        'MaskedPaths':['/proc/acpi','/proc/asound','/proc/interrupts','/proc/kcore',
            '/proc/keys','/proc/latency_stats','/proc/timer_list','/proc/timer_stats',
            '/proc/sched_debug','/proc/scsi','/sys/firmware','/sys/devices/virtual/powercap',
            '/etc/hostname','/etc/hosts','/etc/resolv.conf'],
        'ReadonlyPaths':['/proc'] if holder or compile else ['/proc/bus','/proc/fs','/proc/irq','/proc/sys','/proc/sysrq-trigger'],
        'Mounts':mounts}
    return {'Image':'','User':'65534:65534','Entrypoint':['/supervisor'],'Cmd':['idle'],
        'WorkingDir':'/tmp','Env':ENV,'Tty':False,'OpenStdin':False,
        'AttachStdin':False,'AttachStdout':False,'AttachStderr':False,'StdinOnce':False,
        'Labels':{'zryna410.owner':''},'HostConfig':host}

def policy(record):
    volumes = [{'type':'tmpfs','device':'tmpfs','o':o} for o in [
        'rw,nosuid,nodev,noexec,size=2097152,mode=0555,uid=0,gid=0',
        'rw,nosuid,nodev,noexec,size=2097152,mode=0700,uid=65534,gid=65534']]
    return {'id':'zryna.playground-local-daemon','version':2,
        'compositionKind':record['composition_kind'],
        'compiler':container(record,True,False),'evaluator':container(record,False,False),
        'holder':container(record,True,True),'volumes':volumes,
        'compilerSeconds':10,'evaluationSeconds':5,'cleanupSeconds':2,
        'stagingOverhead':{'memoryBytes':16777216,'pids':2,'cpuQuota':10000},
        'sourceBytes':4096,'requestBytes':32768,'frameBytes':1277956,'stderrBytes':4096,
        'inputArchiveBytes':2162688,'outputArchiveBytes':2162688,
        'compilerRootSha256':record['compiler_root_sha256'],'evaluatorRootSha256':record['evaluator_root_sha256'],
        'compilerExecutableSha256':record['compiler_executable_sha256'],
        'bindingTemplateSha256':record['binding_template_sha256'],'witClosureSha256':record['wit_closure_sha256'],
        'compositionSha256':record['composition_sha256'],
        'compilerMaterials':record['compiler_materials'],'evaluatorMaterials':record['evaluator_materials']}

def encode(record):
    return (json.dumps(policy(record),separators=(',', ':'),sort_keys=True)+'\n').encode()

def digest(record):
    return hashlib.sha256(encode(record)).hexdigest()

if __name__ == '__main__':
    import argparse, pathlib, sys
    parser=argparse.ArgumentParser();parser.add_argument('--record',required=True)
    args=parser.parse_args()
    sys.stdout.buffer.write(encode(json.loads(pathlib.Path(args.record).read_bytes())))
