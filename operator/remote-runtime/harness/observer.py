"""One private observer namespace: pinned preparation, approval helper, runtime."""
import importlib.util
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


def main():
    c.require(os.geteuid() == 0 and len(sys.argv) == 3, 'root observer.py MANIFEST SHA256')
    manifest_path = str(c.trusted(sys.argv[1]))
    m, manifest_pin = c.read_manifest(manifest_path, sys.argv[2], HERE)
    root = c.trusted(m['hostRoot'], True)
    environment = dict(os.environ)
    state = {'format': 'zryna.m6-private-observer-lifecycle.v1', 'acceptance': False,
             'manifest': manifest_pin, 'startedAt': c.now(), 'phaseResults': {},
             'brokerMeaning': 'UID65534 is the byte-forwarding trusted broker, backend private dockerd is UID0'}
    c.write_new(root / 'OBSERVER_INTENT.json', state)
    broker, broker_epoch, streams = None, None, []

    def command(name, argv):
        return c.run_owned(argv, m['source']['path'], 10, root / name, environment)

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
        runtime = root / 'namespace-run'
        runtime.mkdir(mode=0o755)
        os.chown(runtime, 65534, 65534)
        command('socket-overlay', ['/usr/bin/mount', '--bind', str(runtime), '/run'])
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
        for phase, file_key, interpreter, seconds in [
            ('prepare', 'prepare', m['files']['python']['path'], m['limits']['prepareSeconds']),
            ('approval', 'waitApproval', m['files']['python']['path'], m['limits']['approvalSeconds']),
            ('runtime', 'runtime', m['files']['node']['path'], m['limits']['runtimeSeconds'])]:
            c.require(broker.poll() is None and c.epoch(broker.pid) == broker_epoch, 'same live trusted broker')
            c.check_pin(m['files'][file_key])
            c.check_pin(m['files']['python' if phase != 'runtime' else 'node'])
            argv = [interpreter] + ([] if phase == 'runtime' else ['-I', '-B']) + [m['files'][file_key]['path'], manifest_path]
            state['phaseResults'][phase] = c.run_owned(argv, m['source']['path'], seconds, root / phase, environment)
            if phase == 'prepare':
                c.write_new(root / 'PREPARED.json', {'format': 'zryna.m6-host-preparation-completed.v1',
                            'approval': False, 'acceptance': False, 'manifest': manifest_pin,
                            'prepareCommand': c.output_pin(root / 'prepare.json'), 'finishedAt': c.now()})
        state['status'] = 'all-three-pinned-phase-commands-exited-zero'
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
