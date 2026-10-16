"""Bounded actual kernel observations; never a browser, container or command launcher."""
import os
import math
import re
import stat
import time
import owner_proc
import owner_cgroup
import owner_mounts


def require(value, message):
    if not value:
        raise ValueError('UI-KERNEL: ' + message)


def bounded_deadline(deadline, maximum):
    require(type(deadline) in [int, float] and math.isfinite(deadline)
            and 0 < deadline - time.monotonic() <= maximum, 'finite shared owner deadline')


def read(path, deadline, maximum=16384):
    require(time.monotonic() < deadline, 'observation deadline')
    try:
        descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    except FileNotFoundError as error:
        error._ui_kernel_syscall = 'OPEN'
        raise
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
    return owner_cgroup.mapping(read, numbers, membership, cid, deadline)['path']


def security(status, pid):
    require(status['Pid'] == str(pid) and status['Threads'] == '1', 'fixed single-thread UI ancestor')
    for key in ['Uid', 'Gid']:
        require(status[key].split() == ['65534'] * 4, 'all four initial credentials 65534')
    for key in ['CapInh', 'CapPrm', 'CapEff', 'CapBnd', 'CapAmb']:
        require(status[key] == '0000000000000000', 'all initial capabilities zero')
    require(status['NoNewPrivs'] == '1' and status['Seccomp'] == '2', 'initial NNP and seccomp')
    require(status['Cpus_allowed_list'] in ['0-1', '0,1'], 'initial fixed two-CPU affinity')
    namespace = status['NSpid'].split()
    require(len(namespace) >= 2 and namespace[0] == str(pid) and namespace[-1] == '1'
            and all(value.isdigit() and int(value) > 0 for value in namespace), 'private PID1 readback')


def mounts(text, cid, proc_proof=None):
    result = owner_mounts.resolve(text, cid, proc_proof)
    for target, size in [('/tmp', 131072), ('/dev/shm', 65536)]:
        entry = result[target]
        require(entry['filesystem'] == 'tmpfs' and {'rw', 'nosuid', 'nodev', 'noexec'} <= set(entry['options'])
                and 'ro' not in entry['options'], 'actual fixed scratch flags')
        sizes = [option[5:] for option in entry['superOptions'] if option.startswith('size=')]
        require(sizes in [[str(size) + 'k'], [str(size * 1024)]], 'actual fixed scratch size')
    return result


def namespace_links(pid, deadline):
    result = {}
    for subject in ['self', str(pid)]:
        for kind in ['user', 'pid', 'mnt', 'net', 'cgroup']:
            require(time.monotonic() < deadline, 'namespace observation deadline')
            try:
                link = os.readlink(f'/proc/{subject}/ns/{kind}')
                require(re.fullmatch(kind + r':\[[0-9]+\]', link), 'actual namespace link syntax')
                result[subject + '/' + kind] = {'actualLink': link}
            except PermissionError as error:
                result[subject + '/' + kind] = {'actualLink': None, 'unavailableErrno': error.errno}
    for kind in ['pid', 'mnt', 'net', 'cgroup']:
        observer = result['self/' + kind]['actualLink']
        target = result[str(pid) + '/' + kind]['actualLink']
        require(observer is None or target is None or observer != target, 'observable private namespace ' + kind)
    return result


def network(pid, deadline, links):
    devices = read(f'/proc/{pid}/net/dev', deadline)
    names = []
    for line in devices.splitlines()[2:]:
        name, colon, counters = line.partition(':')
        require(colon and len(counters.split()) == 16 and all(x.isdigit() for x in counters.split()),
                'actual network interface counters')
        names.append(name.strip())
    require(names == ['lo'], 'only private loopback interface')
    route = read(f'/proc/{pid}/net/route', deadline)
    require(len(route.splitlines()) == 1 and route.split()[0] == 'Iface', 'no IPv4 routes')
    ipv6 = read(f'/proc/{pid}/net/ipv6_route', deadline)
    for line in ipv6.splitlines():
        row = line.split()
        require(len(row) == 10 and row[-1] == 'lo' and row[0] in ['0' * 32, '0' * 31 + '1']
                and row[1] in ['00', '80'] and row[2] == '0' * 32 and row[3] == '00'
                and row[4] == '0' * 32, 'no IPv6 external destination or next hop')
    observable = all(links[x + '/net']['actualLink'] is not None for x in ['self', str(pid)])
    return {'interfaces': devices, 'ipv4Routes': route, 'ipv6Routes': ipv6,
            'privateNamespaceLinkVerified': observable, 'namespaceIdentityUnobserved': not observable,
            'meaning': 'Actual only-loopback/no-external-routes; unavailable namespace links are not guessed.'}


def process_tree(pid, group, membership, deadline, candidate=None):
    procs = numbers(read(group + '/cgroup.procs', deadline))
    threads = numbers(read(group + '/cgroup.threads', deadline))
    current = read(group + '/pids.current', deadline).strip()
    require(current.isdigit() and 2 <= len(procs) <= int(current) == len(threads) <= 256
            and pid in procs, 'actual bounded cgroup tasks include init')
    captured = {}
    for process in procs:
        first = ticks(process, deadline)
        status = fields(read(f'/proc/{process}/status', deadline))
        require(status['Pid'] == str(process) and status['Tgid'] == str(process)
                and read(f'/proc/{process}/cgroup', deadline) == membership, 'same-group UI process')
        require(ticks(process, deadline) == first, 'stable UI process identity')
        captured[process] = {'pid': process, 'ticks': first, 'parent': int(status['PPid'])}
    owner_cgroup.note_tree(candidate, pid, group, membership, procs, threads, int(current), captured)
    for thread in threads:
        require(owner_cgroup.thread_membership(candidate, read, fields, thread, deadline) == membership, 'same-group UI thread')
    observed = owner_proc.inventory(read, deadline)
    for process in procs:
        require(process in observed and observed[process]['ticks'] == captured[process]['ticks'] and
                observed[process]['parent'] == captured[process]['parent'] and observed[process]['state'] != 'Z',
                'same original owned process in complete visible inventory')
    for child in observed.values():
        require(child['parent'] not in captured or child['pid'] in captured,
                'no escaped or unreaped owned descendant in complete PPid inventory')
    for process in procs:
        seen, cursor = set(), process
        while cursor != pid:
            require(cursor in captured and cursor not in seen, 'all UI processes descend from owned init')
            seen.add(cursor)
            cursor = captured[cursor]['parent']
    require(all(ticks(process, deadline) == captured[process]['ticks'] for process in procs),
            'all UI process identities stable through tree observation')
    require(all(read(f'/proc/{thread}/cgroup', deadline) == membership for thread in threads),
            'all original UI thread memberships stable through global observation')
    require(numbers(read(group + '/cgroup.procs', deadline)) == procs
            and numbers(read(group + '/cgroup.threads', deadline)) == threads
            and read(group + '/pids.current', deadline).strip() == current, 'stable bounded UI process tree')
    return {'processes': list(captured.values()), 'threads': threads, 'pidsCurrent': int(current),
            'childObservation': {'method': 'bounded-visible-proc-ppid-v1', 'visibleRecords': len(observed),
                                 'unrelatedZombiesObserved': sum(value['state'] == 'Z' for value in observed.values())}}


def capture(state, cid, deadline, proc_proof=None, candidate=None):
    bounded_deadline(deadline, 300)
    pid = state['State']['Pid']
    require(state['Id'] == cid and type(pid) is int and pid > 1 and state['State']['Running'] is True
            and all(state['State'][key] is False for key in ['Paused', 'Restarting', 'OOMKilled'])
            and state['RestartCount'] == 0, 'continuously running original UI init')
    first = ticks(pid, deadline)
    raw = read(f'/proc/{pid}/status', deadline)
    security(fields(raw), pid)
    membership = read(f'/proc/{pid}/cgroup', deadline)
    mapping = owner_cgroup.mapping(read, numbers, membership, cid, deadline)
    group = mapping['path']
    expected = {'memory.max': '536870912\n', 'memory.swap.max': '0\n', 'pids.max': '256\n',
                'cpu.max': '200000 100000\n'}
    actual = {name: read(group + '/' + name, deadline) for name in expected}
    require(actual == expected, 'actual fixed UI numeric cgroup controls')
    actual['cpuset.cpus.effective'] = read(group + '/cpuset.cpus.effective', deadline)
    require(actual['cpuset.cpus.effective'] in ['0-1\n', '0,1\n'], 'actual cgroup fixed CPU set')
    tree = process_tree(pid, group, membership, deadline, candidate)
    mountinfo = read(f'/proc/{pid}/mountinfo', deadline, 262144)
    observed_mounts = mounts(mountinfo, cid, proc_proof)
    links = namespace_links(pid, deadline)
    net = network(pid, deadline, links)
    require(ticks(pid, deadline) == first and read(f'/proc/{pid}/cgroup', deadline) == membership,
            'original UI init unchanged through kernel capture')
    return {'pid': pid, 'ticks': first, 'cid': cid, 'startedAt': state['State']['StartedAt'],
            'status': raw, 'membership': membership, 'cgroupPath': group, 'cgroupMapping': mapping, 'cgroup': actual,
            'processes': tree, 'mountinfo': mountinfo, 'mounts': observed_mounts,
            'namespaceLinks': links, 'network': net}


def acquire(inspect, cid, deadline, proc_proof, original, phase, record, anchors=None):
    def observe(limit, candidate):
        state = inspect()
        require(all(state['State'][key] == original[name] for key, name in [('Pid', 'pid'), ('StartedAt', 'startedAt')]) and
                state['Id'] == original['cid'] == cid and state['RestartCount'] == 0, 'retained pre-audit Docker identity')
        return capture(state, cid, limit, proc_proof, candidate)
    return owner_cgroup.acquire(observe,
        read, fields, numbers, original, cid, deadline, phase, record, anchors)


def initial(state, cid, deadline):
    pid = state['State']['Pid']
    require(state['Id'] == cid and type(pid) is int and pid > 1 and state['State']['Running'] is True,
            'original running init before detailed kernel observation')
    first = ticks(pid, deadline)
    membership = read(f'/proc/{pid}/cgroup', deadline)
    require(ticks(pid, deadline) == first, 'stable provisional init identity')
    return {'pid': pid, 'ticks': first, 'cid': cid, 'membership': membership,
            'startedAt': state['State']['StartedAt']}


def gone(init, deadline):
    bounded_deadline(deadline, 2)
    try:
        current = ticks(init['pid'], deadline)
    except FileNotFoundError:
        current = None
    require(current != init['ticks'], 'original UI init still exists')
    group = group_path(init['membership'], init['cid'], deadline)
    try:
        events = read(group + '/cgroup.events', deadline)
        values = {}
        for line in events.splitlines():
            row = line.split()
            require(len(row) == 2 and row[0] not in values and row[1] in ['0', '1'],
                    'unique numeric cgroup cleanup events')
            values[row[0]] = row[1]
        require(values.get('populated') == '0' and not read(group + '/cgroup.procs', deadline).strip()
                and not read(group + '/cgroup.threads', deadline).strip()
                and read(group + '/pids.current', deadline) == '0\n', 'owned UI cgroup fully empty')
    except FileNotFoundError:
        require(not os.path.lexists(group), 'missing cgroup metadata is not group removal')
        return {'originalInitAbsent': True, 'cgroup': 'removed', 'pidReused': current is not None}
    return {'originalInitAbsent': True, 'cgroup': 'empty', 'pidReused': current is not None,
            'cgroupEvents': events}
