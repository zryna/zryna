"""Bounded ephemeral host compatibility probe, never product acceptance evidence."""
import hashlib
import json
import os
from pathlib import Path
import secrets
import signal
import subprocess
import sys
import time

assert os.geteuid() == 0
root, source = Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()
root.mkdir(mode=0o755)
os.chown(root, 0, 65534)
nonce = secrets.token_hex(16)
name = 'zryna410-host-probe-' + nonce
sock = root / 'engine.sock'
daemon_log = (root / 'dockerd.log').open('xb')
environment = dict(os.environ)
for key in ('DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_API_VERSION', 'DOCKER_TLS_VERIFY', 'DOCKER_CERT_PATH'):
    environment.pop(key, None)
result = {'format': 'zryna410.host-compatibility-probe.v1', 'acceptance': False,
          'nonce': nonce, 'name': name, 'cpuAllowed': sorted(os.sched_getaffinity(0)),
          'sourceCommit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=source, text=True).strip(),
          'helperPins': {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                         for p in Path(__file__).parent.glob('*.py')}}
result['originalCgroupMount'] = [line for line in Path('/proc/self/mountinfo').read_text().splitlines()
                                if line.split(' - ')[0].split()[4] == '/sys/fs/cgroup']
result['originalHostProcMount'] = [line for line in Path('/proc/self/mountinfo').read_text().splitlines() if line.split(' - ')[0].split()[4] == '/proc']
daemon, containerd, image, cid = None, None, None, None
attempted = False
index = 0
engine_docker = '/usr/bin/docker'


def run(argv, timeout=30, ok=(0,)):
    global index
    index += 1
    process = subprocess.Popen(argv, env=environment, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               stdin=subprocess.DEVNULL, start_new_session=True)
    try:
        out, err = process.communicate(timeout=timeout)
    except subprocess.TimeoutExpired as error:
        # This group contains only the task-owned command and its descendants.
        os.killpg(process.pid, signal.SIGKILL)
        out, err = process.communicate(timeout=5)
        (root / f'command-{index}.json').write_text(json.dumps({'argv': argv, 'timeoutSeconds': timeout}) + '\n')
        (root / f'command-{index}.stdout').write_bytes(out)
        (root / f'command-{index}.stderr').write_bytes(err)
        raise
    (root / f'command-{index}.json').write_text(json.dumps({'argv': argv, 'returncode': process.returncode}) + '\n')
    (root / f'command-{index}.stdout').write_bytes(out)
    (root / f'command-{index}.stderr').write_bytes(err)
    assert process.returncode in ok, f'command {index} exit {process.returncode}'
    return out


def docker(*args, timeout=30, ok=(0,)):
    return run([engine_docker, '--host=unix://' + str(sock), *args], timeout, ok)


try:
    selected = json.loads(Path(__file__).with_name('RUNC_SELECTION.json').read_bytes())
    assert selected['engineAcquirer'] is not None and selected['binary'] is not None, 'independently verified official runc selection required'
    acquirer = Path(__file__).with_name('engine_acquire.py'); assert hashlib.sha256(acquirer.read_bytes()).hexdigest() == selected['engineAcquirer']['sha256'] and acquirer.stat().st_size == selected['engineAcquirer']['bytes']
    result['engineAcquisition'] = json.loads(run(['/usr/bin/python3', '-I', '-B',
                                                str(Path(__file__).with_name('engine_acquire.py')), str(root)], timeout=260))
    result['selectedRunc'] = selected['binary']
    assert hashlib.sha256((root/'engine-bin/runc').read_bytes()).hexdigest() == selected['binary']['sha256'] and (root/'engine-bin/runc').stat().st_size == selected['binary']['bytes']
    engine_bins = root / 'engine-bin'
    environment['PATH'] = str(engine_bins) + ':' + environment['PATH']
    engine_docker = str(engine_bins / 'docker')
    containerd_config = root / 'containerd.toml'
    containerd_config.write_text('version = 2\ndisabled_plugins = ["io.containerd.grpc.v1.cri"]\n'
                                + 'root = ' + json.dumps(str(root / 'containerd-root')) + '\n'
                                + 'state = ' + json.dumps(str(root / 'containerd-state')) + '\n'
                                + '[grpc]\naddress = ' + json.dumps(str(root / 'containerd.sock')) + '\n')
    containerd_log = (root / 'containerd.log').open('xb')
    containerd_command = [str(engine_bins / 'containerd'), '--config=' + str(containerd_config)]
    containerd = subprocess.Popen(containerd_command, env=environment, stdout=containerd_log,
                                 stderr=subprocess.STDOUT, start_new_session=True)
    result['containerd'] = {'pid': containerd.pid, 'argv': containerd_command,
                            'socket': str(root / 'containerd.sock'),
                            'startTicks': Path(f'/proc/{containerd.pid}/stat').read_text().rsplit(')', 1)[1].split()[19]}
    for _ in range(100):
        assert containerd.poll() is None, 'private task-owned containerd exited'
        if (root / 'containerd.sock').exists():
            break
        time.sleep(0.05)
    else:
        raise TimeoutError('private containerd socket readiness')
    config = root / 'daemon.json'
    config.write_text('{}\n')
    command = [str(engine_bins / 'dockerd'), '--host=unix://' + str(sock), '--group=nogroup',
               '--config-file=' + str(config),
               '--data-root=' + str(root / 'data'), '--exec-root=' + str(root / 'exec'),
               '--pidfile=' + str(root / 'dockerd.pid'), '--bridge=none', '--iptables=false',
               '--ip6tables=false', '--ip-forward=false', '--ip-masq=false',
               '--containerd=' + str(root / 'containerd.sock'),
               '--containerd-namespace=m6-probe-' + nonce,
               '--containerd-plugins-namespace=m6-probe-plugins-' + nonce,
               '--exec-opt=native.cgroupdriver=cgroupfs']
    result['daemonCommand'] = command
    daemon = subprocess.Popen(command, env=environment, stdout=daemon_log, stderr=subprocess.STDOUT,
                              start_new_session=True)
    result['daemonPid'] = daemon.pid
    result['daemonStartTicks'] = Path(f'/proc/{daemon.pid}/stat').read_text().rsplit(')', 1)[1].split()[19]
    for _ in range(200):
        assert daemon.poll() is None, 'task-owned daemon exited'
        if sock.exists():
            break
        time.sleep(0.1)
    else:
        raise TimeoutError('task-owned daemon readiness')
    # A socket can exist before daemon initialization finishes. One bounded
    # startup request waits for readiness; this is not a guest operation budget.
    info = json.loads(docker('info', '--format', '{{json .}}', timeout=30))
    result['engine'] = info
    assert info['CgroupDriver'] == 'cgroupfs' and info['CgroupVersion'] == '2'
    result['containerd']['actualArgv'] = [v.decode() for v in Path(f'/proc/{containerd.pid}/cmdline').read_bytes().split(b'\0') if v]
    assert result['containerd']['actualArgv'] == containerd_command
    assert Path(f'/proc/{containerd.pid}/stat').read_text().rsplit(')', 1)[1].split()[19] == result['containerd']['startTicks']
    assert {0, 1}.issubset(os.sched_getaffinity(0)), 'fixed UI CPUs unavailable'
    docker('pull', 'busybox:1.37.0', timeout=120)
    image = json.loads(docker('image', 'inspect', 'busybox:1.37.0'))[0]['Id']
    result['probeImageId'] = image
    assert image == 'sha256:30ecbe1509090b38ef3260f3609d64d8261230464eaa4d235901db496e9647fb', 'actual original trusted diagnostic image identity differs'
    assert 'name=apparmor' in info['SecurityOptions'] and Path('/sys/module/apparmor/parameters/enabled').read_text().strip() == 'Y', 'AppArmor must remain enabled'
    attempted = True
    run(['unshare', '--mount', '--', '/usr/bin/python3', '-I', '-B',
         str(Path(__file__).with_name('namespace_probe.py')), str(root), str(source), image, name, nonce], timeout=80)
    observed = json.loads((root/'namespace-result.json').read_bytes())
    assert observed['observerExit'] == 0 and observed['actual']['status'] == 'fixed-proc-readonly-apparmor-exec-passed-not-product-qualification'
    cid = observed['actual']['cid']
    actual_mount = [line for line in Path('/proc/self/mountinfo').read_text().splitlines()
                    if line.split(' - ')[0].split()[4] == '/sys/fs/cgroup']
    assert actual_mount == result['originalCgroupMount'], 'original host cgroup mount features changed'
    assert [line for line in Path('/proc/self/mountinfo').read_text().splitlines() if line.split(' - ')[0].split()[4] == '/proc'] == result['originalHostProcMount']
    result['status'] = 'one-fixed-proc-readonly-apparmor-exec-measured-not-product-acceptance'
except BaseException as error:
    result['failure'] = {'type': type(error).__name__, 'message': str(error)}
    raise
finally:
    try:
        if attempted and daemon is not None and daemon.poll() is None:
            raw = docker('inspect', cid or name, ok=(0,1))
            states = json.loads(raw)
            if states:
                assert len(states) == 1
                state = states[0]
                assert (cid is None or state['Id'] == cid) and state['Name'] == '/' + name
                assert state['Image'] == image and state['Config']['Labels'] == {'zryna410.host-probe': nonce}
                cid = state['Id']; result['cleanupInspect'] = state
                docker('rm', '--force', cid)
                assert json.loads(docker('inspect', cid, ok=(1,))) == []
            result['cleanupConfirmed'] = True
    except BaseException as error:
        result['cleanupFailure'] = {'type': type(error).__name__, 'message': str(error)}
        raise
    finally:
        try:
            if daemon is not None and daemon.poll() is None:
                assert Path(f'/proc/{daemon.pid}/stat').read_text().rsplit(')', 1)[1].split()[19] == result['daemonStartTicks']
                daemon.send_signal(signal.SIGTERM)
                try:
                    daemon.wait(timeout=20)
                except subprocess.TimeoutExpired:
                    os.killpg(daemon.pid, signal.SIGKILL)
                    daemon.wait(timeout=5)
        finally:
            try:
                if containerd is not None and containerd.poll() is None:
                    assert Path(f'/proc/{containerd.pid}/stat').read_text().rsplit(')', 1)[1].split()[19] == result['containerd']['startTicks']
                    containerd.terminate()
                    try:
                        containerd.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        os.killpg(containerd.pid, signal.SIGKILL)
                        containerd.wait(timeout=5)
            finally:
                daemon_log.close()
                if 'containerd_log' in globals():
                    containerd_log.close()
                result['daemonStopped'] = daemon is not None and daemon.poll() is not None
                result['containerdStopped'] = containerd is not None and containerd.poll() is not None
                result['originalCgroupMountAfter'] = [line for line in Path('/proc/self/mountinfo').read_text().splitlines()
                                                      if line.split(' - ')[0].split()[4] == '/sys/fs/cgroup']
                result['originalCgroupMountUnchanged'] = result['originalCgroupMountAfter'] == result['originalCgroupMount']
                result['originalHostProcMountAfter'] = [line for line in Path('/proc/self/mountinfo').read_text().splitlines() if line.split(' - ')[0].split()[4] == '/proc']
                result['originalHostProcMountUnchanged'] = result['originalHostProcMountAfter'] == result['originalHostProcMount']
                (root / 'RESULT.json').write_text(json.dumps(result, indent=2) + '\n')
