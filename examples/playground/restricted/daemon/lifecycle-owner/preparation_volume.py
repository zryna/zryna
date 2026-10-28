"""Fresh volume preparation with a never-started, subsequently removed metadata holder."""
import hashlib
import os
import stat
import re
import secrets
from urllib.parse import urlencode
from preparation_primitives import emit, empty_root, load, transport_limit
from preparation_plan import CONFIG, MATERIALS, path as canonical_path, require


def volume(state, name, nonce, root, mountpoint, created=None):
    require(state['Name'] == name and state['Driver'] == 'local' and state['Scope'] == 'local'
            and state['Options'] in [None, {}]
            and state['Labels'] == {'zryna410.materials-nonce': nonce, 'zryna410.materials-root': root}
            and state['Mountpoint'] == mountpoint
            and type(state['CreatedAt']) is str and 20 <= len(state['CreatedAt']) <= 64,
            'exact fresh material volume identity')
    require(created is None or created == state['CreatedAt'], 'volume creation identity changed')


def holder_config(image, name, nonce):
    return dict(CONFIG) | {'Image': image, 'Labels': {'zryna410.provisioner': nonce},
        'HostConfig': {'ReadonlyRootfs': True, 'NetworkMode': 'none', 'CapDrop': ['ALL'],
            'Privileged': False, 'Init': False, 'IpcMode': 'none', 'CgroupnsMode': 'private',
            'Memory': 16777216, 'MemorySwap': 16777216, 'PidsLimit': 2,
            'CpuPeriod': 100000, 'CpuQuota': 10000, 'RestartPolicy': {'Name': 'no'},
            'LogConfig': {'Type': 'none', 'Config': {}}, 'AutoRemove': False,
            'SecurityOpt': ['no-new-privileges:true'], 'Mounts': [{'Type': 'volume', 'Source': name,
                'Target': '/materials', 'ReadOnly': False, 'VolumeOptions': {'NoCopy': False}}]}}


def holder(state, image, name, nonce, container_name):
    # Reuse the unchanged full never-started checker, with only the private process config
    # projected to its old expected values after checking every actual lifecycle field.
    require(all(state['Config'].get(key) == value for key, value in CONFIG.items()), 'lifecycle holder configuration')
    import copy
    projected = copy.deepcopy(state)
    projected['Config'].update(load('plan').CONFIG)
    load('material_seed').holder(projected, image, name, nonce, container_name)


def no_writers(engine, name, mountpoint, record=None):
    """Complete listed Docker writers, including backing-path/inode aliases, not all host writers."""
    record = {} if record is None else record
    require(os.path.realpath(mountpoint, strict=True) == mountpoint, 'canonical selected material backing path')
    paths = {mountpoint}
    for file in MATERIALS:
        current = mountpoint + file['path'][len('/materials'):]
        while current != '/':
            paths.add(current); current = os.path.dirname(current)
    paths.add('/')
    identities = set(); record['materialMetadata'] = []
    for source in sorted(paths):
        value = os.lstat(source)
        require(not stat.S_ISLNK(value.st_mode) and (stat.S_ISREG(value.st_mode) or stat.S_ISDIR(value.st_mode)), 'ordinary backing tree')
        identities.add((value.st_dev, value.st_ino))
        record['materialMetadata'].append({'path': source, 'dev': value.st_dev, 'ino': value.st_ino, 'mode': value.st_mode})
    raw, _ = engine.request('GET', '/containers/json?all=1', statuses=(200,))
    states = load('engine').decode(raw); record['listedContainers'] = states
    require(type(states) is list and len(states) <= 512, 'bounded complete unfiltered container inventory')
    seen = set(); record['inspectedContainers'] = []
    def overlap(source):
        return source == mountpoint or source.startswith(mountpoint + '/') or mountpoint.startswith(source.rstrip('/') + '/')
    for item in states:
        cid = item.get('Id')
        require(type(cid) is str and re.fullmatch('[a-f0-9]{64}', cid) and cid not in seen, 'complete unique container IDs')
        seen.add(cid)
        state = engine.json('GET', '/containers/' + cid + '/json'); record['inspectedContainers'].append(state)
        require(state.get('Id') == cid and type(state.get('Mounts')) is list and len(state['Mounts']) <= 64, 'bounded actual inspected mounts')
        for mount in state['Mounts']:
            require(type(mount.get('RW')) is bool, 'actual mount write state')
            if mount.get('Type') == 'tmpfs': continue
            source = mount.get('Source')
            require(type(source) is str and source.startswith('/') and len(source) <= 4096
                    and not any(part in ['.', '..'] for part in source.split('/')), 'canonical actual source')
            if not mount['RW']: continue
            canonical_path(source)
            require(mount.get('Type') in ['bind', 'volume'] and mount.get('Name') != name and not overlap(source), 'no named or lexical writer')
            actual = os.path.realpath(source, strict=True)
            require(not overlap(actual), 'no resolved material writer alias')
            value = os.stat(actual)
            require((value.st_dev, value.st_ino) not in identities, 'no backing inode writer alias')
    return sorted(seen)


def populate(engine, image, document, manifest, sources, record):
    root = hashlib.sha256(manifest).hexdigest()
    nonce = secrets.token_hex(16)
    name, container_name = 'zryna410-materials-compiler-' + nonce, 'zryna418-material-seed-' + nonce
    mountpoint = engine.expected['dockerRootDir'].rstrip('/') + '/volumes/' + name + '/_data'
    material = {'name': name, 'nonce': nonce, 'createdAt': None, 'manifestSha256': root,
                'manifestBytes': len(manifest), 'files': 10, 'bytes': 133968848}
    stage = {'selection': material, 'holderName': container_name, 'holderId': None,
             'holderRemoved': False, 'emission': None, 'writerScan': None}
    record['materials'] = stage
    # Both random names must be absent before any create; unknown errors are not absence.
    raw, _ = engine.request('GET', '/volumes/' + name, statuses=(404,))
    require(type(load('engine').decode(raw)) is dict, 'actual volume absence')
    raw, _ = engine.request('GET', '/containers/' + container_name + '/json', statuses=(404,))
    require(type(load('engine').decode(raw)) is dict, 'actual holder absence')
    value = engine.json('POST', '/volumes/create', {'Name': name, 'Driver': 'local',
        'Labels': {'zryna410.materials-nonce': nonce, 'zryna410.materials-root': root}})
    volume(value, name, nonce, root, mountpoint); material['createdAt'] = value['CreatedAt']
    created = engine.json('POST', '/containers/create?' + urlencode({'name': container_name}),
                          holder_config(image['id'], name, nonce))
    cid = created.get('Id'); require(type(cid) is str and re.fullmatch('[a-f0-9]{64}', cid), 'actual holder ID')
    stage['holderId'] = cid
    holder(engine.json('GET', '/containers/' + cid + '/json'), image['id'], name, nonce, container_name)
    raw, _ = engine.request('GET', '/containers/' + cid + '/archive?' + urlencode({'path': '/materials'}),
                             statuses=(200,), maximum_response=1536)
    stage['initialEmptyRoot'] = empty_root(raw)
    holder(engine.json('GET', '/containers/' + cid + '/json'), image['id'], name, nonce, container_name)
    _, stage['emission'] = engine.request('PUT', '/containers/' + cid + '/archive?' +
        urlencode({'path': '/materials', 'noOverwriteDirNonDir': '1'}),
        emitter=lambda stream: emit(document, sources, stream), stream_maximum=transport_limit(document), statuses=(200,))
    holder(engine.json('GET', '/containers/' + cid + '/json'), image['id'], name, nonce, container_name)
    engine.request('DELETE', '/containers/' + cid + '?force=0&v=0', statuses=(204,))
    raw, _ = engine.request('GET', '/containers/' + cid + '/json', statuses=(404,))
    require(type(load('engine').decode(raw)) is dict, 'holder removal readback')
    stage['holderRemoved'] = True
    volume(engine.json('GET', '/volumes/' + name), name, nonce, root, mountpoint, material['createdAt'])
    stage['writerScan'] = no_writers(engine, name, mountpoint)
    # This observation is not launch authority. The owner repeats readonly/NoCopy and writer
    # checks immediately around its own mount and audits actual bytes inside its owned parent.
