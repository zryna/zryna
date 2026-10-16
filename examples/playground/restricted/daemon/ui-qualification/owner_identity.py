"""Before/after inspect and material provenance checks for one creation-owned UI object."""
from plan import ENV, require


def image(value, selected):
    require(value['Id'] == selected['image'] and value['Os'] == 'linux' and value['Architecture'] == 'amd64'
            and value.get('Variant', '') == '' and value.get('OsVersion', '') == '', 'UI actual image target')
    require(value['RootFS'] == {'Type': 'layers', 'Layers': ['sha256:' + selected['layer_sha256']]},
            'UI exact image layer')
    expected = {'User': '65534:65534', 'Entrypoint': ['/ui-supervisor'], 'WorkingDir': '/tmp', 'Env': ENV}
    config = value['Config']
    require(type(config) is dict and all(config.get(k) == v for k, v in expected.items())
            and config.get('Cmd') in [None, []], 'UI image configuration')
    for key, field in config.items():
        if key in expected or key == 'Cmd':
            continue
        if key in ['Hostname', 'Domainname', 'MacAddress', 'StopSignal']:
            allowed = field == ''
        elif key in ['AttachStdin', 'AttachStdout', 'AttachStderr', 'Tty', 'OpenStdin', 'StdinOnce',
                     'ArgsEscaped', 'NetworkDisabled']:
            allowed = field is False
        elif key in ['Volumes', 'ExposedPorts', 'Healthcheck', 'Labels']:
            allowed = field is None or type(field) is dict and not field
        elif key in ['OnBuild', 'Shell']:
            allowed = field is None or type(field) is list and not field
        elif key == 'StopTimeout':
            allowed = field is None
        elif key == 'Image':
            allowed = type(field) is str and (not field or field == selected['image'])
        else:
            allowed = False
        require(allowed, 'UI inherited image authority')


def volume(value, pin):
    require(value['Name'] == pin['name'] and value['Driver'] == 'local' and value['Scope'] == 'local'
            and value['Options'] in [None, {}] and value['CreatedAt'] == pin['created_at'] and
            value['Labels'] == {'zryna410.materials-nonce': pin['nonce'],
                                'zryna410.materials-root': pin['root_sha256']}, 'UI exact material volume provenance')


def container(state, cid, name, expected, cleanup=False):
    require(state['Id'] == cid and state['Name'] == '/' + name and state['Image'] == expected['Image'],
            'UI creation-owned container identity')
    config, host = state['Config'], state['HostConfig']
    for key in ['User', 'Entrypoint', 'WorkingDir', 'Env', 'Tty', 'OpenStdin', 'StdinOnce',
                'AttachStdin', 'AttachStdout', 'AttachStderr', 'Labels']:
        require(config.get(key) == expected[key], 'UI fixed process configuration')
    require(config.get('Cmd') in [None, []], 'UI no command arguments')
    config_defaults = {'Domainname': '', 'Image': expected['Image'], 'Volumes': None,
                       'OnBuild': None, 'Healthcheck': None, 'ExposedPorts': None,
                       'Shell': None, 'StopSignal': '', 'StopTimeout': None,
                       'ArgsEscaped': False, 'NetworkDisabled': False, 'MacAddress': ''}
    for key, value in config.items():
        if key in expected and key not in ['HostConfig', 'Cmd']:
            require(value == expected[key], 'UI exact process configuration value')
            continue
        if key == 'Cmd':
            continue
        if key == 'Hostname':
            require(value == cid[:12], 'UI generated fixed hostname')
        else:
            require(key in config_defaults and value == config_defaults[key], 'UI inherited process selector')
    for key, value in expected['HostConfig'].items():
        require(host.get(key) == value, 'UI fixed host control: ' + key)
    nullable_lists = ['CapAdd', 'Devices', 'DeviceRequests', 'DeviceCgroupRules', 'Binds', 'VolumesFrom',
                      'ExtraHosts', 'GroupAdd', 'Links', 'Dns', 'DnsOptions', 'DnsSearch', 'BlkioWeightDevice',
                      'BlkioDeviceReadBps', 'BlkioDeviceWriteBps', 'BlkioDeviceReadIOps', 'BlkioDeviceWriteIOps']
    empty_strings = ['CgroupParent', 'Cgroup', 'ContainerIDFile', 'VolumeDriver', 'Isolation', 'CpusetMems']
    zeroes = ['CpuShares', 'NanoCpus', 'BlkioWeight', 'CpuRealtimePeriod', 'CpuRealtimeRuntime',
              'MemoryReservation', 'CpuCount', 'CpuPercent', 'IOMaximumIOps', 'IOMaximumBandwidth', 'OomScoreAdj']
    for key, value in host.items():
        if key in expected['HostConfig']:
            continue
        if key in nullable_lists:
            require(value in [None, []], 'UI extra host authority')
        elif key in empty_strings:
            require(value == '', 'UI inherited host selector')
        elif key in zeroes:
            require(type(value) is int and value == 0, 'UI unexpected additional numeric control')
        elif key in ['PortBindings', 'Sysctls', 'StorageOpt']:
            require(value in [None, {}], 'UI extra host mapping')
        elif key == 'MemorySwappiness':
            require(value is None, 'UI inherited swappiness')
        elif key == 'ConsoleSize':
            require(value == [0, 0], 'UI no TTY dimensions')
        elif key == 'PublishAllPorts':
            require(value is False, 'UI port publication')
        else:
            require(False, 'UI unknown host configuration key')
    mounts = state['Mounts']
    volumes = [entry for entry in mounts if entry['Type'] == 'volume']
    require(len(volumes) == 1 and volumes[0]['Name'] == expected['HostConfig']['Mounts'][0]['Source'] and
            volumes[0]['Destination'] == '/materials' and volumes[0]['RW'] is False and
            volumes[0]['Driver'] == 'local', 'UI sole read-only material volume')
    require(all(entry['Type'] == 'volume' or entry['Type'] == 'tmpfs' and
                entry['Destination'] in expected['HostConfig']['Tmpfs'] for entry in mounts), 'UI unknown runtime mount')
    require(state['RestartCount'] == 0 and state['State']['Restarting'] is False and
            state['State']['Paused'] is False and type(state['State']['OOMKilled']) is bool and
            type(state['State']['Error']) is str and len(state['State']['Error']) <= 4096,
            'UI bounded original state without restart')
    if not cleanup:
        require(state['State']['OOMKilled'] is False and not state['State']['Error'], 'UI healthy admitted state')
