"""Two explicit read-only cgroup-v2 mount profiles; never normalize parent traversals."""
import os
import re


def require(value, reason):
    if not value:
        raise ValueError('CLI-CGROUP: ' + reason)


def canonical(value):
    return type(value) is str and value.startswith('/') and all(
        re.fullmatch('[A-Za-z0-9_.@+-]+', part) and part not in ['.', '..']
        for part in value.split('/')[1:])


def mapping(read, numbers, membership, cid, deadline):
    require(type(cid) is str and re.fullmatch('[a-f0-9]{64}', cid), 'exact original container identity')
    observer = os.getpid()
    mountinfo = read(f'/proc/{observer}/mountinfo', deadline, 262144)
    entries = []
    for line in mountinfo.splitlines():
        left, separator, right = line.partition(' - ')
        before, after = left.split(), right.split()
        require(separator and len(before) >= 6 and len(after) >= 3, 'complete observer mount metadata')
        if before[4] == '/sys/fs/cgroup':
            require(after[0] == 'cgroup2' and 'ro' in before[5].split(',') and
                    'rw' not in before[5].split(','), 'actual read-only cgroup-v2 mount')
            entries.append({'root': before[3], 'mountOptions': before[5], 'superOptions': after[2]})
    require(len(entries) == 1, 'sole cgroup-v2 mount')
    mount = entries[0]
    own = read(f'/proc/{observer}/cgroup', deadline)
    if mount['root'] == '/..':
        require(own == '0::/\n' and membership == '0::/../docker/' + cid + '\n',
                'exact managed-cloud sibling Docker topology')
        observer_group, selected = '/sys/fs/cgroup/init', '/sys/fs/cgroup/docker/' + cid
        profile = 'managed-init-sibling-docker-v1'
    else:
        require(mount['root'] == '/' and own.startswith('0::/') and own.endswith('\n') and
                own.count('\n') == 1 and membership.startswith('0::/') and membership.endswith('\n') and
                membership.count('\n') == 1, 'exact full-root cgroup-v2 topology')
        relative, own_relative = membership[3:-1], own[3:-1]
        require(canonical(relative) and relative.endswith('/docker/' + cid) and
                (own_relative == '/' or canonical(own_relative)), 'canonical full-root cgroup membership')
        observer_group = '/sys/fs/cgroup' + (own_relative if own_relative != '/' else '')
        selected = '/sys/fs/cgroup' + relative
        profile = 'full-root-canonical-docker-v1'
    own_procs = numbers(read(observer_group + '/cgroup.procs', deadline), 8192)
    require(observer in own_procs and read(f'/proc/{observer}/cgroup', deadline) == own,
            'actual observer PID establishes approved mount coordinate system')
    return {'path': selected, 'profile': profile, 'observerPid': observer,
            'observerMembership': own, 'observerGroup': observer_group, 'observerProcs': own_procs,
            'mount': mount, 'mountinfo': mountinfo}
