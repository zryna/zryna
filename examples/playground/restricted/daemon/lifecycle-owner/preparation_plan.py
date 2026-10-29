"""Closed lifecycle transport selection; declarations do not authenticate vendors or runtimes."""
import hashlib
import json
import re
from preparation_primitives import load

old = load('plan')
require, exact, integer, digest = old.require, old.exact, old.integer, old.digest
ENV = ['LANG=C.UTF-8', 'LC_ALL=C.UTF-8', 'TMPDIR=/tmp', 'ZRYNA_PLAYGROUND_TEST_MATERIALS=/materials']
CONFIG = {'User': '65534:65534', 'Entrypoint': ['/lifecycle/supervisor'], 'Cmd': ['idle'],
          'WorkingDir': '/tmp', 'Env': ENV}
MOUNTS = {'materials', 'input', 'output', 'tmp', 'dev', 'dev/shm', 'proc', 'sys'}
# CPython 3.12 locates exec_prefix using this directory even when the selected
# os/resource/sys bootstrap needs no dynamically loaded extension files.
PYTHON_LANDMARK_DIRECTORIES = {'usr/lib/python3.12/lib-dynload'}
NODE = '/materials/runtime/node/bin/node'
PROVIDERS = [
    ('limits-v3.mjs', 123, 'c3e5d4adea4e08d4f9475269cee3f4e5f870ba052a6ea76ff1e6cb5fb23f6913'),
    ('limits-v4.mjs', 719, '7c6417f75bee60072b65eb5fe7847254835dde4bb07d6500e53c797a7cfba29b'),
    ('node_modules/@typescript/old/lib/typescript.js', 9144216, '569177652966bd528c319171c7dd22860dbf72bde116cbc4f644f1d02bb12e39'),
    ('node_modules/@typescript/old/package.json', 3527, '9332e97c30d3e53ed54910b89207ed657fb444066484df6e5b6965bf130865e9'),
    ('node_modules/@typescript/typescript6/lib/typescript.js', 45, 'd3f3cd2b04b7f466f4484df921b744223f7bd1f3e353ec9110bdf52695b983d5'),
    ('node_modules/@typescript/typescript6/package.json', 841, 'd9b8fb53c67c947fece83bcb73fa8512227ea6a12b110e096fdab8ecdc02b655'),
    ('worker-v3.mjs', 39227, '665ab7eaebae5110377b9832e746859ed4b351c26548c489970ba1d08cd04552'),
    ('worker-v4.mjs', 73118, '80ceea20ee7953a1eb8f85a4724075a6ec801983ceb16ee83752313bb3740a8a'),
    ('worker.mjs', 32112, '404f253a90e070c89fb9a2078e7d0c8144549d34559356cc80d5587e3679e868')]
MATERIALS = [{'path': '/materials/lib/zryna/bootstrap/' + name, 'bytes': size,
              'sha256': sha, 'mode': 0o444} for name, size, sha in PROVIDERS] + [
    {'path': NODE, 'bytes': 124674920,
     'sha256': '243fd8938011479f41b3de101842150fa990f33fbbb3f7aabd330857f2d79e1d', 'mode': 0o555}]
FLAGS = {'sourceAuthenticated': False, 'hostAuthenticated': False, 'runtimeApproved': False,
         'materialsApproved': False, 'ownerExecuted': False, 'executionApproved': False,
         'proofQualified': False, 'performanceQualified': False}


def canonical(value):
    return (json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(',', ':')) + '\n').encode()


def path(name):
    old.path(name, absolute=True)
    require(len(name) <= 240 and len(name.split('/')) <= 17, 'existing lifecycle path bounds')


def document(files, mounts=()):
    ancestors = set(mounts)
    for item in files:
        parts = item['path'].split('/')
        ancestors.update('/'.join(parts[:index]) for index in range(1, len(parts)))
    require(len(ancestors) <= 512 and not ancestors.intersection(item['path'] for item in files),
            'directory bound/file collision')
    return {'files': files, 'directories': sorted(ancestors), 'bytes': sum(item['bytes'] for item in files)}


def source_rows(rows, expected, provenance):
    require(type(rows) is list and len(rows) == len(expected), 'complete explicit source selection')
    require([row['path'] for row in rows] == sorted(expected), 'ordered unique source rows')
    result = []
    for row in rows:
        exact(row, ['path', 'sourcePath', 'provenance'])
        require(row['provenance'] in provenance, 'missing independently selected provenance input')
        path(row['sourcePath'])
        file = expected[row['path']]
        result.append({key: file[key] for key in ['bytes', 'sha256', 'mode']} |
                      {'path': row['path'].lstrip('/'), 'sourcePath': row['sourcePath']})
    return result


def runtime_inventory(runtime, plan):
    exact(runtime, ['files', 'elf', 'python', 'support'])
    require(hashlib.sha256(canonical(runtime)).hexdigest() == plan['runtime']['sha256'], 'selected runtime hash')
    files = runtime['files']
    require(type(files) is list and 0 < len(files) <= 512, 'runtime file bound')
    names = [file['path'] for file in files]
    require(names == sorted(set(names)) and len(set(name.lower() for name in names)) == len(names), 'case-unique inventory')
    by_name = {}
    for file in files:
        exact(file, ['path', 'bytes', 'sha256', 'mode', 'kind', 'nlink', 'uid', 'gid'])
        path(file['path']); integer(file['bytes'], 128 * 1024**2, 1); digest(file['sha256'])
        require(file['kind'] == 'regular' and type(file['nlink']) is int and file['nlink'] == 1
                and type(file['uid']) is int and type(file['gid']) is int and file['uid'] == file['gid'] == 0
                and type(file['mode']) is int and file['mode'] in [0o444, 0o555], 'sealed ordinary runtime')
        require(not any(file['path'] == '/' + root or file['path'].startswith('/' + root + '/')
                        for root in ['materials', 'input', 'output', 'tmp', 'dev', 'proc', 'sys']), 'mount shadow')
        by_name[file['path']] = file
    require(sum(file['bytes'] for file in files) == plan['runtime']['bytes'] <= 128 * 1024**2
            and plan['runtime']['files'] == len(files), 'image byte/count limit')
    roots = ['/lifecycle/test', plan['prebuild']['artifacts']['compiler']['path'],
             '/lifecycle/supervisor', '/usr/bin/python3']
    for name in roots:
        require(name in by_name and by_name[name]['mode'] == 0o555, 'fixed executable root')
    for role, name in [('test', roots[0]), ('compiler', roots[1])]:
        require(all(by_name[name][key] == plan['prebuild']['artifacts'][role][key]
                    for key in ['bytes', 'sha256']), 'exact unchanged captured artifact')
    exact(runtime['python'], ['entry', 'imports', 'files'])
    py = runtime['python']['files']
    require(runtime['python']['entry'] == '/usr/bin/python3' and runtime['python']['imports'] == ['os', 'resource', 'sys']
            and type(py) is list and len(py) <= 512 and py == sorted(set(py))
            and '/usr/bin/python3' in py and all(name in by_name for name in py), 'explicit Python import inventory')
    support = runtime['support']
    require(type(support) is list and len(support) <= 512 and support == sorted(set(support)), 'inert support inventory')
    require(all(name in by_name and by_name[name]['mode'] == 0o444 and
                name.startswith(('/resources/', '/licenses/')) for name in support), 'inert support role')
    rows = runtime['elf']
    require(type(rows) is list and len(rows) <= 512 and [row['path'] for row in rows] == sorted(set(row['path'] for row in rows)), 'ELF row bound')
    elf = {}
    for row in rows:
        exact(row, ['path', 'prefixHex', 'interpreter', 'needed'])
        require(row['path'] in by_name or row['path'] == NODE, 'only pinned Node external root')
        require(type(row['prefixHex']) is str and re.fullmatch('[a-f0-9]{128,256}', row['prefixHex']), 'ELF prefix')
        prefix = bytes.fromhex(row['prefixHex'])
        require(prefix[:6] == b'\x7fELF\x02\x01' and int.from_bytes(prefix[18:20], 'little') == 62, 'ELF x64')
        require(row['interpreter'] is None or row['interpreter'] in by_name, 'declared image interpreter')
        edges = row['needed']
        require(type(edges) is list and len(edges) <= 128 and [edge['name'] for edge in edges] == sorted(set(edge['name'] for edge in edges)), 'needed bound/order')
        for edge in edges:
            exact(edge, ['name', 'path'])
            require(re.fullmatch('[A-Za-z0-9_.+-]+', edge['name']) and edge['path'] in by_name, 'image-only library edge')
        elf[row['path']] = row
    pending, visited = roots + [NODE] + [name for name in py if name in elf], set()
    while pending:
        name = pending.pop()
        if name in visited:
            continue
        require(name in elf, 'missing declared executable/dependency ELF')
        visited.add(name)
        pending.extend(edge['path'] for edge in elf[name]['needed'])
        if elf[name]['interpreter'] is not None:
            pending.append(elf[name]['interpreter'])
    require(visited == set(elf) and ((visited - {NODE}) | set(py) | set(support)) == set(by_name), 'no disconnected/unassigned files')
    return by_name


def validate(selection):
    exact(selection, ['plan', 'planSha256', 'runtime', 'runtimeSources', 'materialSources', 'provenance'])
    plan = selection['plan']; digest(selection['planSha256'])
    require(hashlib.sha256(canonical(plan)).hexdigest() == selection['planSha256'], 'externally selected planner hash')
    require(plan['format'] == 'zryna.linux-lifecycle-owner-plan.v1' and plan['environment'] == dict(entry.split('=', 1) for entry in ENV), 'original lifecycle role/environment')
    for key in ['sourceAuthenticated', 'ownerExecuted', 'materialsApproved', 'runtimeApproved', 'hostApproved', 'executionApproved', 'proofQualified', 'performanceQualified']:
        require(plan[key] is False, 'planner cannot issue approval')
    require(plan['materials']['files'] == MATERIALS and plan['materials']['bytes'] == 133968848, 'exact original ten materials')
    provenance = selection['provenance']
    require(type(provenance) is dict and 0 < len(provenance) <= 512, 'missing bounded provenance inputs')
    for name, value in provenance.items():
        require(type(name) is str and re.fullmatch('[a-z0-9-]{1,64}', name), 'provenance key')
        exact(value, ['path', 'bytes', 'sha256']); path(value['path'])
        integer(value['bytes'], 4 * 1024**2, 1); digest(value['sha256'])
    runtime = runtime_inventory(selection['runtime'], plan)
    image = document(source_rows(selection['runtimeSources'], runtime, provenance), MOUNTS | PYTHON_LANDMARK_DIRECTORIES)
    materials = document(source_rows(selection['materialSources'], {row['path']: row for row in MATERIALS}, provenance))
    for row in materials['files']:
        row['path'] = row['path'].removeprefix('materials/')
    materials = document(materials['files'])
    manifest = json.dumps({'format': 'zryna.playground-materials.v1', 'version': 1, 'role': 'compiler',
        'directories': materials['directories'], 'files': [{key: row[key] for key in ['path', 'bytes', 'sha256', 'mode']}
        for row in materials['files']]}, separators=(',', ':'), ensure_ascii=True).encode('ascii')
    item = runtime.get('/resources/materials-manifest.json')
    require(item and item['bytes'] == len(manifest) and item['sha256'] == hashlib.sha256(manifest).hexdigest(), 'image-pinned exact material manifest')
    return image, materials, manifest
