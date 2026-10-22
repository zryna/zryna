"""Separate closed360 image plan. Does not widen any production preparation role or mode."""
import re
from cli_common import ENV, digest, exact, integer, require
from cli_engine import decode
from cli_image_pins import RUNTIME

MAX_PLAN = 262144
ARCHIVE_SHA256 = 'b258d45e1eb58a701026ced7c74d03dc74c911c90a5ec7da93c656884a97159f'
ROOT = 'zryna-0.2.3-x86_64-unknown-linux-gnu'
EPOCH = 1789379181
CONFIG = {'User': '65534:65534', 'Entrypoint': ['/supervisor'], 'Cmd': ['idle'],
          'WorkingDir': '/work/project', 'Env': ENV}
MOUNTS = {'dev', 'dev/shm', 'input', 'output', 'proc', 'sys', 'tmp', 'work', 'work/project'}


def path(value, absolute=False):
    require(type(value) is str and 0 < len(value) <= (4096 if absolute else 240), 'bounded path')
    require(re.fullmatch(r'/[A-Za-z0-9._@+/-]+' if absolute else r'[A-Za-z0-9._@+/-]+', value), 'ASCII path')
    parts = value.split('/')[1:] if absolute else value.split('/')
    require(len(parts) <= (64 if absolute else 16) and all(part not in ['', '.', '..'] for part in parts),
            'canonical path without traversal')


def ordered(values):
    require(type(values) is list and values == sorted(set(values)), 'sorted unique inventory')


def validate(value):
    exact(value, ['format', 'version', 'archive', 'files', 'runtimeFiles', 'directories', 'bytes', 'config'])
    require(value['format'] == 'zryna.playground-cli-image-plan.v1' and type(value['version']) is int and
            value['version'] == 1 and value['config'] == CONFIG, 'separate fixed CLI image domain/config')
    archive = value['archive']
    exact(archive, ['sourcePath', 'bytes', 'sha256'])
    path(archive['sourcePath'], absolute=True)
    require(archive['bytes'] == 52949082 and type(archive['bytes']) is int and archive['sha256'] == ARCHIVE_SHA256,
            'exact original signed archive carrier')
    files, runtime = value['files'], value['runtimeFiles']
    require(type(files) is list and len(files) == 349 and type(runtime) is list and len(runtime) == 11,
            'complete349 plus exact11 runtime files')
    names, ancestors, total = [], set(), 0
    for file in files:
        exact(file, ['path', 'archivePath', 'bytes', 'sha256', 'mode'])
        path(file['path']); path(file['archivePath'])
        require(file['path'] == 'opt/zryna-release/' + file['archivePath'], 'fixed complete installed release root')
        integer(file['bytes'], 128 * 1024**2, 1); digest(file['sha256'])
        executable = file['archivePath'] in ['bin/zryna', 'runtime/node/bin/node']
        require(type(file['mode']) is int and file['mode'] == (0o755 if executable else 0o644), 'original archive modes')
        names.append(file['path']); total += file['bytes']
    require(total == 154023815, 'complete original release expanded bytes')
    pins = {file['path']: file for file in RUNTIME}
    for file in runtime:
        exact(file, ['path', 'sourcePath', 'bytes', 'sha256', 'mode'])
        path(file['sourcePath'], absolute=True)
        require({k: v for k, v in file.items() if k != 'sourcePath'} == pins.get(file['path']),
                'exact unchanged runtime bytes/modes')
        names.append(file['path']); total += file['bytes']
    ordered([file['path'] for file in files]); ordered([file['path'] for file in runtime]); ordered(sorted(names))
    require(len(set(names)) == 360 and total == value['bytes'] == 160488297 and type(value['bytes']) is int,
            'complete360 image bytes')
    for name in names:
        parts = name.split('/')
        ancestors.update('/'.join(parts[:count]) for count in range(1, len(parts)))
    dirs = value['directories']
    require(type(dirs) is list and len(dirs) <= 512, 'bounded directory closure')
    for name in dirs:
        path(name)
    ordered(dirs)
    require(set(dirs) == ancestors | MOUNTS and not set(names) & set(dirs), 'exact directories and finite mount points')
    return value


def decode_plan(raw):
    require(type(raw) is bytes and 0 < len(raw) <= MAX_PLAN, 'bounded image plan')
    return validate(decode(raw))
