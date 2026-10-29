"""Synthetic observations only. No Engine, process, filesystem or native guest access."""
import copy
import importlib.util
from preparation_fixture import selection
from pathlib import Path


def load_owner():
    spec = importlib.util.spec_from_file_location('private_lifecycle_owner_test', Path(__file__).with_name('owner.py'))
    owner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(owner)
    return owner


OWNER = load_owner()
INPUT = OWNER._load('lifecycle_inputs')
OPERATION = OWNER._load('lifecycle_operation')
EXECUTE = OWNER._load('lifecycle_execute')
SHA = '1' * 64


def selected():
    prepared = selection()
    runtime = prepared['runtime']
    compiler = copy.deepcopy(prepared['plan']['prebuild']['artifacts']['compiler'])
    test = copy.deepcopy(prepared['plan']['prebuild']['artifacts']['test'])
    test['path'] = '/build/release/deps/lifecycle-0123'
    source = {'commit': 'a' * 40, 'tree': 'b' * 40}
    raw = {'synthetic': 'no actual Cargo execution or source authentication'}
    plan = {'format': 'zryna.linux-lifecycle-owner-plan.v1', 'row': 'zryna-playground-compiler/lifecycle/' + INPUT.NAME,
        'source': source, 'prebuild': {'raw': raw, 'artifacts': {'compiler': compiler, 'test': test}},
        'modes': INPUT.modes(), 'environment': dict(entry.split('=', 1) for entry in INPUT.ENV),
        'runtime': {'sha256': INPUT.sha256(INPUT.canonical(runtime)), 'files': len(runtime['files']), 'bytes': sum(row['bytes'] for row in runtime['files'])},
        'materials': {'files': copy.deepcopy(INPUT.MATERIALS), 'bytes': 133968848},
        'parent': INPUT.parent_profile([0, 1]),
        **{key: False for key in ['sourceAuthenticated', 'ownerExecuted', 'materialsApproved', 'runtimeApproved', 'hostApproved', 'executionApproved', 'proofQualified', 'performanceQualified']},
        'baselineAuthority': None, 'thresholds': None}
    context = b'{"synthetic":true}\n'
    return {'plan': plan, 'planSha256': INPUT.sha256(INPUT.canonical(plan)), 'runtime': runtime,
        'capture': copy.deepcopy({'source': source, 'raw': raw, 'compiler': compiler, 'test': test}),
        'image': {'id': 'sha256:' + '6' * 64, 'layerSha256': '7' * 64},
        'materials': {'name': 'zryna410-materials-compiler-' + '8' * 32, 'nonce': '8' * 32, 'createdAt': '2026-10-28T00:00:00Z',
                      'manifestSha256': next(row['sha256'] for row in runtime['files'] if row['path'] == '/resources/materials-manifest.json'),
                      'manifestBytes': next(row['bytes'] for row in runtime['files'] if row['path'] == '/resources/materials-manifest.json'), 'files': 10, 'bytes': 133968848},
        'engine': {'id': 'synthetic-engine', 'version': '28.4.0', 'apiVersion': '1.49', 'cgroupDriver': 'systemd', 'dockerRootDir': '/var/lib/docker'},
        'nativeVm': {'raw': context, 'sha256': INPUT.sha256(context)}}


class FakeEngine:
    def __init__(self, backend, deadline, record):
        self.backend, self.deadline, self.record = backend, deadline, record

    def json(self, method, path, value=None):
        self.backend.events.append((method, path, self.deadline))
        if path.startswith('/containers/create?name='):
            if self.backend.fail == 'create-unknown':
                raise TimeoutError('synthetic unanswered creation')
            self.backend.created += 1
            cid = str(self.backend.created) * 64
            material = self.backend.selected['materials']; root = '/var/lib/docker/volumes/' + material['name'] + '/_data'
            self.backend.objects[cid] = {'Id': cid, 'Name': '/' + path.split('=', 1)[1], 'Image': value['Image'],
                'Config': {key: copy.deepcopy(field) for key, field in value.items() if key != 'HostConfig'},
                'HostConfig': copy.deepcopy(value['HostConfig']), 'Mounts': [{'Type': 'volume', 'Name': material['name'], 'Driver': 'local', 'RW': False, 'Source': root, 'Destination': '/materials'}],
                'RestartCount': 0, 'State': {'Status': 'created', 'Running': False, 'Pid': 0, 'Paused': False, 'Restarting': False,
                    'OOMKilled': False, 'Error': '', 'StartedAt': '0001-01-01T00:00:00Z', 'FinishedAt': '0001-01-01T00:00:00Z', 'ExitCode': 0}}
            return {'Id': cid, 'Warnings': []}
        if path.endswith('/json'):
            return copy.deepcopy(self.backend.objects[path.split('/')[2]])
        if '/wait?' in path:
            return {'StatusCode': 137, 'Error': None}
        raise AssertionError('unrecognized synthetic JSON operation: ' + path)

    def request(self, method, path, **unused):
        self.backend.events.append((method, path, self.deadline))
        cid = path.split('/')[2].split('?')[0]
        if path.endswith('/start'):
            self.backend.objects[cid]['State'].update(Status='running', Running=True, Pid=123, StartedAt='2026-10-28T01:00:00Z')
        elif '/kill?' in path:
            self.backend.objects[cid]['State'].update(Status='exited', Running=False, Pid=0, ExitCode=137)
        elif method == 'DELETE':
            if self.backend.fail == 'remove':
                raise TimeoutError('synthetic uncertain removal')
            del self.backend.objects[cid]
        else:
            raise AssertionError('unrecognized synthetic request: ' + path)
        return b'', None


class FakeKernel:
    def __init__(self, backend):
        self.backend = backend

    def gone(self, init, deadline):
        if self.backend.fail == 'gone':
            raise ValueError('synthetic original init/cgroup not absent')
        return {'originalInitAbsent': True, 'cgroup': 'removed', 'pidReused': False}


class FakeBackend:
    synthetic = True

    def __init__(self, value, fail=None):
        self.selected, self.fail, self.objects, self.events, self.apps = value, fail, {}, [], []
        self.created = 0
        self.seccomp = (Path(__file__).parent.parent / 'cli-parity' / 'compiler-seccomp.json').read_text()
        self.instant = 100.0

    def clock(self):
        return self.instant

    def engine(self, deadline, record):
        return FakeEngine(self, deadline, record)

    def kernel(self, record):
        return FakeKernel(self)

    def audit(self, engine, record):
        self.events.append(('audit', None, engine.deadline))
        if self.fail in ['peer', 'image', 'writer']:
            raise ValueError('synthetic actual ' + self.fail + ' rejected')
        record['synthetic'] = True

    def observe(self, engine, kernel, cid, inspect, deadline):
        inspect(copy.deepcopy(self.objects[cid]))
        if self.fail in ['namespace', 'ancestor', 'zombie']:
            raise ValueError('synthetic actual ' + self.fail + ' rejected')
        if self.fail == 'deadline':
            self.instant = deadline + 0.001
            raise TimeoutError('synthetic original deadline exhausted')
        return {'pid': 123, 'ticks': '111', 'cid': cid, 'startedAt': '2026-10-28T01:00:00Z',
                'membership': '0::/system.slice/docker-' + cid + '.scope\n', 'cgroupPath': '/sys/fs/cgroup/system.slice/docker-' + cid + '.scope', 'quiescent': True}

    def execute(self, engine, cid, mode, step, record):
        self.apps.append((mode['id'], step, EXECUTE.command(self.selected, mode, step)))
        if self.fail == 'app' and step == 'test':
            raise ValueError('synthetic failed test with original streams retained')
        record['synthetic'] = True

    def lookup(self, engine, selector):
        self.events.append(('lookup', selector, engine.deadline))
        value = self.objects.get(selector)
        if value is None:
            value = next((row for row in self.objects.values() if row['Name'] == '/' + selector), None)
        return copy.deepcopy(value)
