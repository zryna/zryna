"""Closed, pinned operator inputs and bounded ownership; no publication authority."""
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import stat
import subprocess
import time


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def require(value, reason):
    if not value:
        raise ValueError(reason)


def exact(value, keys):
    require(type(value) is dict and set(value) == set(keys), 'closed operator manifest fields')


def write_new(path, value):
    with Path(path).open('x') as stream:
        json.dump(value, stream, sort_keys=True, indent=2)
        stream.write('\n')


def trusted(path, directory=False):
    path = Path(path)
    require(path.is_absolute() and str(path) == os.path.normpath(str(path)), 'absolute canonical path')
    for entry in [*reversed(path.parents), path]:
        item = entry.lstat()
        require(item.st_uid == 0 and not item.st_mode & 0o022 and not stat.S_ISLNK(item.st_mode),
                'root-owned ordinary path without writable ancestors or symlinks')
        if entry != path or directory:
            require(stat.S_ISDIR(item.st_mode), 'ordinary directory')
    return path


def file_pin(path):
    path = trusted(path)
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    try:
        before = os.fstat(descriptor)
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and 0 < before.st_size <= 1024**3,
                'bounded ordinary single-link file')
        digest = hashlib.sha256()
        while True:
            block = os.read(descriptor, 65536)
            if not block:
                break
            digest.update(block)
        after = os.fstat(descriptor)
        require((before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns) ==
                (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns), 'stable file')
        return {'path': str(path), 'bytes': before.st_size, 'sha256': digest.hexdigest()}
    finally:
        os.close(descriptor)


def check_pin(value):
    exact(value, ['path', 'bytes', 'sha256'])
    require(type(value['bytes']) is int and re.fullmatch('[a-f0-9]{64}', value['sha256']), 'whole-file pin')
    require(file_pin(value['path']) == value, 'actual operator file differs from selected pin')
    return value['path']


def read_manifest(path, expected_sha256, harness_root):
    require(re.fullmatch('[a-f0-9]{64}', expected_sha256), 'explicit independently selected manifest SHA256')
    descriptor = file_pin(path)
    require(descriptor['bytes'] <= 65536 and descriptor['sha256'] == expected_sha256, 'manifest identity')
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    value = json.loads(Path(path).read_text(), object_pairs_hook=pairs,
                       parse_constant=lambda _: require(False, 'nonfinite JSON'))
    exact(value, ['format', 'run', 'source', 'hostRoot', 'files', 'limits', 'approval'])
    require(value['format'] == 'zryna.m6-remote-host-harness.v1', 'operator manifest version')
    exact(value['run'], ['id', 'attempt'])
    require(all(type(v) is str and re.fullmatch('[1-9][0-9]{0,19}', v) for v in value['run'].values()), 'actual run selectors')
    exact(value['source'], ['path', 'commit', 'tree'])
    require(value['source']['path'] == '/workspace/zryna' and
            all(re.fullmatch('[a-f0-9]{40}', value['source'][k]) for k in ['commit', 'tree']), 'selected source identity')
    trusted('/workspace/zryna', True)
    trusted('/workspace/scratch', True)
    require(value['hostRoot'] == '/workspace/scratch/M6_410_EPHEMERAL_HOST_' +
            value['run']['id'] + '_' + value['run']['attempt'], 'fresh task-owned namespace')
    exact(value['files'], ['host', 'observer', 'common', 'engineAcquire', 'broker',
                           'python', 'node', 'context', 'prepare', 'waitApproval', 'runtime', 'materialCapture'])
    for item in value['files'].values():
        check_pin(item)
    for key, name in [('host', 'host.py'), ('observer', 'observer.py'), ('common', 'common.py'),
                      ('engineAcquire', 'engine_acquire.py'), ('broker', 'broker.py')]:
        require(value['files'][key]['path'] == str(harness_root / name), 'fixed reviewed harness file')
    for key, name in [('context', 'context.json'), ('prepare', 'prepare.py'),
                      ('waitApproval', 'wait-approval.py'), ('runtime', 'runtime.mjs'), ('materialCapture', 'material-capture.py')]:
        path_value = Path(value['files'][key]['path'])
        require(path_value.name == name and path_value.is_relative_to('/workspace/scratch'), 'fixed pinned phase wrapper')
    exact(value['limits'], ['prepareSeconds', 'approvalSeconds', 'runtimeSeconds', 'materialCaptureSeconds'])
    require(value['limits']['materialCaptureSeconds'] == 480, 'separately reviewed inert post-runtime retention budget')
    require(all(type(v) is int and 1 <= v <= 18000 for v in value['limits'].values()) and
            value['limits']['approvalSeconds'] <= 600 and sum(value['limits'].values()) <= 21000,
            'finite trusted host phases, approval at most600seconds')
    require(type(value['approval']) is dict, 'approval JSON delegated only to pinned wait helper')
    return value, descriptor


def epoch(pid):
    return Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()[19]


def group_signal(process, selected_epoch, sig):
    require(epoch(process.pid) == selected_epoch, 'same task-owned process epoch')
    os.killpg(process.pid, sig)


def kill_owned_group(pid, selected_epoch):
    require(epoch(pid) == selected_epoch, 'owned group leader PID was reused')
    try:
        os.killpg(pid, signal.SIGKILL)
    except ProcessLookupError:
        pass


def wait_unreaped(process, seconds):
    until = time.monotonic() + seconds
    while True:
        # Retain the leader as a zombie until group shutdown; PID cannot be reused.
        if os.waitid(os.P_PID, process.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT) is not None:
            return True
        if time.monotonic() >= until:
            return False
        time.sleep(max(0, min(0.05, until - time.monotonic())))


def stop_owned(process, selected_epoch, seconds):
    result = {'pid': process.pid, 'epoch': selected_epoch, 'finishedAt': None}
    try:
        if process.returncode is None:
            group_signal(process, selected_epoch, signal.SIGTERM)
            if not wait_unreaped(process, seconds):
                group_signal(process, selected_epoch, signal.SIGKILL)
            kill_owned_group(process.pid, selected_epoch)
            process.wait(timeout=5)
        result.update(reaped=True, returncode=process.returncode)
    except BaseException as error:
        result.update(reaped=False, error={'type': type(error).__name__, 'message': str(error)})
    result['finishedAt'] = now()
    return result


def output_pin(path):
    data = Path(path).read_bytes()
    return {'path': str(path), 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}


def run_owned(argv, cwd, seconds, prefix, environment, preexec_fn=None):
    prefix = Path(prefix)
    record = {'argv': argv, 'cwd': str(cwd), 'seconds': seconds, 'startedAt': now(), 'timedOut': False}
    write_new(str(prefix) + '.intent.json', record)
    process = None
    with Path(str(prefix) + '.stdout').open('xb') as out, Path(str(prefix) + '.stderr').open('xb') as err:
        try:
            process = subprocess.Popen(argv, cwd=cwd, env=environment, stdin=subprocess.DEVNULL,
                                       stdout=out, stderr=err, start_new_session=True, preexec_fn=preexec_fn)
            selected_epoch = epoch(process.pid)
            record.update(pid=process.pid, epoch=selected_epoch)
            write_new(str(prefix) + '.started.json', record)
            record['timedOut'] = not wait_unreaped(process, seconds)
        except BaseException as error:
            record['error'] = {'type': type(error).__name__, 'message': str(error)}
        finally:
            if process is not None:
                try:
                    # The unreaped owned leader anchors this exact group through shutdown.
                    if 'epoch' in record:
                        kill_owned_group(process.pid, record['epoch'])
                    process.wait(timeout=5)
                    record.update(returncode=process.returncode, reaped=True)
                except BaseException as error:
                    record['reapError'] = {'type': type(error).__name__, 'message': str(error)}
    record.update(finishedAt=now(), stdout=output_pin(str(prefix) + '.stdout'), stderr=output_pin(str(prefix) + '.stderr'))
    write_new(str(prefix) + '.json', record)
    require(record.get('returncode') == 0 and not record['timedOut'] and
            not any(k in record for k in ['error', 'reapError']), 'owned command failed; retained original streams/journal')
    return record


def cgroup_mounts():
    return [line for line in Path('/proc/self/mountinfo').read_text().splitlines()
            if line.split(' - ')[0].split()[4] == '/sys/fs/cgroup']


def caller_file_pin(path, caller_root, empty=False):
    path=Path(path);base=Path(caller_root)
    require(base.is_absolute() and base.name=='public-observations' and base.parent.name.startswith('M6_410_EPHEMERAL_HOST_') and base.parent.parent==Path('/workspace/scratch'),'fixed caller output namespace')
    require(path.is_relative_to(base) and path!=base and path.resolve()==path,'exact ordinary caller output path')
    trusted(base.parent,True)
    for parent in [base,*reversed(path.relative_to(base).parents)]:
        p=parent if parent==base else base/parent
        if p==base:continue
        s=p.lstat();require(stat.S_ISDIR(s.st_mode) and not stat.S_ISLNK(s.st_mode) and s.st_uid==s.st_gid==1000 and not s.st_mode&0o022,'owned caller ancestry')
    s=base.lstat();require(stat.S_ISDIR(s.st_mode) and not stat.S_ISLNK(s.st_mode) and s.st_uid==s.st_gid==1000 and not s.st_mode&0o022,'owned caller root')
    fd=os.open(path,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK|os.O_CLOEXEC)
    try:
        before=os.fstat(fd);require(stat.S_ISREG(before.st_mode) and before.st_nlink==1 and before.st_uid==before.st_gid==1000 and not before.st_mode&0o7022 and (empty or before.st_size>0) and before.st_size<=1024**3,'bounded single-link caller leaf')
        h=hashlib.sha256();size=0
        while block:=os.read(fd,65536):h.update(block);size+=len(block)
        fields=['st_dev','st_ino','st_size','st_mode','st_uid','st_gid','st_nlink','st_mtime_ns','st_ctime_ns'];after=os.fstat(fd);named=path.lstat()
        require(size==before.st_size and all(getattr(before,k)==getattr(after,k)==getattr(named,k) for k in fields),'stable whole caller output')
        return {'path':str(path),'bytes':size,'sha256':h.hexdigest()}
    finally:os.close(fd)
