"""Bounded full visible-proc child scan, including zombies; ENOENT alone may disappear."""
import os
import re
import time
from cli_common import require


def process(text, pid):
    prefix, separator, tail = text.rpartition(')')
    fields = tail.split()
    require(separator and prefix.partition('(')[0].strip() == str(pid) and len(fields) >= 20
            and len(fields[0]) == 1 and fields[0].isalpha() and fields[1].isdigit()
            and fields[19].isdigit(), 'complete actual stat record')
    return int(fields[1]), fields[0], fields[19]


def visible_proc(text):
    rows, ids = [], set()
    for line in text.splitlines():
        left, separator, right = line.partition(' - ')
        before, after = left.split(), right.split()
        require(separator and len(before) >= 6 and len(after) >= 3, 'complete proc mount row')
        require(before[0].isdigit() and before[1].isdigit() and int(before[0]) > 0 and
                before[0] not in ids, 'unique actual mount IDs')
        ids.add(before[0])
        if before[4] == '/proc':
            require(before[3] == '/' and after[0] == after[1] == 'proc' and
                    all(not option.startswith('hidepid') or option == 'hidepid=0'
                        for option in before[5].split(',') + after[2].split(',')), 'visible complete proc mount')
            rows.append({'id': before[0], 'parent': before[1], 'device': before[2],
                         'options': before[5].split(','), 'superOptions': after[2].split(',')})
    require(len(rows) in [1, 2], 'one proc mount or exact readonly remount stack')
    if len(rows) == 2:
        stacks = [(base, top) for base in rows for top in rows if top['parent'] == base['id']]
        require(len(stacks) == 1, 'unique actual proc parent-child stack')
        base, top = stacks[0]
        require(base['parent'] != top['id'] and base['device'] == top['device'] and
                base['superOptions'] == top['superOptions'] and
                {'rw', 'nosuid', 'nodev', 'noexec'} <= set(base['options']) and 'ro' not in base['options'] and
                {'ro', 'nosuid', 'nodev', 'noexec'} <= set(top['options']) and 'rw' not in top['options'],
                'same full proc filesystem with exact readonly effective remount')


def children_reaped(pid, ticks, deadline, read):
    require(os.readlink('/proc/self') == str(os.getpid()), 'observer actual PID namespace identity')
    visible_proc(read('/proc/self/mountinfo', deadline, 262144))
    require(process(read(f'/proc/{pid}/stat', deadline), pid)[2] == ticks, 'original init before scan')
    found = []
    count = 0
    with os.scandir('/proc') as entries:
        for entry in entries:
            require(time.monotonic() < deadline, 'complete proc scan deadline')
            if not re.fullmatch('[0-9]+', entry.name):
                continue
            count += 1
            require(count <= 8192 and 0 < int(entry.name) <= 4294967295, 'finite complete proc inventory')
            try:
                parent, state, started = process(read('/proc/' + entry.name + '/stat', deadline), int(entry.name))
            except FileNotFoundError:
                continue
            if parent == pid:
                found.append({'pid': int(entry.name), 'state': state, 'ticks': started})
    require(process(read(f'/proc/{pid}/stat', deadline), pid)[2] == ticks, 'original init after scan')
    require(time.monotonic() < deadline, 'complete proc scan deadline')
    return {'children': found, 'numericEntries': count, 'noDeniedReadsSkipped': True,
            'zombiesIncluded': True, 'childrenReaped': not found}
