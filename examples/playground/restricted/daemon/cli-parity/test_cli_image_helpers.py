"""Synthetic360 metadata and inert Engine replies, never release bytes or imported image proof."""
import copy
import json
import time
from cli_image_plan import ARCHIVE_SHA256, CONFIG, MOUNTS
from cli_image_pins import RUNTIME


def document():
    files = [{'path': 'opt/zryna-release/bin/zryna', 'archivePath': 'bin/zryna', 'bytes': 17376136,
              'sha256': '0' * 64, 'mode': 0o755},
             {'path': 'opt/zryna-release/runtime/node/bin/node', 'archivePath': 'runtime/node/bin/node',
              'bytes': 124674920, 'sha256': '1' * 64, 'mode': 0o755}]
    remaining = 154023815 - sum(file['bytes'] for file in files)
    for index in range(347):
        size = remaining if index == 346 else 1
        remaining -= size
        files.append({'path': f'opt/zryna-release/fixture/file{index:03}', 'archivePath': f'fixture/file{index:03}',
                      'bytes': size, 'sha256': '2' * 64, 'mode': 0o644})
    runtime = [dict(file, sourcePath='/tmp/inert-material/' + str(index)) for index, file in enumerate(RUNTIME)]
    dirs = set(MOUNTS)
    for file in files + runtime:
        parts = file['path'].split('/')
        dirs.update('/'.join(parts[:count]) for count in range(1, len(parts)))
    return {'format': 'zryna.playground-cli-image-plan.v1', 'version': 1,
            'archive': {'sourcePath': '/tmp/inert-release.tar.gz', 'bytes': 52949082, 'sha256': ARCHIVE_SHA256},
            'files': sorted(files, key=lambda file: file['path']),
            'runtimeFiles': sorted(runtime, key=lambda file: file['path']), 'directories': sorted(dirs),
            'bytes': 160496313, 'config': copy.deepcopy(CONFIG)}


class ImageEngine:
    def __init__(self, wrong_layer=False, collision=False):
        self.engine_uid, self.deadline = 65534, time.monotonic() + 179
        self.calls, self.wrong_layer, self.collision, self.tag = [], wrong_layer, collision, None
    def json(self, method, path, value=None):
        self.calls.append((method, path))
        if path.startswith('/images/json?'):
            return [{'Id': 'existing'}] if self.collision else []
        return {'Id': 'sha256:' + 'a' * 64, 'Os': 'linux', 'Architecture': 'amd64',
                'RootFS': {'Type': 'layers', 'Layers': ['sha256:' + ('f' if self.wrong_layer else 'd') * 64]},
                'Config': copy.deepcopy(CONFIG), 'RepoTags': [self.tag]}
    def request(self, method, path, **kwargs):
        from urllib.parse import parse_qs, urlsplit
        import io
        self.calls.append((method, path))
        args = parse_qs(urlsplit(path).query)
        self.tag = args['repo'][0] + ':' + args['tag'][0]
        emission = kwargs['emitter'](io.BytesIO())
        return (json.dumps({'status': 'sha256:' + 'a' * 64}) + '\n').encode(), emission
