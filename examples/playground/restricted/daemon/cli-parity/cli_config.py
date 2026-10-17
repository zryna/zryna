"""Immutable360 image and the unchanged exact compiler seccomp; operator selection only."""
import copy
import re
from cli_common import ENV, digest, exact, require, sha256
from cli_plan import PLAN_SHA256

SECCOMP_SHA256 = 'bf4a72a0da350e14260d8b1534967de60bb40dff11f088c0eeae3e455c74c33b'
TMPFS = {'/work/project': 'rw,nosuid,nodev,noexec,size=2097152,mode=0700,uid=65534,gid=65534',
         '/input': 'rw,nosuid,nodev,noexec,size=2097152,mode=0555,uid=0,gid=0',
         '/output': 'rw,nosuid,nodev,noexec,size=2097152,mode=0700,uid=65534,gid=65534',
         '/tmp': 'rw,nosuid,nodev,noexec,size=67108864,mode=1777',
         '/dev': 'rw,nosuid,noexec,dev,size=1048576,mode=0755',
         '/dev/shm': 'rw,nosuid,nodev,noexec,size=1048576,mode=1777'}


def approval(value):
    exact(value, ['format', 'version', 'image', 'layer_sha256', 'image_plan_sha256',
                  'seccomp', 'seccomp_sha256', 'command_plan_sha256'])
    require(value['format'] == 'zryna.playground-cli-owner-approval.v1' and
            type(value['version']) is int and value['version'] == 1, 'CLI approval domain')
    require(type(value['image']) is str and re.fullmatch('sha256:[a-f0-9]{64}', value['image']), 'immutable image')
    for key in ['layer_sha256', 'image_plan_sha256']:
        digest(value[key])
    require(value['seccomp_sha256'] == SECCOMP_SHA256 and type(value['seccomp']) is str and
            len(value['seccomp'].encode()) == 5459 and sha256(value['seccomp'].encode()) == SECCOMP_SHA256,
            'exact unchanged compiler default-deny policy')
    require(value['command_plan_sha256'] == PLAN_SHA256, 'fixed full19 command plan')
    return copy.deepcopy(value)


def configuration(selected, nonce):
    require(type(nonce) is str and re.fullmatch('[a-f0-9]{32}', nonce), 'creation nonce')
    return {'Image': selected['image'], 'User': '65534:65534', 'Entrypoint': ['/supervisor'],
        'Cmd': ['idle'], 'WorkingDir': '/work/project', 'Env': ENV, 'Tty': False,
        'OpenStdin': False, 'StdinOnce': False, 'AttachStdin': False, 'AttachStdout': False, 'AttachStderr': False,
        'Labels': {'zryna410.cli-owner': nonce}, 'HostConfig': {
            'ReadonlyRootfs': True, 'NetworkMode': 'none', 'CapDrop': ['ALL'], 'Privileged': False,
            'Init': False, 'IpcMode': 'none', 'CgroupnsMode': 'private', 'Runtime': 'runc',
            'PidMode': '', 'UTSMode': '', 'UsernsMode': '', 'Memory': 536870912, 'MemorySwap': 536870912,
            'PidsLimit': 32, 'CpuPeriod': 100000, 'CpuQuota': 200000, 'CpusetCpus': '0,1',
            'OomKillDisable': False, 'ShmSize': 1048576, 'Tmpfs': TMPFS,
            'RestartPolicy': {'Name': 'no', 'MaximumRetryCount': 0},
            'LogConfig': {'Type': 'none', 'Config': {}}, 'AutoRemove': False,
            'Ulimits': [{'Name': 'core', 'Soft': 0, 'Hard': 0},
                        {'Name': 'fsize', 'Soft': 268435456, 'Hard': 268435456},
                        {'Name': 'nofile', 'Soft': 1024, 'Hard': 1024}],
            'SecurityOpt': ['no-new-privileges:true', 'seccomp=' + selected['seccomp']],
            'MaskedPaths': ['/proc/acpi', '/proc/asound', '/proc/interrupts', '/proc/kcore', '/proc/keys',
                '/proc/latency_stats', '/proc/timer_list', '/proc/timer_stats', '/proc/sched_debug', '/proc/scsi',
                '/sys/firmware', '/sys/devices/virtual/powercap', '/etc/hostname', '/etc/hosts', '/etc/resolv.conf'],
            'ReadonlyPaths': ['/proc'], 'Mounts': []}}
