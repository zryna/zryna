"""PUBLIC-only inert material retention after actual runtime exit; never guest execution."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import subprocess
import sys
import tarfile
import time

HERE = Path(__file__).absolute().parent
spec = importlib.util.spec_from_file_location('capture_common', HERE / 'common.py')
c = importlib.util.module_from_spec(spec)
spec.loader.exec_module(c)
MAXIMUM = 128 * 1024**2


def read(path):
    c.file_pin(path)
    return json.loads(Path(path).read_bytes())


def safe(name):
    p = PurePosixPath(name)
    c.require(name and not p.is_absolute() and str(p) == name and
              all(x not in ['', '.', '..'] for x in name.split('/')), 'canonical archive member')
    return name


def source_snapshot(source):
    rows = []
    for args in [['rev-parse', 'HEAD'], ['rev-parse', 'HEAD^{tree}'],
                 ['status', '--porcelain', '--untracked-files=all']]:
        argv = ['git', '--no-replace-objects', *args]
        r = subprocess.run(argv, cwd=source['path'], capture_output=True, timeout=10, check=True)
        rows.append({'argv': argv, 'stdout': r.stdout.decode(), 'stderr': r.stderr.decode(), 'exitCode': r.returncode})
    c.require(rows[0]['stdout'].strip() == source['commit'] and
              rows[1]['stdout'].strip() == source['tree'] and rows[2]['stdout'] == '', 'exact clean actual checkout')
    return {'source': source, 'commands': rows}


def image_archive(path, metadata):
    rows, names = [], set()
    with tarfile.open(path, 'r:') as archive:
        for item in archive:
            safe(item.name)
            c.require(item.name not in names and len(names) < 512 and (item.isfile() or item.isdir()) and
                      0 <= item.size <= MAXIMUM, 'closed ordinary image archive')
            names.add(item.name)
            if item.isdir():
                c.require(item.size == 0, 'empty ordinary image directory')
                rows.append({'path': item.name, 'directory': True}); continue
            stream = archive.extractfile(item)
            h, count = hashlib.sha256(), 0
            while block := stream.read(65536):
                h.update(block); count += len(block)
            c.require(count == item.size, 'whole image archive leaf')
            if item.name.startswith('blobs/sha256/'):
                c.require(re.fullmatch('blobs/sha256/[a-f0-9]{64}', item.name) and
                          h.hexdigest() == item.name.rsplit('/', 1)[1], 'actual blob identity')
            rows.append({'path': item.name, 'bytes': count, 'sha256': h.hexdigest()})
        c.require(sum(r.get('bytes', 0) for r in rows) <= MAXIMUM, 'whole expanded image bound')
        config_name = 'blobs/sha256/' + metadata['image'].removeprefix('sha256:')
        config = json.load(archive.extractfile(config_name))
        c.require(config['rootfs']['diff_ids'] == metadata['rootFs']['Layers'] and
                  config['config'] == metadata['config'], 'actual image config and full rootfs equality')
        manifest = json.load(archive.extractfile('manifest.json'))
        c.require(len(manifest) == 1 and manifest[0]['Config'] == config_name and
                  manifest[0]['Layers'] == ['blobs/sha256/' + x.removeprefix('sha256:') for x in metadata['rootFs']['Layers']] and
                  all(x in names for x in manifest[0]['Layers']), 'original Docker save manifest')
    return rows


def main(manifest_path):
    c.require(os.geteuid() == 0 and len(sys.argv) == 2, 'root material-capture.py MANIFEST')
    m, manifest_pin = c.read_manifest(manifest_path, os.environ['M6_REMOTE_MANIFEST_SHA256'], HERE)
    c.require(m['approval']['phase'] == 'public' and m['limits']['materialCaptureSeconds'] == 480,
              'explicit separately reviewed public retention bound')
    c.check_pin(m['files']['context']); ctx = read(m['files']['context']['path'])
    host = c.trusted(m['hostRoot'], True); outer = c.trusted(m['approval']['root'], True)
    baseline = c.trusted(ctx['prepare']['root'], True)
    c.require(baseline == outer / 'baseline' and ctx['source'] == m['source'], 'actual public baseline selection')
    caller_root=host/'public-observations';completed_path=caller_root/'caller-journal/ACTUAL_RUN_COMPLETE.json';before=c.caller_file_pin(completed_path,caller_root);completed=json.loads(completed_path.read_bytes());c.require(c.caller_file_pin(completed_path,caller_root)==before,'stable actual caller completion'); command = read(host / 'runtime.json')
    c.require(completed['status'] == 'five-original-public-cohorts-complete' and completed['completed'] == 5 and
              completed['source'] == {'commit': m['source']['commit'], 'tree': m['source']['tree']} and
              command['returncode'] == 0 and command['reaped'] is True and not command['timedOut'] and
              not any(x in command for x in ['error', 'reapError']), 'actual completed/reaped public runtime required')
    actual = read(baseline / 'RESULT.json')
    c.require(actual['status'] == 'prepared-unapproved' and actual['source'] == m['source'], 'actual baseline source')
    out = host / 'material-capture'; out.mkdir(mode=0o755)
    before = source_snapshot(m['source']); c.write_new(out / 'SOURCE_BEFORE.json', before)
    rows, origins, total = [], [], 0

    def retain(path, relative, expected=None):
        nonlocal total
        original = c.file_pin(path)
        if expected is not None:
            c.require(original == expected, 'whole selected original identity')
        destination = out / safe(relative)
        c.require(len(rows) < 2048 and total + original['bytes'] <= 1024**3, 'separate bounded material artifact')
        destination.parent.mkdir(parents=True, exist_ok=True, mode=0o755)
        with Path(path).open('rb') as source, destination.open('xb') as target:
            while block := source.read(65536):
                target.write(block)
        destination.chmod(0o444)
        captured = c.file_pin(destination)
        c.require(c.file_pin(path) == original and captured['bytes'] == original['bytes'] and
                  captured['sha256'] == original['sha256'], 'stable whole original/captured equality')
        rows.append({'relativePath': relative, 'original': original, 'captured': captured})
        total += captured['bytes']
        return captured

    for name in ['RESULT.json', 'MATERIAL_BUILD.json', 'RUNTIME_PLAN_SELECTION.json', 'INPUT_OBSERVATIONS.json']:
        retain(baseline / name, 'baseline/' + name)
    materials = actual['materialBuild']
    for role in ['compiler', 'evaluator']:
        for key, name in [('manifest', 'manifest.json'), ('plan', 'material-plan.json'),
                          ('materialArchive', 'materials.tar.gz'), ('emissionReceipt', 'emission.json')]:
            selected = materials[role][key]
            retain(selected['path'], 'materials/' + role + '/' + name, selected)
    plans = actual['plans']; runtime_root = c.trusted(plans['root'], True)
    c.require(runtime_root == Path(ctx['prepare']['workRoot']) / 'runtime-plans', 'actual selected runtime plan root')
    closure = {}
    for role in ['compiler', 'evaluator', 'ui']:
        selected = plans['plans'][role]
        descriptor = {'path': selected['path'], 'bytes': selected['planBytes'], 'sha256': selected['planSha256']}
        retain(selected['path'], 'plans/' + role + '-plan.json', descriptor)
        plan = read(selected['path'])
        c.require(len(plan['files']) == selected['files'] and plan['bytes'] == selected['bytes'], 'original plan count/bytes')
        for entry in plan['files']:
            c.require(entry['mode'] in [0o444, 0o555], 'original source file runtime mode')
            expected = {'path': entry['sourcePath'], 'bytes': entry['bytes'], 'sha256': entry['sha256']}
            key = (entry['bytes'], entry['sha256'])
            if key not in closure:
                closure[key] = retain(entry['sourcePath'], 'closure/' + entry['sha256'], expected)
            else:
                c.check_pin(expected)
            origins.append({'role': role, 'runtimePath': entry['path'], 'mode': entry['mode'],
                            'original': expected, 'retained': closure[key]})
    for name in ['PLAN_PINS.json', 'SOURCE_ORIGINS.json', 'WIT34_IDENTITY.json', 'IDENTITY.json']:
        retain(runtime_root / name, 'plans/' + name)
    seed = Path(ctx['seed']['directory'])
    for name in ['downloads.json', 'runtime-library-closure.original.json', 'ubuntu-static-source-scope.original.json']:
        retain(seed / 'metadata' / name, 'historical-provenance/' + name)
    # Import only actual authenticated original Engine transport; no image/container mutation APIs.
    engine_dir = Path(m['source']['path']) / 'examples/playground/restricted/daemon/preparation'
    for name in ['engine.py', 'plan.py']:
        p = engine_dir / name; data = p.read_bytes()
        original = subprocess.run(['git', '--no-replace-objects', 'show', m['source']['commit'] + ':' +
                                  str(p.relative_to(m['source']['path']))], cwd=m['source']['path'],
                                  capture_output=True, check=True, timeout=10).stdout
        c.require(data == original, 'actual original Engine transport source')
        retain(p, 'source/' + name)
    sys.path.insert(0, str(engine_dir)); sys.dont_write_bytecode = True
    from engine import Connection, Engine, decode
    # One extra read-only original Engine request; identity must precede daemon shutdown.
    version_started = c.now()
    version_raw, _ = Engine(65534, seconds=30).request('GET', '/version', statuses=(200,))
    version = decode(version_raw); info = read(host / 'engine-info.stdout')
    c.require(version['Version'] == info['ServerVersion'] == '28.4.0' and
              tuple(map(int, version['MinAPIVersion'].split('.'))) <= (1, 49) <=
              tuple(map(int, version['ApiVersion'].split('.'))), 'actual pinned host Engine version and supported API1.49')
    version_path = out / 'docker-engine-version.raw.json'
    with version_path.open('xb') as target: target.write(version_raw)
    version_path.chmod(0o444)
    retain(host / 'engine-info.stdout', 'host-engine-info.stdout')
    retain(host / 'engine-info.json', 'host-engine-info-command.json')
    c.write_new(out / 'ENGINE_IDENTITY_OBSERVATION.json', {
        'format': 'zryna.m6-engine-identity-material.v1', 'version': 1,
        'source': m['source'], 'observation': 'GET /v1.49/version; fixed /var/run/docker.sock peer UID65534',
        'engine': version, 'wholeRawResponse': c.file_pin(version_path),
        'hostEngineInfo': c.file_pin(host / 'engine-info.stdout'), 'startedAt': version_started,
        'finishedAt': c.now(), 'scope': 'Actual readonly transport identity, no binary distribution or runtime acceptance.'})
    exports = []
    for role in ['compiler', 'evaluator']:
        metadata = read(baseline / (role + '-image.json'))
        c.require(metadata == actual['images'][role] and re.fullmatch('sha256:[a-f0-9]{64}', metadata['image']), 'actual image metadata')
        retain(baseline / (role + '-image.json'), 'images/' + role + '-image.json')
        retain(baseline / (role + '-IMAGE_INSPECT.json'), 'images/' + role + '-IMAGE_INSPECT.json')
        path = out / 'images' / (role + '-runtime-image.tar')
        deadline = time.monotonic() + 180; started = c.now()
        connection = Connection(deadline, 65534); response = None; count = 0; h = hashlib.sha256()
        try:
            endpoint = '/v1.49/images/' + metadata['image'] + '/get'
            connection.request('GET', endpoint, headers={'Connection': 'close', 'Accept': 'application/x-tar', 'Accept-Encoding': 'identity'})
            response = connection.getresponse(); headers = response.getheaders()
            c.require(sum(len(k) + len(v) for k, v in headers) <= 16384, 'bounded headers')
            c.write_new(out / 'images' / (role + '-HTTP.json'), {'startedAt': started, 'method': 'GET', 'path': endpoint,
                        'status': response.status, 'headers': headers, 'fixedSocketPeerUid': 65534})
            c.require(response.status == 200 and response.getheader('Content-Encoding', 'identity') == 'identity', 'whole inert export response')
            length = response.getheader('Content-Length')
            c.require(length is None or length.isdecimal() and 0 < int(length) <= MAXIMUM, 'bounded whole image')
            with path.open('xb') as target:
                while block := response.read(65536):
                    c.require(time.monotonic() < deadline and count + len(block) <= MAXIMUM, 'image whole transfer bound')
                    target.write(block); count += len(block); h.update(block)
            c.require(0 < count <= MAXIMUM and time.monotonic() < deadline and
                      (length is None or count == int(length)), 'complete image body')
            path.chmod(0o444); descriptor = c.file_pin(path)
            c.require(descriptor['bytes'] == count and descriptor['sha256'] == h.hexdigest(), 'whole image output reread')
            members = image_archive(path, metadata)
            exports.append({'role': role, 'image': metadata['image'], 'rootFs': metadata['rootFs'],
                            'output': descriptor, 'members': members, 'startedAt': started, 'finishedAt': c.now()})
        finally:
            if response is not None: response.close()
            connection.close()
            if path.exists(): path.chmod(0o444)
    after = source_snapshot(m['source']); c.write_new(out / 'SOURCE_AFTER.json', after)
    c.write_new(out / 'CLOSURE.json', {'source': m['source'], 'files': origins})
    c.write_new(out / 'ACTUAL_IMAGE_EXPORTS.json', {'source': m['source'], 'exports': exports,
                'scope': 'Whole inert Engine GET bytes; no load, container, guest, approval or qualification claim.'})
    generated = {str(p.relative_to(out)) for p in out.rglob('*') if p.is_file()}
    c.require(len(generated) <= 2048, 'closed separate inventory')
    inventory = [dict(relativePath=name, **c.file_pin(out / name)) for name in sorted(generated)]
    c.require(sum(p['bytes'] for p in inventory) <= 1024**3, 'whole separate inventory <=1GiB')
    c.write_new(out / 'MANIFEST.json', {'format': 'zryna.m6-remote-material-capture.v1', 'source': m['source'],
                'manifest': manifest_pin, 'completedRuntime': c.file_pin(host / 'runtime.json'), 'files': inventory,
                'copies': rows, 'sourceBefore': before, 'sourceAfter': after, 'acceptance': False,
                'scope': 'Complete retained inert originals and read-only exports after actual public runtime exit; original final17 validation remains required.'})
    print(json.dumps({'status': 'inert-material-capture-complete', 'manifest': c.file_pin(out / 'MANIFEST.json'),
                      'files': len(inventory), 'bytes': sum(p['bytes'] for p in inventory), 'acceptance': False}))


if __name__ == '__main__':
    main(sys.argv[1])
