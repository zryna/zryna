"""Actual native-VM coordinates and every effective ancestor; no rebasing or unlimited fallback."""
import errno
import os
import re
from cli_common import require, sha256
from cli_engine import decode
from lifecycle_inputs import path

CONTROLS = ['memory.max', 'memory.swap.max', 'pids.max', 'cpu.max', 'cpuset.cpus.effective', 'cpuset.mems.effective', 'cgroup.type']
ROOT_ONLY = {'memory.max', 'memory.swap.max', 'pids.max', 'cpu.max', 'cgroup.type'}
ROOT_RESOURCES = ROOT_ONLY - {'cgroup.type'}


def cpus(raw):
    require(type(raw) is str and 0 < len(raw) <= 4096 and raw.endswith('\n'), 'bounded original cpuset')
    found = set()
    for part in raw.strip().split(','):
        require(re.fullmatch('[0-9]+(?:-[0-9]+)?', part), 'closed cpuset grammar')
        ends = [int(value) for value in part.split('-')]
        first, last = ends[0], ends[-1]
        require(0 <= first <= last <= 4095 and not found.intersection(range(first, last + 1)), 'unique finite cpuset')
        found.update(range(first, last + 1))
    return found


def mount(text):
    rows = []
    for line in text.splitlines():
        left, separator, right = line.partition(' - ')
        fields, filesystem = left.split(), right.split()
        require(separator and len(fields) >= 6 and len(filesystem) >= 3, 'complete original observer mountinfo')
        if filesystem[0] == 'cgroup2':
            rows.append({'root': fields[3], 'mountpoint': fields[4], 'mountId': fields[0]})
    require(len(rows) == 1 and rows[0]['root'] == '/' and rows[0]['mountpoint'] == '/sys/fs/cgroup', 'one unrebased native root cgroup2 coordinate')
    return rows[0]


def original(record):
    require(record['status'] == 'observed', 'independently retained context coordinate available')
    raw = bytes.fromhex(record['rawHex'])
    require(len(raw) == record['bytes'] and sha256(raw) == record['sha256'], 'original context byte identity')
    return raw.decode('ascii', errors='strict')


def native_context(selected, read, ticks, deadline):
    captured = selected['nativeVm']; require(sha256(captured['raw']) == captured['sha256'], 'separate same-VM context pin')
    value = decode(captured['raw'])
    require(value['format'] == 'zryna.linux-native-host-facts.v1' and value['producer']['effectiveUid'] == 0, 'original root native-VM context')
    missing, namespaces, expected_mount = set(), {}, None
    for phase in ['before', 'after']:
        row = value['phases'][phase]
        require(row['mapping']['status'] == 'visible-canonical-coordinates'
                and row['mapping']['mount']['root'] == '/' and row['mapping']['mount']['mountpoint'] == '/sys/fs/cgroup'
                and row['projections']['ZRYNA_NATIVE_RUNNER_ENVIRONMENT'] == {'status': 'observed-environment', 'value': 'github-hosted'},
                'separately corroborated native job root coordinates; not a caller approval flag')
        selected_mount = {key: row['mapping']['mount'][key] for key in ['root', 'mountpoint', 'mountId']}
        require(expected_mount is None or expected_mount == selected_mount, 'stable recorded mount coordinate')
        expected_mount = selected_mount
        root = [group for group in row['ancestorControls'] if group['group'] == '/']
        require(len(root) == 1, 'original context has exactly one visible root observation')
        absent = set()
        for key in CONTROLS:
            item = root[0]['files'][key]
            if item['status'] == 'unavailable':
                require(key in ROOT_ONLY and item['errno'] == errno.ENOENT, 'only documented root ENOENT can be unavailable')
                absent.add(key)
            else:
                original(item)
        require(ROOT_RESOURCES <= absent, 'fixed enabled resource controls must be absent at genuine hierarchy root')
        if phase == 'before':
            missing = absent
        else:
            require(missing == absent, 'same exact documented root-only absent control set')
        pid1 = row['subjects']['pid1']
        for key in ['uid_map', 'gid_map']:
            require(original(pid1['files'][key]).split() == ['0', '0', '4294967295'], 'initial native identity map')
            require(read('/proc/1/' + key, deadline).split() == ['0', '0', '4294967295'], 'same actual native identity map')
        for kind in ['pid', 'mnt', 'cgroup', 'user']:
            link = original(pid1['namespaces'][kind])
            require(re.fullmatch(kind + r':\[[0-9]+\]', link), 'actual recorded native namespace')
            require(kind not in namespaces or namespaces[kind] == link, 'stable recorded namespace')
            require(os.readlink('/proc/1/ns/' + kind) == link == os.readlink('/proc/self/ns/' + kind), 'same VM, not hidden observer namespaces')
            namespaces[kind] = link
    for key in CONTROLS:
        try:
            read('/sys/fs/cgroup/' + key, deadline)
        except FileNotFoundError as error:
            require(error.errno == errno.ENOENT and key in missing, 'live root presence must match selected raw context')
        else:
            require(key not in missing, 'selected absent root control became readable')
    epoch = value['processEpochs']['pid1']
    require(epoch['comparison'] == 'same' and epoch['beforeStartTicks'] == epoch['afterStartTicks'] == ticks(1, deadline), 'same original native PID1 birth')
    require(mount(read('/proc/self/mountinfo', deadline, 262144)) == expected_mount, 'same actual host mount coordinate')
    return {'sha256': captured['sha256'], 'pid1Ticks': epoch['beforeStartTicks'], 'namespaces': namespaces,
            'mount': expected_mount, 'rootOnlyAbsent': sorted(missing)}


def ancestry(selected, cid, membership, read, ticks, deadline):
    context = native_context(selected, read, ticks, deadline)
    require(membership.startswith('0::/') and membership.endswith('\n') and membership.count('\n') == 1, 'one actual cgroup2 membership')
    leaf = membership[3:-1]; path(leaf)
    expected = '/system.slice/docker-' + cid + '.scope' if selected['engine']['cgroupDriver'] == 'systemd' else '/docker/' + cid
    require(leaf == expected, 'exact selected native Docker driver/container leaf')
    groups, current = [], leaf
    while True:
        groups.append(current)
        require(len(groups) <= 17, 'finite complete native ancestor chain')
        if current == '/':
            break
        current = current.rsplit('/', 1)[0] or '/'
    rows = []
    for group in groups:
        base = '/sys/fs/cgroup' + (group if group != '/' else '')
        controls = {}
        for key in CONTROLS:
            try:
                controls[key] = {'raw': read(base + '/' + key, deadline), 'errno': None}
                require(group != '/' or key not in context['rootOnlyAbsent'], 'root availability changed during live ancestor walk')
            except FileNotFoundError as error:
                require(group == '/' and error.errno == errno.ENOENT and key in context['rootOnlyAbsent'], 'missing non-root or unadmitted control')
                controls[key] = {'raw': None, 'errno': error.errno, 'meaning': 'documented root-only absence under independently pinned same-VM context'}
        if group == leaf:
            for key, expected_raw in {'memory.max': '536870912\n', 'memory.swap.max': '0\n', 'pids.max': '32\n', 'cpu.max': '200000 100000\n', 'cgroup.type': 'domain\n'}.items():
                require(controls[key]['raw'] == expected_raw, 'exact leaf envelope: ' + key)
            require(cpus(controls['cpuset.cpus.effective']['raw']) == set(selected['plan']['parent']['cpus']), 'exact leaf CPU set')
        for key, minimum in [('memory.max', 536870912), ('pids.max', 32), ('memory.swap.max', 0)]:
            raw = controls[key]['raw']
            require(raw is None or raw == 'max\n' or re.fullmatch('[0-9]+\n', raw) and int(raw) >= minimum, 'ancestor bounds permit selected envelope')
        raw = controls['cpu.max']['raw']
        if raw is not None:
            require(re.fullmatch('(max|[0-9]+) [1-9][0-9]*\n', raw), 'exact CPU quota/period units')
            quota, period = raw.split()
            require(quota == 'max' or int(quota) >= 2 * int(period), 'two CPUs under every actual ancestor quota')
        require(set(selected['plan']['parent']['cpus']) <= cpus(controls['cpuset.cpus.effective']['raw']), 'selected CPUs permitted by every ancestor')
        cpus(controls['cpuset.mems.effective']['raw'])
        require(controls['cgroup.type']['raw'] in [None, 'domain\n'], 'actual supported domain hierarchy')
        rows.append({'group': group, 'files': controls})
    return {'context': context, 'membership': membership, 'path': '/sys/fs/cgroup' + leaf, 'ancestors': rows}
