"""Original process and owned cgroup absence after creation-owned removal."""
import os
from cli_kernel_read import bounded_deadline, ticks, group_path, read, require

def gone(init, deadline):
    bounded_deadline(deadline, 2)
    try:
        current = ticks(init['pid'], deadline)
    except FileNotFoundError:
        current = None
    require(current != init['ticks'], 'original CLI init still exists')
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
                and read(group + '/pids.current', deadline) == '0\n', 'owned CLI cgroup fully empty')
    except FileNotFoundError:
        require(not os.path.lexists(group), 'missing cgroup metadata is not group removal')
        return {'originalInitAbsent': True, 'cgroup': 'removed', 'pidReused': current is not None}
    return {'originalInitAbsent': True, 'cgroup': 'empty', 'pidReused': current is not None,
            'cgroupEvents': events}
