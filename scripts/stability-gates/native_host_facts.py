"""Bounded fact sampling core. Visible coordinates never establish host authority."""
import hashlib
import re

LIMITS = {'seconds': 8, 'operations': 768, 'rawBytes': 2 * 1024 * 1024, 'depth': 16}
FILES = {'status': 65536, 'stat': 8192, 'cgroup': 16384, 'mountinfo': 262144,
         'uid_map': 4096, 'gid_map': 4096}
NAMESPACES = ('user', 'pid', 'mnt', 'net', 'ipc', 'uts', 'cgroup')
CONTROLS = ('cpu.max', 'memory.max', 'memory.swap.max', 'pids.max',
            'cpuset.cpus.effective', 'cpuset.mems.effective', 'cgroup.type')
PROJECTIONS = ('GITHUB_REPOSITORY', 'GITHUB_SHA', 'GITHUB_REF', 'GITHUB_RUN_ID',
               'GITHUB_RUN_ATTEMPT', 'GITHUB_JOB', 'GITHUB_WORKFLOW_REF', 'GITHUB_WORKFLOW_SHA',
               'ImageOS', 'ImageVersion', 'RUNNER_OS', 'RUNNER_ARCH',
               'ZRYNA_NATIVE_SOURCE_COMMIT', 'ZRYNA_NATIVE_RUNNER_ENVIRONMENT')


class _Budget:
    def __init__(self, io):
        self.io, self.started = io, io.monotonic()
        self.operations, self.raw_bytes = 0, 0

    def sample(self, path, maximum, link=False):
        record = {'path': path, 'operation': 'readlink' if link else 'read'}
        reason = None
        if self.io.monotonic() - self.started >= LIMITS['seconds']:
            reason = 'deadline'
        elif self.operations >= LIMITS['operations']:
            reason = 'operation-budget'
        elif self.raw_bytes + maximum + 1 > LIMITS['rawBytes']:
            reason = 'raw-byte-budget'
        if reason:
            return {**record, 'status': 'unavailable', 'errno': None, 'reason': reason}
        self.operations += 1
        try:
            value = self.io.readlink(path) if link else self.io.read(path, maximum)
            data = value.encode('utf-8', errors='surrogateescape') if link else value
            if not isinstance(data, bytes) or len(data) > maximum + 1:
                raise ValueError('invalid bounded observation adapter')
            self.raw_bytes += len(data)
            record.update(bytes=len(data), sha256=hashlib.sha256(data).hexdigest(), rawHex=data.hex())
            if len(data) > maximum:
                record.update(status='oversize', errno=None, reason='first-extra-byte', readLimit=maximum)
            elif self.io.monotonic() - self.started >= LIMITS['seconds']:
                record.update(status='unavailable', errno=None, reason='deadline-after-read')
            else:
                record.update(status='observed', errno=None)
        except OSError as error:
            record.update(status='unavailable', errno=error.errno, reason='os-error', errorType=type(error).__name__)
        return record


def _text(record):
    if record['status'] != 'observed':
        raise ValueError('required raw coordinate unavailable')
    try:
        return bytes.fromhex(record['rawHex']).decode('utf-8', errors='strict')
    except UnicodeError as error:
        raise ValueError('non-UTF8 coordinate retained without interpretation') from error


def _canonical(path):
    if path == '/':
        return path
    parts = path.split('/')
    if (not path.startswith('/') or len(path) > 1024 or len(parts) - 1 > LIMITS['depth']
            or any(not re.fullmatch(r'[A-Za-z0-9_.@+-]+', part) or part in ('.', '..') for part in parts[1:])):
        raise ValueError('noncanonical or over-depth coordinate; no traversal')
    return path


def _membership(record):
    raw = _text(record)
    if not raw.startswith('0::') or not raw.endswith('\n') or raw.count('\n') != 1:
        raise ValueError('not one unambiguous cgroup2 membership')
    return _canonical(raw[3:-1])


def _mount(record):
    found = []
    for line in _text(record).splitlines():
        left, separator, right = line.partition(' - ')
        fields, filesystem = left.split(), right.split()
        if not separator or len(fields) < 6 or len(filesystem) < 3:
            raise ValueError('incomplete mountinfo; no traversal')
        if not fields[0].isdigit() or not fields[1].isdigit() or not re.fullmatch(r'[0-9]+:[0-9]+', fields[2]):
            raise ValueError('invalid mount identity syntax; no traversal')
        if filesystem[0] == 'cgroup2':
            found.append({'root': fields[3], 'mountpoint': fields[4], 'options': fields[5],
                          'superOptions': filesystem[2], 'mountId': fields[0]})
    if len(found) != 1 or found[0]['root'] != '/' or found[0]['mountpoint'] != '/sys/fs/cgroup':
        raise ValueError('missing, rebased or ambiguous cgroup2 mount; no traversal')
    return found[0]


def _namespace(record, kind):
    value = _text(record)
    if not re.fullmatch(kind + r':\[[0-9]+\]', value):
        raise ValueError('unrecognized namespace link; no traversal')
    return value


def _coordinates(subjects):
    mount = _mount(subjects['self']['files']['mountinfo'])
    for kind in ('mnt', 'cgroup'):
        if _namespace(subjects['self']['namespaces'][kind], kind) != _namespace(subjects['pid1']['namespaces'][kind], kind):
            raise ValueError('self/PID1 coordinates differ; no rebasing')
    # PID1's original view is corroboration, never a replacement coordinate system.
    other = _mount(subjects['pid1']['files']['mountinfo'])
    if any(mount[key] != other[key] for key in ('root', 'mountpoint', 'options', 'superOptions')):
        raise ValueError('different self/PID1 mount view; no traversal')
    memberships = {name: _membership(value['files']['cgroup']) for name, value in subjects.items()}
    ancestors = set()
    for group in memberships.values():
        while True:
            ancestors.add(group)
            if group == '/':
                break
            group = group.rsplit('/', 1)[0] or '/'
    return mount, memberships, sorted(ancestors)


def _projection(io):
    result = {}
    for key in PROJECTIONS:
        value = io.environment(key)
        if isinstance(value, str) and 0 < len(value) <= 2048:
            result[key] = {'status': 'observed-environment', 'value': value}
        else:
            result[key] = {'status': 'unavailable', 'reason': 'missing or outside fixed string bound'}
    return result


def _epoch(record, pid):
    try:
        raw = _text(record)
        head, separator, tail = raw.rpartition(') ')
        fields = tail.split()
        if not separator or int(head.split(' (', 1)[0]) != pid or len(fields) < 20 or not fields[19].isdigit():
            return None
        return fields[19]
    except (ValueError, IndexError):
        return None


def _comparison(before, after):
    if before['status'] != 'observed' or after['status'] != 'observed':
        return 'unavailable'
    return 'same' if before['rawHex'] == after['rawHex'] else 'changed'


def collect_native_host(io):
    """No selectors or approval arguments: all reads derive from these fixed subjects."""
    if not isinstance(io.pid, int) or isinstance(io.pid, bool) or not 1 < io.pid <= 0x7fffffff:
        raise ValueError('fixed positive observer PID required')
    budget, phases, denied = _Budget(io), {}, []
    for phase in ('before', 'after'):
        subjects = {}
        for name, pid in (('self', io.pid), ('pid1', 1)):
            subjects[name] = {'pid': pid, 'files': {}, 'namespaces': {}}
            for leaf, maximum in FILES.items():
                subjects[name]['files'][leaf] = budget.sample(f'/proc/{pid}/{leaf}', maximum)
            for kind in NAMESPACES:
                item = budget.sample(f'/proc/{pid}/ns/{kind}', 4096, link=True)
                subjects[name]['namespaces'][kind] = item
                if item['status'] == 'unavailable' and item['errno'] in (1, 13):
                    denied.append({'phase': phase, 'subject': name, 'namespace': kind, 'errno': item['errno']})
        value = {'subjects': subjects, 'projections': _projection(io), 'ancestorControls': []}
        try:
            mount, memberships, ancestors = _coordinates(subjects)
            value['mapping'] = {'status': 'visible-canonical-coordinates', 'mount': mount, 'memberships': memberships,
                                'scope': 'visible mount boundary only; slash is not proof of global root'}
            for group in ancestors:
                base = '/sys/fs/cgroup' + (group if group != '/' else '')
                value['ancestorControls'].append({'group': group,
                    'files': {leaf: budget.sample(base + '/' + leaf, 4096) for leaf in CONTROLS}})
        except ValueError as error:
            value['mapping'] = {'status': 'unavailable', 'reason': str(error), 'scope': 'raw observations retained; no traversal'}
        phases[phase] = value
    epochs, comparisons = {}, {}
    for name, pid in (('self', io.pid), ('pid1', 1)):
        before = _epoch(phases['before']['subjects'][name]['files']['stat'], pid)
        after = _epoch(phases['after']['subjects'][name]['files']['stat'], pid)
        epochs[name] = {'beforeStartTicks': before, 'afterStartTicks': after,
                        'comparison': 'unavailable' if before is None or after is None else 'same' if before == after else 'changed'}
        first, last = phases['before']['subjects'][name], phases['after']['subjects'][name]
        comparisons[name] = {leaf: _comparison(first['files'][leaf], last['files'][leaf]) for leaf in ('cgroup', 'mountinfo')}
        comparisons[name]['namespaces'] = {kind: _comparison(first['namespaces'][kind], last['namespaces'][kind]) for kind in NAMESPACES}
    return {'format': 'zryna.linux-native-host-facts.v1', 'observerPid': io.pid,
            'limits': dict(LIMITS), 'operations': budget.operations, 'rawBytes': budget.raw_bytes,
            'phases': phases, 'processEpochs': epochs, 'rawComparisons': comparisons, 'namespaceDenials': denied,
            'scope': 'read-only facts; external source/job/helper/artifact corroboration required',
            'sourceAuthenticated': False, 'hostAuthenticated': False, 'runtimeApproved': False,
            'executionApproved': False, 'proofQualified': False, 'performanceQualified': False,
            'baselineAuthority': None, 'thresholds': None}
