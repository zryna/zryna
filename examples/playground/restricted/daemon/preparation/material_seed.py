"""Supported daemon-only empty-directory metadata initialization, never a process start."""
import base64
import hashlib
import tarfile
from urllib.parse import urlencode
from plan import CONFIG, require


def holder(state, image, name, nonce, container_name):
    require(state['Name'] == '/' + container_name and state['Image'] == image and
            state['Config']['Labels'] == {'zryna410.provisioner': nonce}, 'owned metadata-copy holder')
    config, host, process = state['Config'], state['HostConfig'], state['State']
    require(all(config.get(key) == value for key, value in CONFIG.items()), 'fixed nonroot holder process')
    require(process['Status'] == 'created' and process['Running'] is False and process['Pid'] == 0
            and process.get('StartedAt') == '0001-01-01T00:00:00Z'
            and process.get('FinishedAt') == '0001-01-01T00:00:00Z'
            and state.get('RestartCount') == 0, 'never-started owned provisioner must never execute')
    expected = {'ReadonlyRootfs': True, 'NetworkMode': 'none', 'CapDrop': ['ALL'],
                'Privileged': False, 'Init': False, 'IpcMode': 'none', 'CgroupnsMode': 'private',
                'Memory': 16777216, 'MemorySwap': 16777216, 'PidsLimit': 2,
                'CpuPeriod': 100000, 'CpuQuota': 10000, 'AutoRemove': False,
                'SecurityOpt': ['no-new-privileges:true'], 'LogConfig': {'Type': 'none', 'Config': {}}}
    require(all(host.get(key) == value for key, value in expected.items()), 'fixed metadata-copy holder limits')
    require(host['RestartPolicy'].get('Name') == 'no' and
            host['RestartPolicy'].get('MaximumRetryCount', 0) == 0, 'no holder restart')
    for key in ['Binds', 'VolumesFrom', 'CapAdd', 'Devices', 'DeviceRequests', 'GroupAdd',
                'Links', 'DeviceCgroupRules', 'ExtraHosts']:
        require(host.get(key) in [None, []], 'extra metadata-copy holder authority')
    raw = host.get('Mounts')
    require(type(raw) is list and len(raw) == 1 and type(raw[0]) is dict, 'one initialization mount')
    mount = raw[0]
    require(set(mount) <= {'Type', 'Source', 'Target', 'ReadOnly', 'VolumeOptions'}
            and mount.get('Type') == 'volume' and mount.get('Source') == name
            and mount.get('Target') == '/materials' and mount.get('ReadOnly', False) is False,
            'exact preparation-only writable material mount')
    options = mount.get('VolumeOptions')
    require(type(options) is dict and set(options) <= {'NoCopy'} and options.get('NoCopy', False) is False,
            'preparation-only empty metadata copy required')
    mounts = state['Mounts']
    require(type(mounts) is list and len(mounts) == 1 and mounts[0]['Type'] == 'volume'
            and mounts[0]['Name'] == name and mounts[0]['Destination'] == '/materials'
            and mounts[0]['Driver'] == 'local' and mounts[0]['RW'] is True,
            'sole explicitly writable provisioning mount required')


def empty_root(raw):
    # Exactly one USTAR-compatible header plus its two terminal zero blocks. No PAX,
    # hidden members, data blocks, partial EOF or trailing nonzero bytes can qualify.
    require(type(raw) is bytes and len(raw) == 1536 and raw[512:] == bytes(1024),
            'complete single empty directory archive required')
    member = tarfile.TarInfo.frombuf(raw[:512], 'ascii', 'strict')
    require(member.name == 'materials' and member.type == tarfile.DIRTYPE and member.linkname == ''
            and member.uid == 0 and member.gid == 0 and member.mode == 365 and member.size == 0,
            'initialized empty root must be UID/GID0 mode0555')
    return {'name': member.name, 'uid': member.uid, 'gid': member.gid, 'mode': member.mode,
            'bytes': 0, 'members': 1, 'tarBytes': len(raw), 'tarSha256': hashlib.sha256(raw).hexdigest(),
            'rawTarBase64': base64.b64encode(raw).decode('ascii'), 'actualEmptyRoot0555': True}


def inspect_empty(engine, cid):
    raw, _ = engine.request('GET', '/containers/' + cid + '/archive?' +
                            urlencode({'path': '/materials'}), statuses=(200,), maximum_response=1536)
    return empty_root(raw)
