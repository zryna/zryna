"""Separate closed image plan for the pretrusted, finite UI qualification role."""
import copy
import re
from plan import ENV, digest, exact, integer, ordered, path, require

CONFIG = {'User': '65534:65534', 'Entrypoint': ['/ui-supervisor'], 'Cmd': [],
          'WorkingDir': '/tmp', 'Env': ENV}
MOUNTS = {'app', 'resources', 'materials', 'tmp', 'dev', 'proc', 'sys'}
REQUIRED = {'supervisor', 'ui-supervisor', 'app/compiler', 'resources/materials-manifest.json',
            'licenses/zryna-LICENSE', 'etc/fonts/fonts.conf',
            'resources/fonts/NimbusSans-Regular.otf', 'resources/fonts/NimbusMonoPS-Regular.otf',
            'licenses/fonts/fonts-urw-base35-copyright', 'lib64/ld-linux-x86-64.so.2', 'resources/ui-plan.json',
            *('app/' + name + '.mjs' for name in
              ['ui-driver', 'browser-probe', 'browser-intercept', 'browser-diagnostics', 'framing', 'routes', 'wire', 'plan'])}


def validate(document):
    exact(document, ['format', 'version', 'directories', 'files', 'bytes', 'config'])
    require(document['format'] == 'zryna.playground-ui-preparation-plan.v1'
            and type(document['version']) is int and document['version'] == 1, 'closed trusted UI plan')
    require(document['config'] == CONFIG, 'fixed trusted UI image configuration')
    directories = document['directories']
    require(type(directories) is list and len(directories) <= 512, 'finite UI directory inventory')
    for name in directories:
        path(name)
    ordered(directories)
    files = document['files']
    require(type(files) is list and 0 < len(files) <= 512, 'finite UI file inventory')
    names, ancestors, total = [], set(), 0
    for item in files:
        exact(item, ['path', 'sourcePath', 'bytes', 'sha256', 'mode'])
        name = item['path']
        path(name)
        path(item['sourcePath'], absolute=True)
        integer(item['bytes'], 128 * 1024**2, 1)
        digest(item['sha256'])
        library = bool(re.fullmatch(r'lib/x86_64-linux-gnu/[A-Za-z0-9_.+-]+\.so(?:\.[A-Za-z0-9_.+-]+)*', name))
        copyright = bool(re.fullmatch(r'licenses/runtime/ubuntu(?:22|24)/[a-z0-9+.-]+/copyright', name))
        require(name in REQUIRED or library or copyright, 'undeclared trusted UI runtime path')
        executable = name in ['supervisor', 'ui-supervisor', 'app/compiler', 'lib64/ld-linux-x86-64.so.2']
        require(type(item['mode']) is int and item['mode'] == (365 if executable else 292),
                'fixed trusted UI path mode')
        names.append(name)
        parts = name.split('/')
        ancestors.update('/'.join(parts[:index]) for index in range(1, len(parts)))
        total += item['bytes']
    ordered(names)
    require(REQUIRED <= set(names) and not set(names) & set(directories), 'complete trusted UI root')
    require(set(directories) == ancestors | MOUNTS, 'exact trusted UI directory closure')
    integer(document['bytes'], 128 * 1024**2, 1)
    require(document['bytes'] == total, 'exact trusted UI aggregate bytes')
    return document


def candidate(files):
    files = sorted(files, key=lambda item: item['path'])
    directories = set(MOUNTS)
    for item in files:
        parts = item['path'].split('/')
        directories.update('/'.join(parts[:index]) for index in range(1, len(parts)))
    return validate({'format': 'zryna.playground-ui-preparation-plan.v1', 'version': 1,
                     'directories': sorted(directories), 'files': files,
                     'bytes': sum(item['bytes'] for item in files), 'config': copy.deepcopy(CONFIG)})
