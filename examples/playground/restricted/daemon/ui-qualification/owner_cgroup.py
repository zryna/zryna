"""Two explicit read-only cgroup-v2 mount profiles; never normalize parent traversals."""
import os
import re
import time
import owner_proc


def require(value, reason):
    if not value:
        raise ValueError('UI-CGROUP: ' + reason)


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


class _Candidate:
    pass


_ACTIVE = None


def note_tree(candidate, pid, group, membership, procs, threads, current, captured):
    if candidate is None:
        return
    require(type(candidate) is _Candidate and _ACTIVE is candidate and candidate.tree is None and
            pid == candidate.original['pid'] and membership == candidate.original['membership'], 'original acquisition context')
    leaders = [dict(captured[pid]) for pid in sorted(captured)]
    if candidate.anchors is not None:
        require(leaders == candidate.anchors, 'all retained leader epochs unchanged across candidates')
    candidate.anchors = leaders
    candidate.row['leaders'] = leaders
    candidate.tree = {'processes': list(procs), 'threads': list(threads), 'current': current,
                      'group': group, 'leaders': leaders}
    candidate.row['before'] = {key: candidate.tree[key] for key in ('processes', 'threads', 'current')}


def thread_membership(candidate, read, fields, thread, deadline):
    path = f'/proc/{thread}/cgroup'
    if candidate is None:
        return read(path, deadline)
    require(type(candidate) is _Candidate and _ACTIVE is candidate and candidate.tree is not None and
            thread in candidate.tree['threads'], 'actual captured acquisition thread')
    if thread in candidate.tree['processes']:
        return read(path, deadline)
    before = owner_proc.task_epoch(read, fields, thread, deadline)
    try:
        value = read(path, deadline)
    except FileNotFoundError as error:
        candidate.error = error
        known = error.errno == 2 and error.filename == path and getattr(error, '_ui_kernel_syscall', None) == 'OPEN'
        candidate.row.update(task=thread, failure='OPEN_ENOENT_FIRST' if known else 'OTHER', targetBefore=before)
        try:
            candidate.row['targetAfter'] = owner_proc.task_epoch(read, fields, thread, deadline)
        except BaseException:
            raise error
        raise
    after = owner_proc.task_epoch(read, fields, thread, deadline)
    owner_proc.complete_task(before, after, candidate.tree['processes'])
    return value


def _init_epoch(read, original, deadline):
    first = owner_proc.process(read(f"/proc/{original['pid']}/stat", deadline), original['pid'])
    membership = read(f"/proc/{original['pid']}/cgroup", deadline)
    last = owner_proc.process(read(f"/proc/{original['pid']}/stat", deadline), original['pid'])
    require(first['ticks'] == last['ticks'] == original['ticks'] and membership == original['membership'] and
            first['state'] not in ('Z', 'X', 'x') and last['state'] not in ('Z', 'X', 'x'), 'original init epoch remains live')


def acquire(capture, read, fields, numbers, original, cid, deadline, phase, record, anchors=None):
    global _ACTIVE
    require(_ACTIVE is None and phase in ('POST_PROC_AUDIT', 'POST_MATERIAL_AUDIT') and
            type(record) is dict and not record, 'fixed fresh pre-capability acquisition')
    started = time.monotonic()
    window = min(deadline, started + 1)
    record.update(format='zryna.ui-kernel-acquisition.v1', version=1, phase=phase, status='started',
                  windowMs=1000, attempts=[], acceptedAttempt=None)
    if anchors is not None:
        anchors = [dict(value) for value in sorted(anchors, key=lambda value: value['pid'])]
    last_error = None
    try:
        require(cid == original['cid'], 'retained original container identity')
        for number in range(1, 4):
            if time.monotonic() >= window and last_error is not None:
                raise last_error
            require(time.monotonic() < window, 'single bounded acquisition window')
            row = {'number': number, 'status': 'started', 'failure': None, 'task': None, 'before': None, 'after': None,
                   'targetBefore': None, 'targetAfter': None, 'initStable': None, 'leadersStable': None,
                   'visibleProcesses': None, 'unrelatedZombies': None, 'noEscapedOwnedDescendants': None, 'leaders': None}
            record['attempts'].append(row)
            candidate = _Candidate()
            candidate.original, candidate.anchors, candidate.tree, candidate.error, candidate.row = dict(original), anchors, None, None, row
            _ACTIVE = candidate
            try:
                _init_epoch(read, original, window)
                result = capture(window, candidate)
                require(all(result[key] == original[key] for key in ('pid', 'ticks', 'cid', 'membership', 'startedAt')),
                        'successful candidate is retained initial identity')
                require(candidate.tree is not None and sorted(result['processes']['processes'], key=lambda value: value['pid']) == candidate.anchors,
                        'successful capture retains all original leader anchors')
                _init_epoch(read, original, window)
            except FileNotFoundError as error:
                row['status'] = 'fatal'
                if candidate.error is not error or candidate.tree is None:
                    raise
                try:
                    _init_epoch(read, original, window)
                    row['initStable'] = True
                    owner_proc.retired_task(read, fields, numbers, original, candidate.tree, error, row, window)
                    _init_epoch(read, original, window)
                    require(time.monotonic() < window, 'retirement proof before original acquisition deadline')
                except BaseException:
                    raise error
                row['status'] = 'retired-task'
                anchors = candidate.anchors
                last_error = error
                if number == 3:
                    raise
                continue
            except BaseException:
                row['status'] = 'fatal'
                raise
            finally:
                _ACTIVE = None
            row.update(status='accepted', initStable=True, leadersStable=True, noEscapedOwnedDescendants=True,
                       after={'processes': [value['pid'] for value in result['processes']['processes']],
                              'threads': result['processes']['threads'], 'current': result['processes']['pidsCurrent']},
                       visibleProcesses=result['processes']['childObservation']['visibleRecords'],
                       unrelatedZombies=result['processes']['childObservation']['unrelatedZombiesObserved'])
            record.update(status='accepted', acceptedAttempt=number)
            return result
    except BaseException:
        record['status'] = 'failed'
        raise
