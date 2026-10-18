"""Acquire one exact supported Engine distribution for the ephemeral host."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile

root = Path(sys.argv[1])
assert os.geteuid() == 0
archive = root / 'docker-28.4.0.tgz'
assert not archive.exists()
url = 'https://download.docker.com/linux/static/stable/x86_64/docker-28.4.0.tgz'
subprocess.run(['curl', '--disable', '--fail', '--location', '--silent', '--show-error',
                '--proto', '=https', '--max-time', '120', '--max-filesize', '82093093',
                url, '--output', str(archive)], check=True, timeout=125)
assert archive.stat().st_size == 82093093
assert hashlib.file_digest(archive.open('rb'), 'sha256').hexdigest() == '21516934188f06d0e5f232cbde8112701f6d82899016240bc7a5d619f6b0059c'
expected = {'dockerd', 'docker', 'docker-init', 'containerd', 'ctr', 'runc',
            'containerd-shim-runc-v2', 'docker-proxy'}
directory = root / 'engine-bin'
directory.mkdir(mode=0o755)
pins = {}
with tarfile.open(archive, 'r:gz') as stream:
    members = stream.getmembers()
    assert len(members) == 9 and members[0].name == 'docker' and members[0].isdir()
    assert {m.name for m in members[1:]} == {'docker/' + name for name in expected}
    assert all(m.isfile() and 0 < m.size <= 100 * 1024**2 for m in members[1:])
    for member in members[1:]:
        data = stream.extractfile(member).read(member.size + 1)
        assert len(data) == member.size
        target = directory / member.name.split('/')[1]
        with target.open('xb') as output:
            output.write(data)
        target.chmod(0o555)
        pins[target.name] = {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}
print(json.dumps({'url': url, 'archiveBytes': 82093093,
                  'archiveSha256': '21516934188f06d0e5f232cbde8112701f6d82899016240bc7a5d619f6b0059c',
                  'binaries': pins}))
