"""Own one ephemeral Engine and observer; no product or approval logic."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import time

HERE = Path(__file__).absolute().parent
spec = importlib.util.spec_from_file_location('remote_host_common', HERE / 'common.py')
c = importlib.util.module_from_spec(spec)
spec.loader.exec_module(c)


def main():
    c.require(os.geteuid() == 0 and len(sys.argv) == 3, 'root host.py MANIFEST SHA256')
    manifest_path = str(c.trusted(sys.argv[1]))
    m, manifest_pin = c.read_manifest(manifest_path, sys.argv[2], HERE)
    root = Path(m['hostRoot'])
    root.mkdir(mode=0o755)
    environment = dict(os.environ)
    for key in ['DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_API_VERSION', 'DOCKER_TLS_VERIFY', 'DOCKER_CERT_PATH']:
        environment.pop(key, None)
    environment.update(M6_REMOTE_MANIFEST_SHA256=sys.argv[2], M6_REMOTE_HOST_ROOT=str(root))
    state = {'format': 'zryna.m6-ephemeral-host-lifecycle.v1', 'acceptance': False,
             'manifest': manifest_pin, 'sourceSelectedByOperator': m['source'],
             'startedAt': c.now(), 'originalCgroupMount': c.cgroup_mounts(), 'ownedProcesses': {}}
    c.write_new(root / 'HOST_INTENT.json', state)
    processes, logs = {}, []

    def start(key, argv):
        stream = (root / (key + '.log')).open('xb')
        logs.append(stream)
        process = subprocess.Popen(argv, env=environment, stdin=subprocess.DEVNULL,
                                   stdout=stream, stderr=subprocess.STDOUT, start_new_session=True)
        processes[key] = (process, c.epoch(process.pid))
        state['ownedProcesses'][key] = {'pid': process.pid, 'epoch': processes[key][1], 'argv': argv}
        return process

    def socket_ready(process, socket, seconds):
        until = time.monotonic() + seconds
        while time.monotonic() < until:
            c.require(process.poll() is None, 'private daemon exited during startup')
            if socket.exists():
                return
            time.sleep(0.05)
        raise TimeoutError('private daemon socket startup')

    try:
        python = m['files']['python']['path']
        c.run_owned([python, '-I', '-B', m['files']['engineAcquire']['path'], str(root)],
                    m['source']['path'], 260, root / 'engine-acquisition', environment)
        bins = root / 'engine-bin'
        environment['PATH'] = str(bins) + ':' + environment['PATH']
        containerd_socket = root / 'containerd.sock'
        config = root / 'containerd.toml'
        with config.open('x') as stream:
            stream.write('version = 2\ndisabled_plugins = ["io.containerd.grpc.v1.cri"]\n' +
                         'root = ' + json.dumps(str(root / 'containerd-root')) + '\n' +
                         'state = ' + json.dumps(str(root / 'containerd-state')) + '\n' +
                         '[grpc]\naddress = ' + json.dumps(str(containerd_socket)) + '\n')
        containerd = start('containerd', [str(bins / 'containerd'), '--config=' + str(config)])
        socket_ready(containerd, containerd_socket, 10)
        daemon_config = root / 'daemon.json'
        with daemon_config.open('x') as stream:
            stream.write('{}\n')
        socket = root / 'engine.sock'
        nonce = m['run']['id'] + '-' + m['run']['attempt']
        daemon = start('dockerd', [str(bins / 'dockerd'), '--host=unix://' + str(socket), '--group=nogroup',
                                 '--config-file=' + str(daemon_config), '--data-root=' + str(root / 'data'),
                                 '--exec-root=' + str(root / 'exec'), '--pidfile=' + str(root / 'dockerd.pid'),
                                 '--bridge=none', '--iptables=false', '--ip6tables=false', '--ip-forward=false',
                                 '--ip-masq=false', '--containerd=' + str(containerd_socket),
                                 '--containerd-namespace=m6-runtime-' + nonce,
                                 '--containerd-plugins-namespace=m6-runtime-plugins-' + nonce,
                                 '--exec-opt=native.cgroupdriver=cgroupfs'])
        socket_ready(daemon, socket, 20)
        c.run_owned([str(bins / 'docker'), '--host=unix://' + str(socket), 'info', '--format', '{{json .}}'],
                    m['source']['path'], 30, root / 'engine-info', environment)
        info = json.loads((root / 'engine-info.stdout').read_bytes())
        c.require(info['CgroupDriver'] == 'cgroupfs' and info['CgroupVersion'] == '2' and
                  info['DockerRootDir'] == str(root / 'data') and
                  info['Containerd']['Address'] == str(containerd_socket) and
                  info['Containerd']['Namespaces'] == {'Containers': 'm6-runtime-' + nonce,
                                                       'Plugins': 'm6-runtime-plugins-' + nonce}, 'private actual Engine identity')
        for key, (process, selected_epoch) in processes.items():
            actual = [v.decode() for v in Path(f'/proc/{process.pid}/cmdline').read_bytes().split(b'\0') if v]
            c.require(c.epoch(process.pid) == selected_epoch and actual == state['ownedProcesses'][key]['argv'],
                      'same actual private daemon command/epoch')
        c.run_owned(['/usr/bin/unshare', '--mount', '--', python, '-I', '-B',
                     m['files']['observer']['path'], manifest_path, sys.argv[2]],
                    m['source']['path'], sum(m['limits'].values()) + 60, root / 'observer', environment)
        state['observer'] = json.loads((root / 'OBSERVER_RESULT.json').read_bytes())
        c.require(state['observer']['status'] == 'all-three-pinned-phase-commands-exited-zero', 'observer did not complete')
        c.require(c.cgroup_mounts() == state['originalCgroupMount'], 'original host cgroup mount changed')
        state['status'] = 'owned-host-phases-completed-no-acceptance-verdict'
    except BaseException as error:
        state['failure'] = {'type': type(error).__name__, 'message': str(error)}
        raise
    finally:
        try:
            state['stops'] = {}
            state['phaseGroupStops'] = {}
            # These exact journals are written before each fixed phase is waited.
            # A killed observer cannot strand a phase's separate process group.
            for phase in ['prepare', 'approval', 'runtime']:
                started = root / (phase + '.started.json')
                if started.exists():
                    try:
                        row = json.loads(started.read_bytes())
                        completed = root / (phase + '.json')
                        if completed.exists():
                            final = json.loads(completed.read_bytes())
                            c.require(final.get('reaped') and not final.get('reapError'), 'phase group shutdown unconfirmed')
                            state['phaseGroupStops'][phase] = {'completedJournal': c.output_pin(completed), 'nativeCleanupProved': False}
                        else:
                            c.kill_owned_group(row['pid'], row['epoch'])
                            state['phaseGroupStops'][phase] = {'signalAttempted': True, 'nativeCleanupProved': False}
                    except BaseException as error:
                        state['phaseGroupStops'][phase] = {'error': {'type': type(error).__name__, 'message': str(error)}}
            for key in ['dockerd', 'containerd']:
                if key in processes:
                    state['stops'][key] = c.stop_owned(*processes[key], 20 if key == 'dockerd' else 10)
        finally:
            for stream in logs:
                stream.close()
            state.update(finishedAt=c.now(), originalCgroupMountAfter=c.cgroup_mounts())
            state['originalCgroupMountUnchanged'] = state['originalCgroupMountAfter'] == state['originalCgroupMount']
            c.write_new(root / 'HOST_RESULT.json', state)
        c.require(all(v['reaped'] for v in state['stops'].values()) and
                  all('error' not in v for v in state['phaseGroupStops'].values()) and
                  state['originalCgroupMountUnchanged'], 'host lifecycle cleanup/invariance unconfirmed')


if __name__ == '__main__':
    main()
