"""Explicit operator preparation only: import fixed roots and populate never-started volumes.

Returned records are unapproved observations. They never admit a guest or authenticate a
publisher. The caller must retain failure ownership and independently review final pins.
"""
import json
import re
import secrets
from urllib.parse import urlencode
from emit import run as emit_plan
from engine import decode
from handles import Sources
from plan import MAX_PLAN, CONFIG, decode as decode_plan, digest, integer, path, require, validate
from material_seed import holder as initialized_holder, inspect_empty


def selected_plan(plan_path, plan_bytes, plan_sha256, kind):
    path(plan_path, absolute=True)
    integer(plan_bytes, MAX_PLAN, 1)
    digest(plan_sha256)
    sources = Sources()
    try:
        raw = sources.read_plan(plan_path, plan_bytes, plan_sha256)
        document = validate(decode_plan(raw))
        require(document['kind'] == kind, 'independently selected preparation kind')
        for item in document['files']:
            sources.verify(item)
        sources.revalidate_files()
        return document
    finally:
        sources.close()


def transport_limit(document):
    # All USTAR records, individual padding, terminal blocks and stream record padding.
    return document['bytes'] + (len(document['files']) + len(document['directories']) + 1) * 1024 + 20480


def validate_runtime_image(state, image):
    require(state['Id'] == image and state['Os'] == 'linux' and state['Architecture'] == 'amd64'
            and state.get('Variant', '') == '' and state.get('OsVersion', '') == '',
            'imported immutable image target differs')
    config = state['Config']
    require(type(config) is dict and all(config.get(key) == value for key, value in CONFIG.items()),
            'imported fixed runtime configuration differs')
    for key, value in config.items():
        if key in CONFIG:
            continue
        if key in ['Hostname', 'Domainname', 'MacAddress', 'StopSignal']:
            permitted = value == ''
        elif key in ['AttachStdin', 'AttachStdout', 'AttachStderr', 'Tty', 'OpenStdin',
                     'StdinOnce', 'ArgsEscaped', 'NetworkDisabled']:
            permitted = value is False
        elif key in ['Volumes', 'ExposedPorts', 'Healthcheck', 'Labels']:
            permitted = value is None or type(value) is dict and not value
        elif key in ['OnBuild', 'Shell']:
            permitted = value is None or type(value) is list and not value
        elif key == 'StopTimeout':
            permitted = value is None
        elif key == 'Image':
            permitted = type(value) is str and (value == '' or re.fullmatch('sha256:[a-f0-9]{64}', value))
        else:
            permitted = False
        require(permitted, 'inherited or unknown runtime image authority refused')


def import_runtime(engine, plan_path, plan_bytes, plan_sha256):
    document = selected_plan(plan_path, plan_bytes, plan_sha256, 'runtime')
    tag = 'zryna410-candidate/' + document['role'] + ':' + secrets.token_hex(16)
    repository, suffix = tag.rsplit(':', 1)
    changes = ['USER 65534:65534', 'ENTRYPOINT ["/supervisor"]', 'CMD ["idle"]', 'WORKDIR /tmp']
    changes.extend('ENV ' + entry for entry in CONFIG['Env'])
    query = urlencode([('fromSrc', '-'), ('repo', repository), ('tag', suffix),
                       ('platform', 'linux/amd64'), *(('changes', change) for change in changes)])
    record = {'status': 'unapproved-operator-preparation', 'kind': 'runtime', 'role': document['role'],
              'tag': tag, 'planBytes': plan_bytes, 'planSha256': plan_sha256,
              'image': None, 'emission': None, 'failure': None}
    try:
        raw, emission = engine.request('POST', '/images/create?' + query,
            emitter=lambda output: emit_plan(plan_path, plan_bytes, plan_sha256, output),
            stream_maximum=transport_limit(document), statuses=(200,))
        record['emission'] = emission
        events = [decode(line) for line in raw.splitlines() if line]
        require(0 < len(events) <= 512 and all(type(event) is dict for event in events), 'finite import events')
        require(all('error' not in event and 'errorDetail' not in event for event in events), 'image import failed')
        ids = [event.get('status') for event in events if re.fullmatch(r'sha256:[a-f0-9]{64}', str(event.get('status')))]
        require(len(ids) == 1, 'one actual imported image identity required')
        record['image'] = ids[0]
        state = engine.json('GET', '/images/' + ids[0] + '/json')
        validate_runtime_image(state, ids[0])
        require(state['RootFS'] == {'Type': 'layers', 'Layers': ['sha256:' + emission['archiveSha256']]},
                'imported complete root differs from emitted stream')
        record['rootFs'] = state['RootFS']
        record['config'] = state['Config']
        record['sourceInventoryBeforeAfterEqual'] = True
    except BaseException as error:
        record['failure'] = type(error).__name__ + ': ' + str(error)[:1024]
        # Import may have completed before a failed transport; retain the unique tag for recovery.
    return record


def holder_config(image, name, nonce):
    require(re.fullmatch(r'sha256:[a-f0-9]{64}', image), 'independently approved runtime image required')
    return {'Image': image, 'User': '65534:65534', 'Entrypoint': ['/supervisor'], 'Cmd': ['idle'],
            'WorkingDir': '/tmp', 'Env': CONFIG['Env'], 'Labels': {'zryna410.provisioner': nonce},
            'HostConfig': {'ReadonlyRootfs': True, 'NetworkMode': 'none', 'CapDrop': ['ALL'],
                'Privileged': False, 'Init': False, 'IpcMode': 'none', 'CgroupnsMode': 'private',
                'Memory': 16777216, 'MemorySwap': 16777216, 'PidsLimit': 2,
                'CpuPeriod': 100000, 'CpuQuota': 10000, 'RestartPolicy': {'Name': 'no'},
                'LogConfig': {'Type': 'none', 'Config': {}}, 'AutoRemove': False,
                'SecurityOpt': ['no-new-privileges:true'],
                'Mounts': [{'Type': 'volume', 'Source': name, 'Target': '/materials',
                            'ReadOnly': False, 'VolumeOptions': {'NoCopy': False}}]}}


def volume_identity(value, name, nonce, root, created_at=None):
    require(value['Name'] == name and value['Driver'] == 'local' and value['Scope'] == 'local'
            and value['Options'] in [None, {}]
            and value['Labels'] == {'zryna410.materials-nonce': nonce, 'zryna410.materials-root': root}
            and type(value['CreatedAt']) is str and 20 <= len(value['CreatedAt']) <= 64,
            'created volume provenance differs')
    require(created_at is None or value['CreatedAt'] == created_at, 'volume creation identity changed')


def populate_materials(engine, image, plan_path, plan_bytes, plan_sha256, root_sha256):
    document = selected_plan(plan_path, plan_bytes, plan_sha256, 'materials')
    digest(root_sha256)
    from compose import material_manifest
    import hashlib
    require(hashlib.sha256(material_manifest(document)).hexdigest() == root_sha256,
            'independently selected material root differs')
    nonce = secrets.token_hex(16)
    name = 'zryna410-materials-' + document['role'] + '-' + nonce
    holder_name = 'zryna410-provisioner-' + nonce
    record = {'status': 'unapproved-operator-preparation', 'kind': 'materials', 'role': document['role'],
              'name': name, 'nonce': nonce, 'root_sha256': root_sha256, 'files': len(document['files']),
              'bytes': document['bytes'], 'created_at': None, 'holderName': holder_name,
              'holderId': None, 'holderRemoved': False, 'emission': None, 'failure': None,
              'initialEmptyRoot': None,
              'initialization': 'daemon copy of approved empty image directory metadata only; never started'}
    try:
        validate_runtime_image(engine.json('GET', '/images/' + image + '/json'), image)
        value = engine.json('POST', '/volumes/create', {'Name': name, 'Driver': 'local',
            'Labels': {'zryna410.materials-nonce': nonce, 'zryna410.materials-root': root_sha256}})
        volume_identity(value, name, nonce, root_sha256)
        record['created_at'] = value['CreatedAt']
        holder = engine.json('POST', '/containers/create?' + urlencode({'name': holder_name}),
                             holder_config(image, name, nonce))
        require(re.fullmatch('[a-f0-9]{64}', str(holder.get('Id'))), 'actual provisioner identity required')
        record['holderId'] = holder['Id']
        state = engine.json('GET', '/containers/' + holder['Id'] + '/json')
        initialized_holder(state, image, name, nonce, holder_name)
        record['initialEmptyRoot'] = inspect_empty(engine, holder['Id'])
        initialized_holder(engine.json('GET', '/containers/' + holder['Id'] + '/json'),
                           image, name, nonce, holder_name)
        _, emission = engine.request('PUT', '/containers/' + holder['Id'] + '/archive?' +
            urlencode({'path': '/materials', 'noOverwriteDirNonDir': '1'}),
            emitter=lambda output: emit_plan(plan_path, plan_bytes, plan_sha256, output),
            stream_maximum=transport_limit(document), statuses=(200,))
        record['emission'] = emission
        state = engine.json('GET', '/containers/' + holder['Id'] + '/json')
        initialized_holder(state, image, name, nonce, holder_name)
        engine.request('DELETE', '/containers/' + holder['Id'] + '?force=0&v=0', statuses=(204,))
        raw, _ = engine.request('GET', '/containers/' + holder['Id'] + '/json', statuses=(404,))
        require(type(decode(raw)) is dict, 'actual removed provisioner observation required')
        record['holderRemoved'] = True
        volume_identity(engine.json('GET', '/volumes/' + name), name, nonce, root_sha256, record['created_at'])
    except BaseException as error:
        record['failure'] = type(error).__name__ + ': ' + str(error)[:1024]
        # On uncertainty retain exact names/IDs. Never approve, force-remove, or delete a volume here.
    return record
