"""Bounded complete visible /proc PPid inventory, including unreaped zombies."""
import os
import re
import time


def require(value, reason):
    if not value:
        raise ValueError('UI-PROC: ' + reason)


def visible(mountinfo, observer):
    found = False
    for line in mountinfo.splitlines():
        left, separator, right = line.partition(' - ')
        before, after = left.split(), right.split()
        require(separator and len(before) >= 6 and len(after) >= 3, 'complete observer mount metadata')
        if before[4] != '/proc':
            continue
        options = before[5].split(',') + after[2].split(',')
        require(not found and before[3] == '/' and after[0] == 'proc' and
                not any(option.startswith('hidepid') and option != 'hidepid=0' for option in options),
                'complete visible proc without hidden processes')
        found = True
    require(found and os.readlink('/proc/self') == str(observer), 'actual complete observer PID namespace')


def process(text, pid):
    before, separator, after = text.rpartition(')')
    prefix, opening, _ = before.partition('(')
    values = after.split()
    require(separator and opening and prefix.strip() == str(pid) and len(values) >= 20 and
            re.fullmatch('[A-Za-z]', values[0]) and re.fullmatch('[0-9]{1,10}', values[1]) and
            re.fullmatch('[0-9]{1,20}', values[19]), 'complete exact process stat identity')
    parent = int(values[1])
    require(parent <= 4294967295, 'bounded parent identity')
    return {'pid': pid, 'parent': parent, 'ticks': values[19], 'state': values[0]}


def inventory(read, deadline):
    require(time.monotonic() < deadline, 'global inventory deadline')
    observer = os.getpid()
    visible(read(f'/proc/{observer}/mountinfo', deadline, 262144), observer)
    result, total, numeric = {}, 0, 0
    with os.scandir('/proc') as entries:
        for entry in entries:
            total += 1
            require(time.monotonic() < deadline and total <= 8704, 'finite complete proc traversal')
            if not entry.name.isascii() or not entry.name.isdecimal():
                continue
            numeric += 1
            require(re.fullmatch('[1-9][0-9]{0,9}', entry.name) and numeric <= 8192,
                    'bounded canonical process inventory')
            pid = int(entry.name)
            require(pid <= 4294967295 and pid not in result, 'unique numeric process identity')
            try:
                text = read(f'/proc/{pid}/stat', deadline)
            except FileNotFoundError:
                # Only disappearance is admissible. An unreaped zombie cannot disappear.
                continue
            result[pid] = process(text, pid)
    require(time.monotonic() < deadline, 'complete process inventory finished before deadline')
    return result


def task_epoch(read, fields, task, deadline):
    result = {'statAbsent': False, 'statusAbsent': False, 'state': None, 'ticks': None, 'tgid': None}
    for name in ('stat', 'status'):
        path = f'/proc/{task}/{name}'
        try:
            text = read(path, deadline)
        except FileNotFoundError as error:
            require(error.errno == 2 and error.filename == path and
                    getattr(error, '_ui_kernel_syscall', None) == 'OPEN', 'only exact task open absence')
            result[name + 'Absent'] = True
            continue
        if name == 'stat':
            value = process(text, task)
            require(value['state'] in 'RSDZTWtXxKPI', 'known actual task state')
            result.update(state=value['state'], ticks=value['ticks'])
        else:
            value = fields(text)
            require(value['Pid'] == str(task) and re.fullmatch('[1-9][0-9]{0,9}', value['Tgid']) and
                    int(value['Tgid']) <= 4294967295, 'exact bounded task status')
            result['tgid'] = int(value['Tgid'])
    return result


def task_group(read, numbers, group, deadline):
    procs = numbers(read(group + '/cgroup.procs', deadline))
    threads = numbers(read(group + '/cgroup.threads', deadline))
    current = read(group + '/pids.current', deadline).strip()
    require(current.isascii() and current.isdecimal() and 2 <= len(procs) <= int(current) == len(threads) <= 256
            and set(procs) <= set(threads), 'complete bounded task group')
    return {'processes': procs, 'threads': threads, 'current': int(current)}


def complete_task(before, after, leaders):
    require(not any(value[key] for value in (before, after) for key in ('statAbsent', 'statusAbsent')) and
            before['state'] not in ('Z', 'X', 'x') and after['state'] not in ('Z', 'X', 'x') and
            before['ticks'] == after['ticks'] and before['tgid'] == after['tgid'] and
            before['tgid'] in leaders, 'complete stable live task epoch')


def retired_task(read, fields, numbers, original, tree, error, row, deadline):
    task = row['task']
    prior, known = row.get('priorTask'), tree.get('first', {}).get(task)
    require((row['failure'] == 'OPEN_ENOENT_FIRST' and prior is None) or
            (row['failure'] == 'OPEN_ENOENT_LAST' and type(known) is tuple and len(known) == 4 and
             prior == {'ticks': known[0], 'tgid': known[1], 'stateBefore': known[2],
                       'stateAfter': known[3], 'membershipVerified': True}), 'exact first or proven-known last membership failure')
    require(type(task) is int and task in tree['threads'] and task not in tree['processes'] and
            error.errno == 2 and error.filename == f'/proc/{task}/cgroup' and
            getattr(error, '_ui_kernel_syscall', None) == 'OPEN' and
            all(row[key][field] is True for key in ('targetBefore', 'targetAfter')
                for field in ('statAbsent', 'statusAbsent')), 'only witnessed retired first or proven-known last membership task')
    after = task_group(read, numbers, tree['group'], deadline)
    row['after'] = after
    before = row['before']
    removed = set(before['threads']) - set(after['threads'])
    require(after['processes'] == before['processes'] and set(after['threads']) < set(before['threads']) and
            before['current'] - after['current'] == 1 and removed == {task} and
            not removed & set(before['processes']), 'exact retired-task subset and task count delta')
    leaders = tree['leaders']
    for leader in leaders:
        require(process(read(f"/proc/{leader['pid']}/stat", deadline), leader['pid'])['ticks'] == leader['ticks'] and
                read(f"/proc/{leader['pid']}/cgroup", deadline) == original['membership'], 'original leader epoch and membership')
    observed = inventory(read, deadline)
    row['visibleProcesses'] = len(observed)
    row['unrelatedZombies'] = sum(value['state'] == 'Z' for value in observed.values())
    owned = {value['pid']: value for value in leaders}
    for leader in leaders:
        value = observed.get(leader['pid'])
        require(value is not None and value['ticks'] == leader['ticks'] and value['parent'] == leader['parent'] and
                value['state'] not in ('Z', 'X', 'x'), 'all original live leader identities in complete inventory')
    for child in observed.values():
        require(child['parent'] not in owned or child['pid'] in owned, 'no escaped or unreaped owned descendant')
    for leader in leaders:
        seen, cursor = set(), leader['pid']
        while cursor != original['pid']:
            require(cursor in owned and cursor not in seen, 'all original leaders descend from original init')
            seen.add(cursor)
            cursor = owned[cursor]['parent']
        require(process(read(f"/proc/{leader['pid']}/stat", deadline), leader['pid'])['ticks'] == leader['ticks'] and
                read(f"/proc/{leader['pid']}/cgroup", deadline) == original['membership'], 'leader unchanged through retirement inventory')
    require(task_group(read, numbers, tree['group'], deadline) == after, 'retirement group stable through inventory')
    absent = task_epoch(read, fields, task, deadline)
    require(absent['statAbsent'] and absent['statusAbsent'], 'retired task still absent after complete inventory')
    row['leadersStable'] = True
    row['noEscapedOwnedDescendants'] = True
