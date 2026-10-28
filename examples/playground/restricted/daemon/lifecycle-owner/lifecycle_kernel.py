"""Actual private init, complete ancestors, mount and zombie-inclusive observations."""
import os
import time
from cli_common import require
from cli_kernel_read import read as kernel_read, ticks, fields, numbers
from cli_kernel_net import namespace_links, network
from cli_mounts import resolve
from cli_proc import children_reaped
from lifecycle_cgroup import ancestry, cpus


class Pending(ValueError):
    pass


class Kernel:
    def __init__(self, selected, records):
        self.selected, self.records, self.bytes, self.reads = selected, records, 0, 0

    def capture(self, state, cid, deadline):
        require(time.monotonic() < deadline, 'original shared phase deadline')
        pid = state['State']['Pid']
        require(state['Id'] == cid and type(pid) is int and pid > 1 and state['State']['Running'] is True
                and not any(state['State'][key] for key in ['Paused', 'Restarting', 'OOMKilled']) and state['RestartCount'] == 0,
                'original running private init')
        record = {'pid': pid, 'cid': cid, 'startedAt': state['State']['StartedAt'], 'reads': []}
        self.records.append(record)
        def read(path, end, maximum=16384):
            require(self.reads < 16384 and time.monotonic() < end, 'finite original kernel observation budget')
            self.reads += 1
            item = {'path': path}; record['reads'].append(item)
            try:
                raw = kernel_read(path, end, maximum)
                self.bytes += len(raw)
                require(self.bytes <= 2 * 1024 * 1024, 'complete kernel evidence retained within phase bound')
                item.update(raw=raw, errno=None)
                return raw
            except OSError as error:
                item['errno'] = error.errno
                raise
        first = ticks(pid, deadline); record['ticks'] = first
        status = fields(read(f'/proc/{pid}/status', deadline))
        require(status['Pid'] == str(pid) and status['Threads'] == '1', 'single-thread original init')
        require(all(status[key].split() == ['65534'] * 4 for key in ['Uid', 'Gid']), 'all initial credentials65534')
        require(all(status[key] == '0000000000000000' for key in ['CapInh', 'CapPrm', 'CapEff', 'CapBnd', 'CapAmb']), 'zero capabilities')
        require(status['NoNewPrivs'] == '1' and status['Seccomp'] == '2', 'actual NNP/seccomp')
        require(cpus(status['Cpus_allowed_list'] + '\n') == set(self.selected['plan']['parent']['cpus']), 'exact init CPU affinity')
        ids = status['NSpid'].split()
        require(len(ids) >= 2 and ids[0] == str(pid) and ids[-1] == '1' and all(item.isdigit() for item in ids), 'actual private PID1')
        limits = read(f'/proc/{pid}/limits', deadline)
        for name, expected in {'Max core file size': ['0', '0', 'bytes'], 'Max file size': ['268435456', '268435456', 'bytes'],
                               'Max open files': ['512', '512', 'files']}.items():
            require([line[len(name):].split() for line in limits.splitlines() if line.startswith(name)] == [expected], 'exact inherited limit: ' + name)
        membership = read(f'/proc/{pid}/cgroup', deadline)
        record['group'] = ancestry(self.selected, cid, membership, read, ticks, deadline)
        group = record['group']['path']
        record['membership'], record['cgroupPath'] = membership, group
        for key in ['uid_map', 'gid_map']:
            require(read(f'/proc/{pid}/' + key, deadline).split() == ['0', '0', '4294967295'], 'selected non-userns rootless-credential mapping')
        links = namespace_links(pid, deadline); record['namespaceLinks'] = links
        for kind in ['pid', 'mnt', 'net', 'cgroup', 'user']:
            own, target = links['self/' + kind]['actualLink'], links[str(pid) + '/' + kind]['actualLink']
            require(own is not None and target is not None, 'namespace identity cannot be unknown')
            require(own == target if kind == 'user' else own != target, 'actual selected namespace separation')
        record['network'] = network(pid, deadline, links)
        mounts = resolve(read(f'/proc/{pid}/mountinfo', deadline, 262144), cid); record['mounts'] = mounts
        require(mounts['/sys/fs/cgroup']['root'] == membership[3:-1], 'private cgroup mount matches original membership')
        require('/materials' in mounts and 'ro' in mounts['/materials']['options'] and 'rw' not in mounts['/materials']['options'], 'actual read-only material mount')
        for target, count in self.selected['plan']['parent']['writableBytes'].items():
            row = mounts[target]
            required = {'rw', 'nosuid', 'noexec'} | (set() if target == '/dev' else {'nodev'})
            require(row['filesystem'] == 'tmpfs' and required <= set(row['options']) and 'ro' not in row['options'], 'actual fixed scratch flags')
            sizes = [value[5:] for value in row['superOptions'] if value.startswith('size=')]
            require(sizes in [[str(count)], [str(count // 1024) + 'k']], 'actual fixed scratch byte limit')
        for target in mounts:
            require(not any(target.startswith(base + '/') for base in ['/materials', '/tmp', '/input', '/output']), 'no hidden scratch/material overmount')
            if target != '/':
                require(not any(file['path'] == target or file['path'].startswith(target.rstrip('/') + '/') for file in self.selected['runtime']['files']),
                        'no image/test/compiler/Python subtree shadowing')
        procs, threads = numbers(read(group + '/cgroup.procs', deadline), 32), numbers(read(group + '/cgroup.threads', deadline), 32)
        current = read(group + '/pids.current', deadline)
        record['tasks'] = {'procs': procs, 'threads': threads, 'current': current}
        if procs != [pid] or threads != [pid] or current != '1\n':
            raise Pending('owned tasks still quiescing within original deadline')
        record['children'] = children_reaped(pid, first, deadline, read)
        if not record['children']['childrenReaped']:
            raise Pending('owned zombies/children not yet reaped')
        require(ticks(pid, deadline) == first and read(f'/proc/{pid}/cgroup', deadline) == membership, 'original init stable after full observation')
        require(numbers(read(group + '/cgroup.procs', deadline), 32) == [pid] and numbers(read(group + '/cgroup.threads', deadline), 32) == [pid]
                and read(group + '/pids.current', deadline) == '1\n', 'still exactly the reaping init')
        record['quiescent'] = True
        return record

    def gone(self, init, deadline):
        require(time.monotonic() < deadline, 'shared cleanup deadline')
        try:
            current = ticks(init['pid'], deadline)
        except FileNotFoundError:
            current = None
        require(current != init['ticks'], 'original private init still exists')
        group = init['cgroupPath']
        raw = {}
        try:
            for name in ['cgroup.events', 'cgroup.procs', 'cgroup.threads', 'pids.current']:
                raw[name] = kernel_read(group + '/' + name, deadline)
            pairs = [line.split() for line in raw['cgroup.events'].splitlines()]
            require(all(len(row) == 2 for row in pairs) and len({row[0] for row in pairs}) == len(pairs), 'closed unique original cgroup events')
            events = dict(pairs)
            require(events.get('populated') == '0' and not raw['cgroup.procs'].strip()
                    and not raw['cgroup.threads'].strip() and raw['pids.current'] == '0\n', 'original cgroup completely empty')
            state = 'empty'
        except FileNotFoundError:
            require(not os.path.lexists(group), 'partial missing cgroup metadata is not group removal')
            state = 'removed'
        return {'originalInitAbsent': True, 'cgroup': state, 'pidReused': current is not None, 'cgroupRaw': raw}
