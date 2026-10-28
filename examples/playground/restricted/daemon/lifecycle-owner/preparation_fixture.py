"""Nonexecutable synthetic declarations for transport tests; no forged auth capability."""
import copy
import hashlib
import json
from preparation_plan import ENV, MATERIALS, canonical, document


def selection():
    files = []
    roots = ['/lifecycle/supervisor', '/lifecycle/test', '/usr/bin/python3', '/build/release/zryna-playground-compiler']
    for name in sorted(roots + ['/resources/materials-manifest.json']):
        files.append({'path': name, 'bytes': 64, 'sha256': '1' * 64, 'mode': 0o444 if name.startswith('/resources/') else 0o555,
                      'kind': 'regular', 'nlink': 1, 'uid': 0, 'gid': 0})
    materials = document([{key: row[key] for key in ['bytes', 'sha256', 'mode']} |
                          {'path': row['path'][len('/materials/'):]} for row in sorted(MATERIALS, key=lambda row: row['path'])])
    materials['files'] = [{key: row[key] for key in ['path', 'bytes', 'sha256', 'mode']} for row in materials['files']]
    manifest = json.dumps({'format': 'zryna.playground-materials.v1', 'version': 1, 'role': 'compiler',
        'directories': materials['directories'], 'files': materials['files']}, separators=(',', ':')).encode()
    for file in files:
        if file['path'] == '/resources/materials-manifest.json':
            file.update(bytes=len(manifest), sha256=hashlib.sha256(manifest).hexdigest())
    prefix = bytearray(64); prefix[:6] = b'\x7fELF\x02\x01'; prefix[18] = 62
    runtime = {'files': files, 'elf': [{'path': name, 'prefixHex': prefix.hex(), 'interpreter': None, 'needed': []}
        for name in sorted(roots + ['/materials/runtime/node/bin/node'])],
        'python': {'entry': '/usr/bin/python3', 'imports': ['os', 'resource', 'sys'], 'files': ['/usr/bin/python3']},
        'support': ['/resources/materials-manifest.json']}
    by_name = {row['path']: row for row in files}
    artifact = lambda name: {key: by_name[name][key] for key in ['path', 'bytes', 'sha256']}
    plan = {'format': 'zryna.linux-lifecycle-owner-plan.v1', 'environment': dict(entry.split('=', 1) for entry in ENV),
        'runtime': {'sha256': hashlib.sha256(canonical(runtime)).hexdigest(), 'bytes': sum(row['bytes'] for row in files), 'files': len(files)},
        'materials': {'files': copy.deepcopy(MATERIALS), 'bytes': 133968848},
        'prebuild': {'artifacts': {'compiler': artifact('/build/release/zryna-playground-compiler'), 'test': artifact('/lifecycle/test')}}}
    for key in ['sourceAuthenticated', 'ownerExecuted', 'materialsApproved', 'runtimeApproved', 'hostApproved',
                'executionApproved', 'proofQualified', 'performanceQualified']:
        plan[key] = False
    rows = lambda values: [{'path': row['path'], 'sourcePath': '/fixture/' + str(index), 'provenance': 'fixture'}
                           for index, row in enumerate(sorted(values, key=lambda row: row['path']))]
    return {'plan': plan, 'planSha256': hashlib.sha256(canonical(plan)).hexdigest(), 'runtime': runtime,
            'runtimeSources': rows(files), 'materialSources': rows(MATERIALS),
            'provenance': {'fixture': {'path': '/fixture/provenance', 'bytes': 1, 'sha256': '2' * 64}}}


def repin(value):
    runtime, plan = value['runtime'], value['plan']
    plan['runtime'].update(sha256=hashlib.sha256(canonical(runtime)).hexdigest(),
        bytes=sum(row['bytes'] for row in runtime['files']), files=len(runtime['files']))
    value['planSha256'] = hashlib.sha256(canonical(plan)).hexdigest()
    return value
