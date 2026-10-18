"""Probe unchanged Engine and cgroup reader in an isolated host mount namespace."""
import hashlib
import json
import os
from pathlib import Path
import socket
import struct
import subprocess
import sys
import time

root, source, cid = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3]
assert os.geteuid() == 0 and len(cid) == 64
subprocess.run(['mount', '--make-rprivate', '/'], check=True)
subprocess.run(['mount', '--bind', '/sys/fs/cgroup', '/sys/fs/cgroup'], check=True)
subprocess.run(['mount', '-o', 'remount,bind,ro', '/sys/fs/cgroup'], check=True)
runtime = root / 'namespace-run'
runtime.mkdir(mode=0o755)
os.chown(runtime, 65534, 65534)
subprocess.run(['mount', '--bind', str(runtime), '/run'], check=True)
broker_code = Path(__file__).with_name('broker.py')


def nobody():
    os.setgroups([])
    os.setgid(65534)
    os.setuid(65534)


broker = subprocess.Popen(['/usr/bin/python3', '-I', '-B', str(broker_code), str(root / 'engine.sock')],
                          preexec_fn=nobody, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
result = {'acceptance': False, 'source': 'unchanged original helper modules',
          'peerMeaning': 'UID65534 is the trusted duplex broker; the private real dockerd remains UID0',
          'brokerSha256': hashlib.sha256(broker_code.read_bytes()).hexdigest()}
try:
    for _ in range(100):
        if Path('/var/run/docker.sock').exists():
            break
        assert broker.poll() is None, 'trusted transport exited'
        time.sleep(0.02)
    sys.path.insert(0, str(source / 'examples/playground/restricted/daemon/preparation'))
    sys.path.insert(0, str(source / 'examples/playground/restricted/daemon/ui-qualification'))
    from engine import Engine
    import owner_cgroup
    import owner_kernel
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
        connection.connect('/var/run/docker.sock')
        _, uid, gid = struct.unpack('3i', connection.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
        result.update(peerUid=uid, peerGid=gid, peerName=connection.getpeername())
        assert uid == 65534 and connection.getpeername() == '/var/run/docker.sock'
    engine = Engine(65534, seconds=20)
    result['engineVersionViaOriginalApi149'] = engine.json('GET', '/version')
    state = engine.json('GET', '/containers/' + cid + '/json')
    pid = state['State']['Pid']
    assert state['Id'] == cid and state['State']['Running'] and pid > 1
    membership = Path(f'/proc/{pid}/cgroup').read_text()
    result.update(containerId=cid, pid=pid, membership=membership)

    deadline = time.monotonic() + 10
    result['proc'] = {name: owner_kernel.read(f'/proc/{pid}/{name}', deadline, 262144)
                      for name in ('status', 'stat', 'cgroup', 'mountinfo')}
    owner_kernel.security(owner_kernel.fields(result['proc']['status']), pid)
    result['namespaces'] = owner_kernel.namespace_links(pid, deadline)
    result['network'] = owner_kernel.network(pid, deadline, result['namespaces'])
    result['mapping'] = owner_cgroup.mapping(owner_kernel.read, owner_kernel.numbers, membership, cid, deadline)
    result['status'] = 'fixed-peer-and-original-cgroup-mapping-passed'
finally:
    broker.terminate()
    try:
        out, err = broker.communicate(timeout=5)
    except subprocess.TimeoutExpired:
        broker.kill()
        out, err = broker.communicate(timeout=2)
    finally:
        (root / 'namespace-result.json').write_text(json.dumps(result, indent=2) + '\n')
    (root / 'broker.stdout').write_bytes(out)
    (root / 'broker.stderr').write_bytes(err)
