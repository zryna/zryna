"""Private source-bound operator input checks, not publication authority."""
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess

OLD_COMMIT = 'b3be654c452ea51f2c136a75d2eb89995eb4bd66'
OLD_TREE = '3fa38d7b62b06a64ac5385d610be164f9b1961b8'


def need(value, message):
    if not value:
        raise ValueError('REMOTE-RUNTIME: ' + message)


def exact(value, keys):
    need(type(value) is dict and set(value) == set(keys), 'closed context fields')


def decode(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            need(key not in result, 'duplicate key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: need(False, 'nonfinite JSON'))


def ordinary(path):
    path = Path(path)
    need(path.is_absolute() and path.resolve() == path, 'physical absolute path')
    for entry in [*reversed(path.parents), path]:
        item = entry.lstat()
        need(item.st_uid == 0 and not item.st_mode & 0o022 and not stat.S_ISLNK(item.st_mode), 'trusted root-owned path')
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK)
    try:
        a = os.fstat(descriptor)
        need(stat.S_ISREG(a.st_mode) and a.st_size <= 128 * 1024**2, 'bounded ordinary file')
        with os.fdopen(descriptor, 'rb', closefd=False) as stream:
            raw = stream.read(a.st_size + 1)
        b = os.fstat(descriptor)
        need((a.st_dev, a.st_ino, a.st_size, a.st_mtime_ns, a.st_ctime_ns) ==
             (b.st_dev, b.st_ino, b.st_size, b.st_mtime_ns, b.st_ctime_ns) and len(raw) == a.st_size, 'stable whole bytes')
        return raw
    finally:
        os.close(descriptor)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def pin(path):
    path = Path(path)
    need(path.is_absolute() and path.resolve() == path, 'physical pin path')
    for entry in [*reversed(path.parents), path]:
        item = entry.lstat()
        need(item.st_uid == 0 and not item.st_mode & 0o022 and not stat.S_ISLNK(item.st_mode), 'trusted pin ancestry')
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK)
    try:
        a = os.fstat(descriptor)
        need(stat.S_ISREG(a.st_mode) and a.st_size <= 512 * 1024**2, 'bounded whole tool/stream pin')
        with os.fdopen(descriptor, 'rb', closefd=False) as stream:
            digest = hashlib.file_digest(stream, 'sha256').hexdigest()
        b = os.fstat(descriptor)
        need((a.st_dev,a.st_ino,a.st_size,a.st_mtime_ns,a.st_ctime_ns) ==
             (b.st_dev,b.st_ino,b.st_size,b.st_mtime_ns,b.st_ctime_ns), 'stable whole pin')
        return {'path': str(path), 'bytes': a.st_size, 'sha256': digest}
    finally:
        os.close(descriptor)


def checked(value):
    exact(value, ['path', 'bytes', 'sha256'])
    need(pin(value['path']) == value, 'selected whole-file pin')
    return Path(value['path'])


def save(path, value):
    raw = (json.dumps(value, sort_keys=True, indent=2) + '\n').encode()
    with Path(path).open('xb') as stream:
        stream.write(raw)
    Path(path).chmod(0o400)


def git_source(source):
    def git(*args):
        return subprocess.check_output(['git', *args], cwd=source['path'], timeout=10).decode().strip()
    need(git('rev-parse', 'HEAD') == source['commit'] and git('rev-parse', 'HEAD^{tree}') == source['tree'] and
         git('status', '--porcelain') == '', 'actual frozen clean Git checkout')


def context_inputs(context_path, manifest_path):
    need(os.geteuid() == 0, 'trusted operator UID0; guest remains65534')
    ctx, manifest = decode(ordinary(context_path)), decode(ordinary(manifest_path))
    exact(ctx, ['format', 'version', 'source', 'candidateReview', 'frozenMain', 'seed', 'prepare', 'runtime'])
    need(ctx['format'] == 'zryna.m6-remote-runtime-context.v1' and ctx['version'] == 1, 'context domain')
    need(ctx['source'] == manifest['source'] and ctx['source']['path'] == '/workspace/zryna' and
         ctx['source']['commit'] != OLD_COMMIT, 'actual new-source selection, never historical default')
    need(pin(context_path) == {**manifest['files']['context'], 'path': str(context_path)}, 'byte-exact selected context copy')
    need(os.environ.get('GITHUB_RUN_ID') == manifest['run']['id'] and
         os.environ.get('GITHUB_RUN_ATTEMPT') == manifest['run']['attempt'] and
         os.environ.get('GITHUB_SHA') == manifest['approval']['operatorCommit'], 'actual job/operator context')
    root = Path(ctx['prepare']['root'])
    need(str(root) == manifest['approval']['root'] and
         Path(context_path) == root / 'REMOTE_RUNTIME_CONTEXT.json', 'actual private composition/context root')
    for key in ['candidateReview', 'frozenMain']:
        checked(ctx[key])
    frozen = decode(ordinary(ctx['frozenMain']['path']))
    need(frozen['commit'] == ctx['source']['commit'] and frozen['tree'] == ctx['source']['tree'], 'actual frozen source')
    runtime = ctx['runtime']
    exact(runtime, ['template', 'operatorRoot', 'outputRoot', 'dependencies'])
    need(Path(runtime['operatorRoot']) == root / 'runtime-operators' and
         Path(runtime['outputRoot']) == Path(manifest['hostRoot']) / 'runtime-observations', 'fixed fresh runtime namespaces')
    exact(runtime['dependencies'], ['storageWatcher', 'toolsClosure', 'browser', 'playwrightEntry', 'probe', 'inputObservations'])
    checked(runtime['dependencies']['storageWatcher'])
    for key in ['toolsClosure', 'inputObservations', 'browser', 'playwrightEntry']:
        exact(runtime['dependencies'][key], ['path'])
        pin(runtime['dependencies'][key]['path'])
    probe = runtime['dependencies']['probe']
    exact(probe, ['directory', 'imageMetadata', 'source', 'configBlob'])
    checked(probe['source'])
    for key in ['imageMetadata', 'configBlob']:
        exact(probe[key], ['path'])
        ordinary(probe[key]['path'])
    need(Path(probe['imageMetadata']['path']) == Path(probe['directory']) / 'IMAGE.json', 'actual probe metadata locator')
    git_source(ctx['source'])
    return ctx, manifest, root
