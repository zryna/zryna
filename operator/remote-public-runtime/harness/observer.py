"""One private observer namespace: pinned public baseline, then same-process public authentication/gate/runtime."""
import importlib.util
import ctypes
import hashlib
import stat
import json
import os
from pathlib import Path
import socket
import struct
import subprocess
import sys
import time

HERE = Path(__file__).absolute().parent
spec = importlib.util.spec_from_file_location('remote_host_common', HERE / 'common.py')
c = importlib.util.module_from_spec(spec)
spec.loader.exec_module(c)



def resolver_snapshot(root):
    configured = Path('/etc/resolv.conf')
    fields = ['st_dev', 'st_ino', 'st_mode', 'st_nlink', 'st_size', 'st_uid', 'st_gid', 'st_mtime_ns', 'st_ctime_ns']
    identity = lambda value: {key: getattr(value, key) for key in fields}
    link = configured.lstat()
    c.require(link.st_uid == 0 and (stat.S_ISLNK(link.st_mode) or stat.S_ISREG(link.st_mode)),
              'original root-configured resolver entry')
    link_text = os.readlink(configured) if stat.S_ISLNK(link.st_mode) else None
    target = configured.resolve(strict=True)
    c.require(str(target) == os.path.normpath(str(target)), 'canonical original resolved resolver path')
    descriptor = os.open(target, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    try:
        before = os.fstat(descriptor)
        c.require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and 0 <= before.st_uid < 1000 and
                  not before.st_mode & 0o022 and 0 <= before.st_size <= 65536,
                  'ordinary single-link non-writable system-owner resolver, not necessarily UID0')
        pieces, count = [], 0
        while True:
            block = os.read(descriptor, min(65537 - count, 65536))
            if not block: break
            pieces.append(block); count += len(block)
            c.require(count <= 65536, 'whole original resolver input limit')
        data = b''.join(pieces)
        c.require(len(data) == before.st_size and identity(before) == identity(os.fstat(descriptor)) and
                  identity(before) == identity(target.lstat()), 'stable whole opened resolver target')
    finally:
        os.close(descriptor)
    c.require(identity(configured.lstat()) == identity(link) and configured.resolve(strict=True) == target and
              (link_text is None or os.readlink(configured) == link_text), 'unchanged root-configured resolver link')
    snapshot = root / 'DNS.original.conf'
    with snapshot.open('xb') as stream: stream.write(data)
    snapshot.chmod(0o444)
    metadata = {'configuredPath': str(configured), 'configuredEntry': identity(link), 'readlink': link_text,
                'resolvedPath': str(target), 'originalTarget': identity(before),
                'wholeInput': {'path': str(snapshot), 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()},
                'meaning': 'Actual provided system DNS bytes; original resolved owner may be UID991, never asserted UID0.'}
    c.require(snapshot.stat().st_uid == 0 and snapshot.read_bytes() == data, 'root-owned byte-exact retained snapshot')
    return metadata, data


def resolver_restore(root, runtime, original, data):
    configured = Path(original['configuredPath']); target = Path(original['resolvedPath'])
    c.require(all(getattr(configured.lstat(), key) == value for key, value in original['configuredEntry'].items()) and
              (original['readlink'] is None or os.readlink(configured) == original['readlink']),
              'original configured resolver entry unchanged by overlay')
    restored = {'status': 'original-target-outside-run-no-alteration', 'path': str(target)}
    if target.is_relative_to('/run'):
        relative = target.relative_to('/run')
        c.require(relative.parts and all(x not in ['', '.', '..'] for x in relative.parts), 'exact original target beneath run')
        base = Path('/run'); base_stat = base.lstat(); runtime_stat = runtime.lstat()
        c.require(stat.S_ISDIR(base_stat.st_mode) and (base_stat.st_dev, base_stat.st_ino) ==
                  (runtime_stat.st_dev, runtime_stat.st_ino) and base_stat.st_uid == 65534 and not base_stat.st_mode & 0o022,
                  'same original trusted broker overlay; base UID65534 explicitly retained')
        current = base; parents = []
        for part in relative.parts[:-1]:
            current = current / part; current.mkdir(mode=0o755)
            item = current.lstat()
            c.require(stat.S_ISDIR(item.st_mode) and item.st_uid == 0 and not item.st_mode & 0o022,
                      'fresh root-owned ordinary resolver parent, no preexisting path/link')
            parents.append(str(current))
        descriptor = os.open(target, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o444)
        try:
            os.fchmod(descriptor, 0o444); os.fchown(descriptor, 0, 0)
            remaining = memoryview(data)
            while remaining:
                count = os.write(descriptor, remaining); c.require(count > 0, 'resolver exact whole write'); remaining = remaining[count:]
            item = os.fstat(descriptor)
            c.require(stat.S_ISREG(item.st_mode) and item.st_nlink == 1 and item.st_uid == item.st_gid == 0 and
                      stat.S_IMODE(item.st_mode) == 0o444 and item.st_size == len(data), 'fresh root-owned readonly target')
        finally:
            os.close(descriptor)
        restored = {'status': 'same-original-provided-resolver-bytes-restored-create-only', 'path': str(target),
                    'newOwnerUid': 0, 'newOwnerGid': 0, 'newMode': 0o444, 'freshRootOwnedParents': parents,
                    'originalOwnerUid': original['originalTarget']['st_uid'], 'overlayBaseOwnerUid': base_stat.st_uid}
    c.require(configured.resolve(strict=True) == target and target.read_bytes() == data and
              hashlib.sha256(target.read_bytes()).hexdigest() == original['wholeInput']['sha256'],
              'same configured path resolves to exact original input, no new DNS authority')
    result = {'format': 'zryna.m6-original-resolver-preservation.v1', 'original': original, 'restoration': restored,
              'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest(), 'finishedAt': c.now(), 'acceptance': False,
              'scope': 'Only preservation of original configured DNS in private mount namespace. No new resolver, TLS/host-policy/budget/Engine/kernel/guest change.'}
    c.write_new(root / 'RESOLVER_PRESERVATION.json', result); (root / 'RESOLVER_PRESERVATION.json').chmod(0o444)
    return {'record': c.output_pin(root / 'RESOLVER_PRESERVATION.json'), 'wholeInput': original['wholeInput'],
            'restoration': restored}


def main():
    c.require(os.geteuid() == 0 and len(sys.argv) == 3, 'root observer.py MANIFEST SHA256')
    manifest_path = str(c.trusted(sys.argv[1]))
    m, manifest_pin = c.read_manifest(manifest_path, sys.argv[2], HERE)
    root = c.trusted(m['hostRoot'], True)
    environment = dict(os.environ)
    state = {'format': 'zryna.m6-public-observer-lifecycle.v1', 'acceptance': False,
             'manifest': manifest_pin, 'startedAt': c.now(), 'phaseResults': {},
             'brokerMeaning': 'UID65534 is the byte-forwarding trusted broker, backend private dockerd is UID0'}
    c.write_new(root / 'OBSERVER_INTENT.json', state)
    broker, broker_epoch, streams = None, None, []

    def command(name, argv):
        return c.run_owned(argv, m['source']['path'], 10, root / name, environment)

    def no_new_privileges():
        libc=ctypes.CDLL(None,use_errno=True)
        c.require(libc.prctl(38,1,0,0,0)==0,'NNP before original same-process public authentication/runtime')

    def nobody():
        os.setgroups([])
        os.setgid(65534)
        os.setuid(65534)

    try:
        command('mount-private', ['/usr/bin/mount', '--make-rprivate', '/'])
        original = c.cgroup_mounts()
        c.require(len(original) == 1, 'sole original cgroup mount')
        before, separator, after = original[0].partition(' - ')
        fields, metadata = before.split(), after.split()
        c.require(separator and metadata[0] == 'cgroup2', 'actual cgroup2 filesystem')
        features = metadata[2].split(',')
        c.require(set(features) <= {'rw', 'nsdelegate', 'memory_recursiveprot'} and 'rw' in features,
                  'preserve all observed cgroup superblock features or refuse')
        command('cgroup-unmount', ['/usr/bin/umount', '/sys/fs/cgroup'])
        options = ['ro', 'nosuid', 'nodev', 'noexec'] + [v for v in features if v != 'rw']
        command('cgroup-readonly', ['/usr/bin/mount', '-t', 'cgroup2', '-o', ','.join(options),
                                    'cgroup2', '/sys/fs/cgroup'])
        mounted = c.cgroup_mounts()
        c.require(len(mounted) == 1 and mounted[0].split(' - ')[1].split()[2] == metadata[2] and
                  'ro' in mounted[0].split(' - ')[0].split()[5].split(','), 'sole read-only unchanged-feature mount')
        state.update(originalObserverCgroupMount=original, actualObserverCgroupMount=mounted)
        resolver, resolver_bytes = resolver_snapshot(root)
        state['resolverOriginal'] = resolver
        runtime = root / 'namespace-run'
        runtime.mkdir(mode=0o755)
        os.chown(runtime, 65534, 65534)
        command('socket-overlay', ['/usr/bin/mount', '--bind', str(runtime), '/run'])
        state['resolverPreservation'] = resolver_restore(root, runtime, resolver, resolver_bytes)
        broker_code = root / 'broker-reviewed.py'
        with broker_code.open('xb') as stream:
            stream.write(Path(m['files']['broker']['path']).read_bytes())
        broker_code.chmod(0o444)
        broker_out, broker_err = (root / 'broker.stdout').open('xb'), (root / 'broker.stderr').open('xb')
        streams += [broker_out, broker_err]
        broker = subprocess.Popen([m['files']['python']['path'], '-I', '-B', str(broker_code), str(root / 'engine.sock')],
                                  env=environment, stdin=subprocess.DEVNULL, stdout=broker_out, stderr=broker_err,
                                  preexec_fn=nobody, start_new_session=False)
        # It shares this observer's owned process group, so the outer timeout also kills it.
        broker_epoch = c.epoch(broker.pid)
        state['broker'] = {'pid': broker.pid, 'epoch': broker_epoch,
                           'codeSha256': m['files']['broker']['sha256']}
        until = time.monotonic() + 2
        while True:
            c.require(broker.poll() is None and time.monotonic() < until, 'bounded broker readiness')
            try:
                with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as stream:
                    stream.settimeout(until - time.monotonic())
                    stream.connect('/var/run/docker.sock')
                    _, uid, gid = struct.unpack('3i', stream.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
                    c.require(uid == gid == 65534 and stream.getpeername() == '/var/run/docker.sock', 'actual fixed broker peer')
                    state['broker'].update(peerUid=uid, peerGid=gid, peerName=stream.getpeername())
                break
            except (FileNotFoundError, ConnectionRefusedError):
                time.sleep(0.02)
        before=os.lstat('/var/run/docker.sock');c.require(stat.S_ISSOCK(before.st_mode) and before.st_uid==65534,'fixed original broker socket')
        state['broker']['socketBefore']={'device':before.st_dev,'inode':before.st_ino,'uid':before.st_uid,'gid':before.st_gid,'mode':stat.S_IMODE(before.st_mode)}
        os.chown('/var/run/docker.sock',-1,1000);os.chmod('/var/run/docker.sock',0o660);after=os.lstat('/var/run/docker.sock')
        c.require((after.st_dev,after.st_ino,after.st_uid,after.st_gid,stat.S_IMODE(after.st_mode))==(before.st_dev,before.st_ino,65534,1000,0o660),'same broker inode, selected caller group, exact0660')
        state['broker']['socketAfter']={'device':after.st_dev,'inode':after.st_ino,'uid':after.st_uid,'gid':after.st_gid,'mode':stat.S_IMODE(after.st_mode)}
        for phase, file_key, interpreter, seconds in [
            ('prepare', 'prepare', m['files']['python']['path'], m['limits']['prepareSeconds']),
            ('runtime', 'runtime', m['files']['node']['path'], m['limits']['runtimeSeconds'] + m['limits']['approvalSeconds']),
            ('material-capture', 'materialCapture', m['files']['python']['path'], m['limits']['materialCaptureSeconds'])]:
            c.require(broker.poll() is None and c.epoch(broker.pid) == broker_epoch, 'same live trusted broker')
            c.check_pin(m['files'][file_key])
            c.check_pin(m['files']['python' if phase != 'runtime' else 'node'])
            argv = [interpreter] + ([] if phase == 'runtime' else ['-I', '-B']) + [m['files'][file_key]['path'], manifest_path]
            state['phaseResults'][phase] = c.run_owned(argv, m['source']['path'], seconds, root / phase, environment, preexec_fn=no_new_privileges if phase=='runtime' else None)
        state['status'] = 'public-phases-and-inert-material-retention-exited-zero'
    except BaseException as error:
        state['failure'] = {'type': type(error).__name__, 'message': str(error)}
        raise
    finally:
        try:
            if broker is not None:
                if broker.poll() is None:
                    c.require(c.epoch(broker.pid) == broker_epoch, 'same original broker PID')
                    broker.terminate()
                    try:
                        broker.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        broker.kill()
                        broker.wait(timeout=2)
                state['broker']['returncode'] = broker.returncode
        finally:
            for stream in streams:
                stream.close()
            state.update(finishedAt=c.now(), brokerStdout=c.output_pin(root / 'broker.stdout') if streams else None,
                         brokerStderr=c.output_pin(root / 'broker.stderr') if streams else None)
            c.write_new(root / 'OBSERVER_RESULT.json', state)


if __name__ == '__main__':
    main()
