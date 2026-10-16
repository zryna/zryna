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
