"""Closed private selections. Pins are operator inputs, never imported approval booleans."""
import copy
import json
import re
from cli_common import digest, exact, integer, require, sha256
from preparation_plan import ENV, MATERIALS, path, canonical as _canonical, runtime_inventory
from preparation_volume import volume
from preparation import image_state

NAME = 'authenticated_large_provider_stages_then_compiles_and_removes_owned_stage'
ORDINARY = ['inherited_one_mib_hard_limit_rejects_before_material_creation',
            'malformed_request_stops_before_material_capture_on_each_host']
SECCOMP_SHA = 'bf4a72a0da350e14260d8b1534967de60bb40dff11f088c0eeae3e455c74c33b'
FLAGS = {'sourceAuthenticated': False, 'hostAuthenticated': False, 'runtimeApproved': False,
         'executionApproved': False, 'proofQualified': False, 'performanceQualified': False,
         'baselineAuthority': None, 'thresholds': None}
TMPFS = {'/input': 'rw,nosuid,nodev,noexec,size=2097152,mode=0555,uid=0,gid=0',
         '/output': 'rw,nosuid,nodev,noexec,size=2097152,mode=0700,uid=65534,gid=65534',
         '/tmp': 'rw,nosuid,nodev,noexec,size=67108864,mode=1777',
         '/dev': 'rw,nosuid,noexec,dev,size=1048576,mode=0755',
         '/dev/shm': 'rw,nosuid,nodev,noexec,size=1048576,mode=1777'}


def canonical(value):
    raw = _canonical(value)
    require(len(raw) <= 262144, 'bounded canonical plan/runtime')
    return raw


def modes():
    return [{'id': 'default', 'executable': '/lifecycle/test', 'args': ['--test-threads=1', '--color', 'never'],
             'counts': {'passed': 2, 'failed': 0, 'ignored': 1, 'measured': 0, 'filtered': 0},
             'requiredIgnored': [NAME], 'requiredPassed': ORDINARY},
            {'id': 'positive', 'executable': '/lifecycle/test', 'args': ['--ignored', '--exact', NAME, '--test-threads=1', '--color', 'never'],
             'counts': {'passed': 1, 'failed': 0, 'ignored': 0, 'measured': 0, 'filtered': 2}, 'requiredPassed': [NAME]}]


def pin(value):
    exact(value, ['path', 'bytes', 'sha256']); path(value['path']); digest(value['sha256']); integer(value['bytes'], 128 * 1024**2, 1)


def parent_profile(cpus):
    return {'uid': 65534, 'gid': 65534, 'readonlyRoot': True, 'network': 'none', 'capabilities': [], 'noNewPrivileges': True,
        'privateNamespaces': ['pid', 'mount', 'network', 'cgroup'], 'memoryBytes': 536870912, 'swapBytes': 0, 'tasks': 32,
        'seccomp': {'bytes': 5459, 'sha256': SECCOMP_SHA}, 'cpus': cpus, 'cpuQuota': 200000, 'cpuPeriod': 100000,
        'executionMs': 10000, 'cleanupMs': 2000, 'inheritedLimits': {'core': 0, 'fsize': 268435456, 'nofile': 512},
        'stagingFsize': 12582912, 'runtimeLimits': {'fsize': 1277956, 'nofile': 128},
        'materialMount': {'path': '/materials', 'readOnly': True, 'noCopy': True},
        'writableBytes': {'/tmp': 67108864, '/input': 2097152, '/output': 2097152, '/dev': 1048576, '/dev/shm': 1048576}}


def validate(selected):
    selected = copy.deepcopy(selected)
    exact(selected, ['plan', 'planSha256', 'runtime', 'image', 'capture', 'materials', 'engine', 'nativeVm'])
    plan, runtime = selected['plan'], selected['runtime']
    exact(plan['source'], ['commit', 'tree'])
    require(all(type(value) is str and re.fullmatch('[a-f0-9]{40}', value) for value in plan['source'].values()), 'selected immutable source coordinates')
    digest(selected['planSha256']); require(sha256(canonical(plan)) == selected['planSha256'], 'separately selected original planner bytes')
    require(plan['format'] == 'zryna.linux-lifecycle-owner-plan.v1' and plan['row'] == 'zryna-playground-compiler/lifecycle/' + NAME, 'fixed row')
    require(plan['modes'] == modes() and plan['environment'] == dict(entry.split('=', 1) for entry in ENV), 'exact two modes/environment')
    for key in ['sourceAuthenticated', 'ownerExecuted', 'materialsApproved', 'runtimeApproved', 'hostApproved',
                'executionApproved', 'proofQualified', 'performanceQualified']:
        require(plan[key] is False, 'planner flags remain unapproved')
    require(plan['baselineAuthority'] is None and plan['thresholds'] is None, 'no policy admission')
    require(plan['parent'] == parent_profile(plan['parent']['cpus']), 'unchanged complete parent contract')
    cpus = plan['parent']['cpus']; require(type(cpus) is list and len(cpus) == 2 and cpus == sorted(set(cpus)), 'two selected CPUs')
    for cpu in cpus: integer(cpu, 4095)
    exact(selected['capture'], ['source', 'compiler', 'test', 'raw'])
    require(selected['capture']['source'] == plan['source'] and selected['capture']['raw'] == plan['prebuild']['raw'], 'source/capture pins')
    for key in ['compiler', 'test']:
        pin(selected['capture'][key]); require(selected['capture'][key] == plan['prebuild']['artifacts'][key], 'original Cargo artifact selection')
    compiler = selected['capture']['compiler']['path']
    require(compiler.endswith('/release/zryna-playground-compiler') and
            not any(compiler == root or compiler.startswith(root + '/') for root in ['/tmp', '/materials', '/input', '/output', '/dev', '/proc', '/sys', '/lifecycle', '/usr', '/lib', '/lib64', '/resources']), 'original unshadowed compiler path')
    require(sha256(canonical(runtime)) == plan['runtime']['sha256'], 'separately selected complete runtime descriptor')
    by_name = runtime_inventory(runtime, plan)
    image = selected['image']; exact(image, ['id', 'layerSha256'])
    require(re.fullmatch('sha256:[a-f0-9]{64}', image['id']), 'immutable image ID'); digest(image['layerSha256'])
    material = selected['materials']; exact(material, ['name', 'nonce', 'createdAt', 'manifestSha256', 'manifestBytes', 'files', 'bytes'])
    require(re.fullmatch('zryna410-materials-compiler-[a-f0-9]{32}', material['name']) and re.fullmatch('[a-f0-9]{32}', material['nonce']), 'creation-owned material name')
    require(material['name'].endswith(material['nonce']) and type(material['createdAt']) is str and 20 <= len(material['createdAt']) <= 64, 'same prepared material identity')
    digest(material['manifestSha256']); integer(material['manifestBytes'], 262144, 1)
    require(material['files'] == 10 and material['bytes'] == 133968848 and plan['materials']['bytes'] == material['bytes'] and plan['materials']['files'] == MATERIALS, 'exact original provider/Node selection')
    manifest = by_name.get('/resources/materials-manifest.json')
    require(manifest and manifest['sha256'] == material['manifestSha256'] and manifest['bytes'] == material['manifestBytes'] and manifest['mode'] == 0o444, 'independent immutable material manifest')
    engine = selected['engine']; exact(engine, ['id', 'version', 'apiVersion', 'cgroupDriver', 'dockerRootDir'])
    require(all(type(engine[key]) is str and 0 < len(engine[key]) <= 256 for key in engine), 'explicit Engine identity')
    require(engine['cgroupDriver'] in ['systemd', 'cgroupfs'], 'selected native driver'); path(engine['dockerRootDir'])
    exact(selected['nativeVm'], ['raw', 'sha256']); digest(selected['nativeVm']['sha256'])
    require(type(selected['nativeVm']['raw']) is bytes and 0 < len(selected['nativeVm']['raw']) <= 6291456 and sha256(selected['nativeVm']['raw']) == selected['nativeVm']['sha256'], 'independently corroborated same-VM root context')
    return selected


def configuration(selected, nonce, seccomp):
    require(re.fullmatch('[a-f0-9]{32}', nonce) and len(seccomp.encode()) == 5459 and sha256(seccomp.encode()) == SECCOMP_SHA, 'fixed seccomp and nonce')
    return {'Image': selected['image']['id'], 'User': '65534:65534', 'Entrypoint': ['/lifecycle/supervisor'], 'Cmd': ['idle'],
        'WorkingDir': '/tmp', 'Env': ENV, 'Tty': False, 'OpenStdin': False, 'StdinOnce': False,
        'AttachStdin': False, 'AttachStdout': False, 'AttachStderr': False, 'Labels': {'zryna418.lifecycle-owner': nonce},
        'HostConfig': {'ReadonlyRootfs': True, 'NetworkMode': 'none', 'CapDrop': ['ALL'], 'Privileged': False,
            'Init': False, 'IpcMode': 'none', 'CgroupnsMode': 'private', 'Runtime': 'runc', 'PidMode': '', 'UTSMode': '', 'UsernsMode': '',
            'Memory': 536870912, 'MemorySwap': 536870912, 'PidsLimit': 32, 'CpuPeriod': 100000, 'CpuQuota': 200000,
            'CpusetCpus': ','.join(map(str, selected['plan']['parent']['cpus'])), 'OomKillDisable': False, 'ShmSize': 1048576,
            'Tmpfs': TMPFS, 'RestartPolicy': {'Name': 'no', 'MaximumRetryCount': 0}, 'LogConfig': {'Type': 'none', 'Config': {}}, 'AutoRemove': False,
            'Ulimits': [{'Name': 'core', 'Soft': 0, 'Hard': 0}, {'Name': 'fsize', 'Soft': 268435456, 'Hard': 268435456}, {'Name': 'nofile', 'Soft': 512, 'Hard': 512}],
            'SecurityOpt': ['no-new-privileges:true', 'seccomp=' + seccomp],
            'MaskedPaths': ['/proc/acpi', '/proc/asound', '/proc/interrupts', '/proc/kcore', '/proc/keys', '/proc/latency_stats', '/proc/timer_list',
                '/proc/timer_stats', '/proc/sched_debug', '/proc/scsi', '/sys/firmware', '/sys/devices/virtual/powercap', '/etc/hostname', '/etc/hosts', '/etc/resolv.conf'],
            'ReadonlyPaths': ['/proc'], 'Mounts': [{'Type': 'volume', 'Source': selected['materials']['name'], 'Target': '/materials',
                                                 'ReadOnly': True, 'VolumeOptions': {'NoCopy': True}}]}}


def image_identity(value, selected):
    # This extra identity restriction is stricter than the legacy image reader.
    require(value['Config'].get('Image', '') in ['', selected['image']['id']], 'same image configuration identity')
    image_state(value, selected['image']['id'], selected['image']['layerSha256'])


def volume_identity(value, selected):
    material = selected['materials']
    volume(value, material['name'], material['nonce'], material['manifestSha256'],
           selected['engine']['dockerRootDir'] + '/volumes/' + material['name'] + '/_data', material['createdAt'])


def container_identity(state, cid, name, expected, selected):
    require(state['Id'] == cid and state['Name'] == '/' + name and state['Image'] == selected['image']['id'], 'creation-owned original object')
    config, host = state['Config'], state['HostConfig']
    required = {key: field for key, field in expected.items() if key != 'HostConfig'}
    require(all(config.get(key) == field for key, field in required.items()), 'fixed process/no inherited selectors')
    defaults = {'Hostname': cid[:12], 'Domainname': '', 'Image': expected['Image'], 'Volumes': None, 'OnBuild': None,
                'Healthcheck': None, 'ExposedPorts': None, 'Shell': None, 'StopSignal': '', 'StopTimeout': None,
                'ArgsEscaped': False, 'NetworkDisabled': False, 'MacAddress': ''}
    require(all(key in required or key in defaults and field == defaults[key] for key, field in config.items()), 'unknown process selector')
    for key, field in expected['HostConfig'].items():
        require(host.get(key) == field or key == 'OomKillDisable' and field is False and key in host and host[key] is None,
                'exact host control: ' + key)
    lists = ['CapAdd', 'Devices', 'DeviceRequests', 'DeviceCgroupRules', 'Binds', 'VolumesFrom', 'ExtraHosts', 'GroupAdd', 'Links', 'Dns', 'DnsOptions', 'DnsSearch',
             'BlkioWeightDevice', 'BlkioDeviceReadBps', 'BlkioDeviceWriteBps', 'BlkioDeviceReadIOps', 'BlkioDeviceWriteIOps']
    strings = ['CgroupParent', 'Cgroup', 'ContainerIDFile', 'VolumeDriver', 'Isolation', 'CpusetMems']
    zeroes = ['CpuShares', 'NanoCpus', 'BlkioWeight', 'CpuRealtimePeriod', 'CpuRealtimeRuntime', 'MemoryReservation', 'CpuCount', 'CpuPercent',
              'IOMaximumIOps', 'IOMaximumBandwidth', 'OomScoreAdj']
    for key, field in host.items():
        if key in expected['HostConfig']:
            continue
        allowed = (key in lists and field in [None, []] or key in strings and field == '' or key in zeroes and type(field) is int and field == 0
                   or key in ['PortBindings', 'Sysctls', 'StorageOpt'] and field in [None, {}] or key == 'MemorySwappiness' and field is None
                   or key == 'ConsoleSize' and field == [0, 0] or key == 'PublishAllPorts' and field is False)
        require(allowed, 'unknown/inherited host authority')
    material = [row for row in state['Mounts'] if row['Destination'] == '/materials']
    require(len(material) == 1 and material[0]['Type'] == 'volume' and material[0]['Name'] == selected['materials']['name']
            and material[0]['Driver'] == 'local' and material[0]['RW'] is False and material[0]['Source'] ==
            selected['engine']['dockerRootDir'] + '/volumes/' + selected['materials']['name'] + '/_data', 'actual read-only selected material volume')
    require(all(row in material or row['Type'] == 'tmpfs' and row['Destination'] in TMPFS for row in state['Mounts']), 'no extra mounts')
    require(state['RestartCount'] == 0 and state['State']['Paused'] is False and state['State']['Restarting'] is False
            and type(state['State']['OOMKilled']) is bool and type(state['State']['Error']) is str and len(state['State']['Error']) <= 4096,
            'original bounded process state')
