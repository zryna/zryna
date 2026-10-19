"""One fixed sequential private runtime attempt; no default qualification verdict."""
import datetime
import ctypes
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

spec = importlib.util.spec_from_file_location('runtime_support', Path(__file__).with_name('runtime_support.py'))
s = importlib.util.module_from_spec(spec)
spec.loader.exec_module(s)


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


CALLER_UID = 1000  # ROOT-selected actual nonroot profile; preflight39180547128 retained separately.
CALLER_GID = CALLER_UID


def identity_status():
    raw = Path('/proc/self/status').read_bytes()
    fields = dict(line.split(':',1) for line in raw.decode().splitlines() if ':' in line)
    facts = {key: fields[key].strip() for key in ['Uid','Gid','Groups','CapInh','CapPrm','CapEff','CapBnd','CapAmb','NoNewPrivs']}
    s.need(facts['Uid'].split() == [str(CALLER_UID)]*4 and facts['Gid'].split() == [str(CALLER_GID)]*4 and
           facts['Groups'] == '' and facts['NoNewPrivs'] == '1' and
           all(int(facts[key],16) == 0 for key in ['CapInh','CapPrm','CapEff','CapBnd','CapAmb']), 'actual nonroot credentials, no supplementary groups, zero capabilities and NNP')
    return {'pid':os.getpid(),'parentPid':os.getppid(),'uid':os.getuid(),'euid':os.geteuid(),'gid':os.getgid(),
            'egid':os.getegid(),'groups':os.getgroups(),'status':facts,'statusSha256':hashlib.sha256(raw).hexdigest()}


def drop_caller(identity_path):
    # Trusted pre-exec privilege reduction only; never grants a capability or runs a helper.
    libc = ctypes.CDLL(None, use_errno=True)
    def prctl(option, first=0):
        s.need(libc.prctl(option, first, 0, 0, 0) == 0, 'required caller privilege reduction')
    last = int(Path('/proc/sys/kernel/cap_last_cap').read_text().strip())
    s.need(0 <= last <= 63, 'bounded actual capability set')
    for capability in range(last+1):
        value = libc.prctl(23, capability, 0, 0, 0)  # PR_CAPBSET_READ
        s.need(value in [0,1], 'actual capability bounding set')
        if value: prctl(24, capability)  # PR_CAPBSET_DROP
    s.need(libc.prctl(47, 4, 0, 0, 0) == 0, 'clear caller ambient capabilities')
    prctl(38, 1)  # PR_SET_NO_NEW_PRIVS
    os.setgroups([]); os.setresgid(CALLER_GID,CALLER_GID,CALLER_GID); os.setresuid(CALLER_UID,CALLER_UID,CALLER_UID)
    class Header(ctypes.Structure):
        _fields_ = [('version',ctypes.c_uint32),('pid',ctypes.c_int)]
    class Data(ctypes.Structure):
        _fields_ = [('effective',ctypes.c_uint32),('permitted',ctypes.c_uint32),('inheritable',ctypes.c_uint32)]
    header, data = Header(0x20080522,0), (Data*2)()
    s.need(libc.capset(ctypes.byref(header),ctypes.byref(data)) == 0, 'zero caller capability sets')
    value = identity_status()
    with Path(identity_path).open('x') as stream: json.dump(value,stream,sort_keys=True); stream.write('\n')
    Path(identity_path).chmod(0o400)


def caller_preexec(identity_path):
    try:
        drop_caller(identity_path)
    except BaseException as error:
        # Preserve this trusted fixed-helper failure, without credential/environment values.
        os.write(2,(json.dumps({'phase':'caller-credential-drop','class':type(error).__name__,'message':str(error)[:256]})+'\n').encode())
        raise


def main():
    s.need(len(sys.argv) == 2, 'fixed runner manifest argument')
    manifest = s.decode(s.ordinary(sys.argv[1]))
    root = Path(manifest['approval']['root'])
    ctx, manifest, root = s.context_inputs(root / 'REMOTE_RUNTIME_CONTEXT.json', sys.argv[1])
    operators = Path(ctx['runtime']['operatorRoot'])
    bound = s.decode(s.ordinary(operators / 'BOUND_MANIFEST.json'))
    s.need(bound['source'] == ctx['source'] and bound['phase'] == 'qualification', 'actual bound source/phase')
    for row in bound['files'] + bound['inputs']:
        s.checked(row)
    s.need(CALLER_UID in [1000,65534] and bound['caller'] == {'uid':CALLER_UID,'gid':CALLER_GID,'supplementaryGroups':[]}, 'fixed independently selected nonroot identity')
    registry = bound['jobs']
    names = ['parity', 'plain-keyboard', 'original-ui-owner', 'http-boundaries', 'session-isolation',
             'browser-faults', 'compiler-owner-faults', 'compiler-resources', 'pipe-boundaries',
             'storage-boundaries', 'material-denials', 'role-kernel-policy', 'host-death-and-owned-recovery']
    s.need([row['name'] for row in registry] == names and len(registry) == 13, 'fixed mature thirteen selections')
    output = Path(ctx['runtime']['outputRoot'])
    output.mkdir(mode=0o755)
    caller_output, trusted_output = output/'caller', output/'trusted'
    caller_output.mkdir(mode=0o700); os.chown(caller_output,CALLER_UID,CALLER_GID)
    trusted_output.mkdir(mode=0o700)
    identities = caller_output/'.identities'; identities.mkdir(mode=0o700); os.chown(identities,CALLER_UID,CALLER_GID)
    journal = output / 'execution'
    journal.mkdir(mode=0o700)
    s.need(all(not Path(row['result']).parent.exists() for row in registry), 'all original cohort roots unrun')
    rows, started = [], now()
    s.save(journal / 'INTENT.json', {'source': ctx['source'], 'manifest': s.pin(sys.argv[1]),
        'binding': s.pin(operators / 'BOUND_MANIFEST.json'), 'startedAt': started, 'noRetries': True,
        'fullQualification': False, 'diagnosticOnly': True, 'qualificationAcceptance': False, 'selectionOrdinal':3, 'scope': 'One original UI owner diagnostic only. No full13,68 or final7 metadata phase; first owning failure stops with original cleanup and no retry.'})
    deadline = time.monotonic() + 2400
    failure_phase = 'owning-cohorts'
    try:
        for index, job in [(3, registry[2])]:
            s.need(job['name']=='original-ui-owner' and job['role']=='nonroot-caller','single fixed ordinal3 diagnostic selection')
            role = 'trusted-probe' if job['name']=='role-kernel-policy' else 'nonroot-caller'
            s.need(job['role']==role and Path(job['result']).is_relative_to(trusted_output if role=='trusted-probe' else caller_output), 'fixed owning role and output namespace')
            s.git_source(ctx['source'])
            s.need(time.monotonic() < deadline, 'original bounded whole operator window')
            for row in bound['approvals']:
                s.checked(row)
            prefix = journal / (f'{index:02}-' + job['name'])
            row = {'name': job['name'], 'argv': job['argv'], 'startedAt': now(),
                   'trustedOuterSeconds': job['seconds'], 'nativeCleanupEstablishedByOuterOwner': False}
            s.save(str(prefix) + '.START.json', row)
            print(json.dumps({'event': 'actual-cohort-start', 'name': job['name']}), flush=True)
            child = None
            environment = os.environ.copy(); environment.pop('GITHUB_TOKEN',None)
            identity_path = identities/(f'{index:02}-'+job['name']+'.node.json')
            preexec_path = identities/(f'{index:02}-'+job['name']+'.preexec.json')
            if role=='nonroot-caller':
                environment.update(GIT_CONFIG_COUNT='1',GIT_CONFIG_KEY_0='safe.directory',GIT_CONFIG_VALUE_0='/workspace/zryna',GIT_CONFIG_NOSYSTEM='1',GIT_CONFIG_GLOBAL='/dev/null',GIT_NO_REPLACE_OBJECTS='1')
                environment['M6_410_CALLER_IDENTITY_PATH'] = str(identity_path)
            else:
                environment.pop('M6_410_CALLER_IDENTITY_PATH',None)
            with Path(str(prefix) + '.stdout').open('xb') as out, Path(str(prefix) + '.stderr').open('xb') as err:
                try:
                    # Inherit the runtime phase's owned group; the harness can terminate the whole group.
                    child = subprocess.Popen(job['argv'], cwd='/workspace/zryna', env=environment,
                                             stdin=subprocess.DEVNULL, stdout=out, stderr=err,
                                             preexec_fn=(lambda: caller_preexec(preexec_path)) if role=='nonroot-caller' else None)
                    row['pid'] = child.pid
                    row['epoch'] = Path(f'/proc/{child.pid}/stat').read_text().rsplit(')', 1)[1].split()[19]
                    try:
                        row['exitCode'] = child.wait(timeout=min(job['seconds'], max(0.01, deadline-time.monotonic())))
                    except subprocess.TimeoutExpired:
                        row['timedOut'] = True
                        child.kill()
                        try:
                            row['exitCode'] = child.wait(timeout=10)
                        except subprocess.TimeoutExpired:
                            row['directChildReapUnknown'] = True
                finally:
                    if child is not None and child.poll() is None:
                        child.kill()
                        try:
                            child.wait(timeout=10)
                        except subprocess.TimeoutExpired:
                            row['directChildReapUnknown'] = True
            row['role'] = role
            if role=='nonroot-caller':
                row['preexecIdentity'] = s.result_pin(preexec_path) if preexec_path.exists() else None
                row['actualNodeIdentity'] = s.result_pin(identity_path) if identity_path.exists() else None
                if identity_path.exists():
                    observed = s.decode(s.result_read(identity_path))
                    row['actualCallerIdentityVerified'] = observed.get('pid')==row.get('pid') and observed.get('uid')==CALLER_UID and observed.get('gid')==CALLER_GID and observed.get('euid')==CALLER_UID and observed.get('egid')==CALLER_GID and observed.get('groups')==[] and observed.get('status',{}).get('Uid','').split()==[str(CALLER_UID)]*4 and observed.get('status',{}).get('Gid','').split()==[str(CALLER_GID)]*4 and observed.get('status',{}).get('Groups')=='' and observed.get('status',{}).get('NoNewPrivs')=='1' and all(int(observed.get('status',{}).get(key,'1'),16)==0 for key in ['CapInh','CapPrm','CapEff','CapBnd','CapAmb'])
                else: row['actualCallerIdentityVerified'] = False
            row.update(finishedAt=now(), stdout=s.pin(str(prefix)+'.stdout'), stderr=s.pin(str(prefix)+'.stderr'))
            result = Path(job['result'])
            if result.exists():
                row['result'] = s.result_pin(result)
                try:
                    row['observedStatus'] = s.decode(s.result_read(result)).get('status')
                except BaseException as error:
                    row['resultParseFailure'] = str(error)
            row['owningCommandPassed'] = row.get('exitCode') == 0 and not row.get('timedOut') and row.get('observedStatus') == job['status'] and (role=='trusted-probe' or row.get('actualCallerIdentityVerified') is True)
            rows.append(row)
            s.save(str(prefix)+'.COMPLETE.json', row)
            s.need(row['owningCommandPassed'], 'first owning command failure; retained raw and no retry/default qualification')
        s.save(journal / 'RESULT.json', {'status': 'single-ui-owner-diagnostic-command-completed', 'source': ctx['source'],
            'rows': rows, 'startedAt': started, 'finishedAt': now(), 'fullQualification': False,
            'diagnosticOnly': True, 'qualificationAcceptance': False, 'completed':1, 'selectionOrdinal':3,
            'scope': 'Only original UI owning command ordinal3 executed. No full13 or68 qualification; no final7 GET confirmation. Original controls/identity/cleanup unchanged.'})
    except BaseException as error:
        s.save(journal / 'FAILURE.json', {'status': ('stopped-at-first-owning-failure-no-retry' if failure_phase == 'owning-cohorts' else 'final-readonly-confirmation-failed-after-thirteen-commands'), 'source': ctx['source'], 'failurePhase': failure_phase,
            'failure': {'type': type(error).__name__, 'message': str(error)}, 'rows': rows,
            'startedAt': started, 'finishedAt': now(), 'fullQualification': False})
        raise


if __name__ == '__main__':
    main()
