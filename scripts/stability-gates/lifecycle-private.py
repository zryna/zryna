"""Fixed native private handoff; accepts data, never source, callbacks or argv selectors."""
import contextlib
import hashlib
import json
import os
from pathlib import Path
import runpy
import sys
import time

root = Path(__file__).resolve().parents[2]
entry = runpy.run_path(str(root / 'examples/playground/restricted/daemon/lifecycle-owner/owner.py'))
load = entry['_load']
value = load('plan').decode(sys.stdin.buffer.read(4 * 1024**2 + 1))
load('plan').exact(value, ['work', 'preparation'])
base = Path(value['work'])
assert base.parent == Path('/tmp') and base.name.startswith('zryna-lifecycle-')
assert base.lstat().st_uid == 0 and base.lstat().st_mode & 0o777 == 0o700
assert os.getuid() == os.geteuid() == 0


def save(name, value):
    with (base / name).open('xb') as stream:
        stream.write(value if type(value) is bytes else (json.dumps(value) + '\n').encode())


observer = runpy.run_path(str(root / 'scripts/stability-gates/collect-native-host-facts.py'))
with (base / 'native.json').open('x') as stream, contextlib.redirect_stdout(stream):
    observer['main']()
native = (base / 'native.json').read_bytes()
requests, anchor = [], []
engine = load('lifecycle_engine').Engine({}, time.monotonic() + 180, requests, anchor)
try:
    info, version = engine.json('GET', '/info'), engine.json('GET', '/version')
finally:
    save('engine-discovery.json', requests)
expected = {'id': info['ID'], 'version': version['Version'], 'apiVersion': version['ApiVersion'],
            'cgroupDriver': info['CgroupDriver'], 'dockerRootDir': info['DockerRootDir']}
selection = value['preparation']
image, materials, manifest = load('preparation_plan').validate(selection)
source = next(row for row in selection['runtimeSources'] if row['path'] == '/resources/materials-manifest.json')
assert Path(source['sourcePath']) == base / 'manifest.json'
save('manifest.json', manifest)
prepared = load('preparation').prepare(selection, expected, anchor)
save('preparation.json', prepared)
assert prepared['failure'] is None
plan = selection['plan']
selected = {'plan': plan, 'planSha256': selection['planSha256'], 'runtime': selection['runtime'],
            'image': prepared['image'], 'materials': prepared['materials']['selection'], 'engine': expected,
            'nativeVm': {'raw': native, 'sha256': hashlib.sha256(native).hexdigest()},
            'capture': {'source': plan['source'], 'compiler': plan['prebuild']['artifacts']['compiler'],
                        'test': plan['prebuild']['artifacts']['test'], 'raw': plan['prebuild']['raw']}}
result = entry['run_pair'](selected)
save('pair.json', result)
assert result['completed'] and all(phase['cleanup']['confirmed'] for phase in result['phases'])
