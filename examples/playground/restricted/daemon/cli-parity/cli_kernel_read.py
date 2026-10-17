"""Bounded actual kernel observations; never a browser, container or command launcher."""
import os
import math
import re
import stat
import time
import cli_cgroup


def require(value, message):
    if not value:
        raise ValueError('CLI-KERNEL: ' + message)


def bounded_deadline(deadline, maximum):
    require(type(deadline) in [int, float] and math.isfinite(deadline)
            and 0 < deadline - time.monotonic() <= maximum, 'finite shared owner deadline')


def read(path, deadline, maximum=16384):
    require(time.monotonic() < deadline, 'observation deadline')
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    try:
        require(stat.S_ISREG(os.fstat(descriptor).st_mode), 'ordinary kernel metadata')
        data = bytearray()
        while True:
            block = os.read(descriptor, min(4096, maximum + 1 - len(data)))
            if not block:
                break
            data.extend(block)
            require(len(data) <= maximum, 'kernel first-extra-byte bound')
        require(time.monotonic() < deadline, 'observation deadline')
        return data.decode('ascii', errors='strict')
    finally:
        os.close(descriptor)


def ticks(pid, deadline):
    text = read(f'/proc/{pid}/stat', deadline)
    fields = text.rsplit(')', 1)[-1].split()
    require(text.startswith(str(pid) + ' (') and len(fields) > 19 and fields[19].isdigit(),
            'stable process start identity')
    return fields[19]


def fields(text):
    result = {}
    for line in text.splitlines():
        key, colon, value = line.partition(':')
        require(colon and key not in result, 'unique process status fields')
        result[key] = value.strip()
    return result


def numbers(text, maximum=256):
    values = text.split()
    require(len(values) <= maximum and all(re.fullmatch('[1-9][0-9]{0,9}', x) for x in values),
            'bounded positive process identities')
    result = [int(value) for value in values]
    require(len(set(result)) == len(result), 'unique process identities')
    return sorted(result)


def group_path(membership, cid, deadline):
    return cli_cgroup.mapping(read, numbers, membership, cid, deadline)['path']


def security(status, pid):
    require(status['Pid'] == str(pid) and status['Threads'] == '1', 'fixed single-thread CLI ancestor')
    for key in ['Uid', 'Gid']:
        require(status[key].split() == ['65534'] * 4, 'all four initial credentials 65534')
    for key in ['CapInh', 'CapPrm', 'CapEff', 'CapBnd', 'CapAmb']:
        require(status[key] == '0000000000000000', 'all initial capabilities zero')
    require(status['NoNewPrivs'] == '1' and status['Seccomp'] == '2', 'initial NNP and seccomp')
    require(status['Cpus_allowed_list'] in ['0-1', '0,1'], 'initial fixed two-CPU affinity')
    namespace = status['NSpid'].split()
    require(len(namespace) >= 2 and namespace[0] == str(pid) and namespace[-1] == '1'
            and all(value.isdigit() and int(value) > 0 for value in namespace), 'private PID1 readback')
