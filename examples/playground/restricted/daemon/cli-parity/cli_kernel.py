"""Before/after single PID1 and all fixed numeric/mount controls, with actual child reaping."""
from cli_kernel_read import bounded_deadline, ticks, fields, security, group_path, read, numbers, require
from cli_kernel_net import namespace_links, network
from cli_proc import children_reaped
from cli_kernel_gone import gone
import cli_cgroup
import cli_mounts


class ReapPending(ValueError):
    pass


def mounts(text, cid):
    result = cli_mounts.resolve(text, cid)
    require('/materials' not in result, 'CLI zero external material mounts')
    for target, size in [('/work/project', 2048), ('/input', 2048), ('/output', 2048),
                         ('/tmp', 65536), ('/dev/shm', 1024)]:
        entry = result[target]
        require(entry['filesystem'] == 'tmpfs' and {'rw', 'nosuid', 'nodev', 'noexec'} <= set(entry['options'])
                and 'ro' not in entry['options'], 'actual finite scratch mount flags')
        sizes = [value[5:] for value in entry['superOptions'] if value.startswith('size=')]
        require(sizes in [[str(size) + 'k'], [str(size * 1024)]], 'actual fixed tmpfs size')
    entry = result['/dev']
    require(entry['filesystem'] == 'tmpfs' and {'rw', 'nosuid', 'noexec'} <= set(entry['options'])
            and 'nodev' not in entry['options'] and 'ro' not in entry['options'] and
            [value[5:] for value in entry['superOptions'] if value.startswith('size=')] in [['1024k'], ['1048576']],
            'actual fixed1MiB device tmpfs')
    for target in result:
        require(not any(target == '/' + name or target.startswith('/' + name + '/')
                        for name in ['opt', 'app', 'resources', 'lib', 'lib64', 'supervisor']),
                'no admitted installed image subtree overmount')
        require(not any(target.startswith(path + '/') for path in
                        ['/work/project', '/input', '/output', '/tmp']), 'no scratch subtree overmount')
    return result


def capture(state, cid, deadline, cancel=None):
    bounded_deadline(deadline, 10)
    if cancel is not None:
        cancel()
    def checked_read(path, selected_deadline, maximum=16384):
        if cancel is not None:
            cancel()
        return read(path, selected_deadline, maximum)
    pid = state['State']['Pid']
    require(state['Id'] == cid and type(pid) is int and pid > 1 and state['State']['Running'] is True
            and all(state['State'][key] is False for key in ['Paused', 'Restarting', 'OOMKilled'])
            and state['RestartCount'] == 0, 'original running CLI namespace init')
    first = ticks(pid, deadline)
    raw = read(f'/proc/{pid}/status', deadline)
    security(fields(raw), pid)
    membership = read(f'/proc/{pid}/cgroup', deadline)
    mapping = cli_cgroup.mapping(checked_read, numbers, membership, cid, deadline)
    group = mapping['path']
    expected = {'memory.max': '536870912\n', 'memory.swap.max': '0\n', 'pids.max': '32\n',
                'cpu.max': '200000 100000\n'}
    actual = {name: read(group + '/' + name, deadline) for name in expected}
    require(actual == expected, 'actual fixed CLI cgroup limits')
    actual['cpuset.cpus.effective'] = read(group + '/cpuset.cpus.effective', deadline)
    require(actual['cpuset.cpus.effective'] in ['0-1\n', '0,1\n'], 'actual two-CPU cgroup set')
    def idle():
        procs = numbers(read(group + '/cgroup.procs', deadline), 32)
        threads = numbers(read(group + '/cgroup.threads', deadline), 32)
        current = read(group + '/pids.current', deadline).strip()
        require(pid in procs and pid in threads and current.isdigit() and 1 <= int(current) <= 32,
                'actual bounded tasks include original init')
        if procs != [pid] or threads != [pid] or current != '1':
            raise ReapPending('CLI-OWNER: owned tasks not yet reaped')
    idle()
    children = children_reaped(pid, first, deadline, checked_read)
    if not children['childrenReaped']:
        raise ReapPending('CLI-OWNER: owned children including zombies not yet reaped')
    idle()
    mountinfo = read(f'/proc/{pid}/mountinfo', deadline, 262144)
    observed_mounts = mounts(mountinfo, cid)
    require(observed_mounts['/sys/fs/cgroup']['root'] == membership[3:].strip(),
            'actual private cgroup mount root matches original target membership')
    links = namespace_links(pid, deadline)
    net = network(pid, deadline, links)
    require(ticks(pid, deadline) == first and read(f'/proc/{pid}/cgroup', deadline) == membership,
            'original CLI init stable through observation')
    if cancel is not None:
        cancel()
    return {'pid': pid, 'ticks': first, 'cid': cid, 'startedAt': state['State']['StartedAt'],
            'status': raw, 'membership': membership, 'cgroupPath': group, 'cgroupMapping': mapping, 'cgroup': actual,
            'children': children, 'mountinfo': mountinfo, 'mounts': observed_mounts,
            'namespaceLinks': links, 'network': net}
