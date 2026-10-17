"""Closed, independently selected trusted UI test role; never a guest selector."""
import copy
import hashlib
import json
import re
from plan import ENV, digest, exact, integer, require
from owner_mounts import PROC_READONLY


def approval(value):
    exact(value, ['format', 'version', 'image', 'layer_sha256', 'seccomp', 'seccomp_sha256',
                  'material', 'ui_plan_sha256'])
    require(value['format'] == 'zryna.playground-ui-owner-approval.v1' and
            type(value['version']) is int and value['version'] == 1, 'UI owner approval domain')
    require(re.fullmatch('sha256:[a-f0-9]{64}', value['image']), 'UI immutable image')
    for key in ['layer_sha256', 'seccomp_sha256', 'ui_plan_sha256']:
        digest(value[key])
    policy = value['seccomp']
    require(type(policy) is str and 0 < len(policy.encode()) <= 65536 and
            hashlib.sha256(policy.encode()).hexdigest() == value['seccomp_sha256'], 'UI reviewed seccomp bytes')
    profile = json.loads(policy)
    require(profile.get('defaultAction') in ['SCMP_ACT_ERRNO', 'SCMP_ACT_KILL', 'SCMP_ACT_KILL_PROCESS']
            and type(profile.get('syscalls')) is list, 'UI default deny seccomp')
    pin = value['material']
    exact(pin, ['name', 'nonce', 'created_at', 'root_sha256', 'files', 'bytes'])
    require(re.fullmatch('[a-f0-9]{32}', pin['nonce']) and
            pin['name'] == 'zryna410-materials-evaluator-' + pin['nonce'], 'UI exact evaluator volume')
    digest(pin['root_sha256'])
    require(type(pin['created_at']) is str and 20 <= len(pin['created_at']) <= 64, 'UI volume creation time')
    require(type(pin['files']) is int and pin['files'] == 419, 'UI full approved evaluator419 files')
    integer(pin['bytes'], 1024**3, 1)
    return copy.deepcopy(value)


def configuration(selected, nonce):
    return {'Image': selected['image'], 'User': '65534:65534', 'Entrypoint': ['/ui-supervisor'],
            'Cmd': [], 'WorkingDir': '/tmp', 'Env': ENV, 'Tty': False, 'OpenStdin': True,
            'StdinOnce': False, 'AttachStdin': True, 'AttachStdout': True, 'AttachStderr': True,
            'Labels': {'zryna410.ui-owner': nonce}, 'HostConfig': {
                'ReadonlyRootfs': True, 'NetworkMode': 'none', 'CapDrop': ['ALL'],
                'Privileged': False, 'Init': False, 'IpcMode': 'none', 'CgroupnsMode': 'private',
                'Runtime': 'runc', 'PidMode': '', 'UTSMode': '', 'UsernsMode': '',
                'Memory': 536870912, 'MemorySwap': 536870912, 'PidsLimit': 256,
                'CpuPeriod': 100000, 'CpuQuota': 200000, 'CpusetCpus': '0,1',
                'OomKillDisable': False, 'ShmSize': 67108864,
                'Tmpfs': {'/tmp': 'rw,nosuid,nodev,noexec,size=134217728,mode=1777',
                          '/dev': 'rw,nosuid,noexec,dev,size=1048576,mode=0755',
                          '/dev/shm': 'rw,nosuid,nodev,noexec,size=67108864,mode=1777'},
                'RestartPolicy': {'Name': 'no', 'MaximumRetryCount': 0},
                'LogConfig': {'Type': 'none', 'Config': {}}, 'AutoRemove': False,
                'Ulimits': [{'Name': 'core', 'Soft': 0, 'Hard': 0},
                            {'Name': 'fsize', 'Soft': 268435456, 'Hard': 268435456},
                            {'Name': 'nofile', 'Soft': 512, 'Hard': 512}],
                'SecurityOpt': ['no-new-privileges:true', 'seccomp=' + selected['seccomp']],
                'MaskedPaths': ['/proc/acpi', '/proc/asound', '/proc/interrupts', '/proc/kcore',
                    '/proc/keys', '/proc/latency_stats', '/proc/timer_list', '/proc/timer_stats',
                    '/proc/sched_debug', '/proc/scsi', '/sys/firmware', '/sys/devices/virtual/powercap',
                    '/etc/hostname', '/etc/hosts', '/etc/resolv.conf'],
                'ReadonlyPaths': list(PROC_READONLY),
                'Mounts': [{'Type': 'volume', 'Source': selected['material']['name'], 'Target': '/materials',
                            'ReadOnly': True, 'VolumeOptions': {'NoCopy': True}}]}}
