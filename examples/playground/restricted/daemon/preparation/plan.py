"""Closed private-operator archive plans; these bytes never constitute approval."""
import json
import re

MAX_PLAN = 4 * 1024 * 1024
ENV = ['PATH=/materials/runtime/node/bin', 'LC_ALL=C.UTF-8', 'LANG=C.UTF-8',
       'TMPDIR=/tmp', 'HOME=/nonexistent', 'TZ=UTC', 'SOURCE_DATE_EPOCH=0']
CONFIG = {'User': '65534:65534', 'Entrypoint': ['/supervisor'], 'Cmd': ['idle'],
          'WorkingDir': '/tmp', 'Env': ENV}
MOUNTS = {'app', 'resources', 'materials', 'input', 'output', 'tmp', 'dev', 'proc', 'sys'}
BASE = {'supervisor', 'app/compiler', 'resources/materials-manifest.json', 'licenses/zryna-LICENSE',
        'lib64/ld-linux-x86-64.so.2'}
EVALUATION = {'resources/loader.js', 'resources/evaluation-identity.json', 'app/evaluate.mjs',
              *('app/' + name + '.mjs' for name in
                ['worker', 'protocol', 'limits', 'json', 'report', 'executable', 'evaluation']),
              *('app/daemon/' + name + '.mjs' for name in
                ['evaluate', 'evaluation-input', 'evaluation-page', 'evaluation-server'])}

def require(value, reason):
    if not value:
        raise ValueError(reason)

def exact(value, keys):
    require(type(value) is dict and set(value) == set(keys), 'closed record fields required')

def integer(value, maximum, minimum=0):
    require(type(value) is int and minimum <= value <= maximum, 'finite integer required')

def digest(value):
    require(type(value) is str and re.fullmatch('[a-f0-9]{64}', value), 'SHA256 required')

def path(value, absolute=False):
    require(type(value) is str and bool(value), 'nonempty path required')
    require(len(value) <= (4096 if absolute else 240) and
            re.fullmatch(r'/[A-Za-z0-9._@+/-]+' if absolute else r'[A-Za-z0-9._@+/-]+', value),
            'portable ASCII path required')
    parts = value.split('/')[1:] if absolute else value.split('/')
    require(all(part not in ['', '.', '..'] for part in parts) and
            len(parts) <= (64 if absolute else 16), 'canonical bounded path required')

def ordered(values):
    require(type(values) is list and values == sorted(set(values)), 'sorted unique paths required')

def decode(raw):
    require(type(raw) is bytes and 0 < len(raw) <= MAX_PLAN, 'bounded plan bytes required')
    def pairs(items):
        output = {}
        for key, value in items:
            require(key not in output, 'duplicate JSON key')
            output[key] = value
        return output
    return json.loads(raw.decode('utf8', errors='strict'), object_pairs_hook=pairs,
                      parse_constant=lambda _: (_ for _ in ()).throw(ValueError('nonfinite JSON')))

def validate(document):
    exact(document, ['format', 'version', 'kind', 'role', 'directories', 'files', 'bytes', 'config'])
    require(document['format'] == 'zryna.playground-preparation-plan.v1' and
            type(document['version']) is int and document['version'] == 1, 'plan version')
    require(document['kind'] in ['runtime', 'materials'] and
            document['role'] in ['compiler', 'evaluator'], 'closed plan role/kind')
    runtime, compiler = document['kind'] == 'runtime', document['role'] == 'compiler'
    require(document['config'] == (CONFIG if runtime else None), 'exact runtime configuration')
    directories = document['directories']
    require(type(directories) is list and len(directories) <= 512, 'directory count')
    for name in directories:
        path(name)
    ordered(directories)
    files = document['files']
    limit = 512 if runtime or not compiler else 64
    require(type(files) is list and 0 < len(files) <= limit, 'file count')
    names, ancestors, total = [], set(), 0
    for item in files:
        exact(item, ['path', 'sourcePath', 'bytes', 'sha256', 'mode'])
        name = item['path']
        path(name)
        path(item['sourcePath'], absolute=True)
        maximum = 128 * 1024**2 if runtime or compiler else 512 * 1024**2
        integer(item['bytes'], maximum, 1)
        digest(item['sha256'])
        require(type(item['mode']) is int and item['mode'] in [292, 365], 'fixed ordinary file mode')
        if runtime:
            library = bool(re.fullmatch(r'lib/x86_64-linux-gnu/[A-Za-z0-9_.+-]+\.so(?:\.[A-Za-z0-9_.+-]+)*', name))
            copyright = bool(re.fullmatch(r'licenses/runtime/ubuntu(?:22|24)/[a-z0-9+.-]+/copyright', name))
            require(name in BASE or not compiler and name in EVALUATION or library or copyright,
                    'undeclared runtime path')
            executable = name in ['supervisor', 'app/compiler', 'lib64/ld-linux-x86-64.so.2']
        else:
            require(name == 'runtime/node/bin/node' or name.startswith('licenses/') or
                    (name.startswith('lib/zryna/bootstrap/') if compiler else
                     name.startswith(('browser/chrome-linux64/', 'playwright/'))),
                    'undeclared material path')
            executable = name in ['runtime/node/bin/node', 'browser/chrome-linux64/chrome',
                                   'browser/chrome-linux64/chrome_crashpad_handler',
                                   'browser/chrome-linux64/chrome_sandbox']
        require(item['mode'] == (365 if executable else 292), 'path-specific mode')
        names.append(name)
        parts = name.split('/')
        ancestors.update('/'.join(parts[:index]) for index in range(1, len(parts)))
        total += item['bytes']
    ordered(names)
    required = BASE | (set() if compiler else EVALUATION) if runtime else {'runtime/node/bin/node'}
    require(required <= set(names), 'complete required inventory')
    if not runtime and not compiler:
        require({'browser/chrome-linux64/chrome', 'playwright/index.mjs', 'playwright/package.json'} <= set(names),
                'complete evaluator material roots')
    require(not set(names) & set(directories), 'file/directory collision')
    require(set(directories) == ancestors | (MOUNTS if runtime else set()), 'exact directory closure')
    maximum = 128 * 1024**2 if runtime else 192 * 1024**2 if compiler else 1024**3
    integer(document['bytes'], maximum, 1)
    require(total == document['bytes'], 'aggregate exact bytes')
    return document
