"""One creation-owned fixed operation; unknown teardown is sticky and never accepted."""
import re
import secrets
import time
from urllib.parse import urlencode
from cli_common import require, sha256
from cli_engine import Engine, decode
from cli_config import approval, configuration
from cli_plan import operation
from cli_exec import run as execute
from cli_stage import run as project_stage
import cli_identity as identity
import cli_kernel as kernel


def never_started(state):
    value = state['State']
    return (value['Status'] == 'created' and value['Pid'] == 0 and value['Running'] is False and
            value['StartedAt'] == '0001-01-01T00:00:00Z' and value['FinishedAt'] == '0001-01-01T00:00:00Z')


class Owner:
    def __init__(self, selected, identifier, engine=None, cancel=None):
        self.selected, self.operation = approval(selected), operation(identifier)
        self.engine = engine if engine is not None else Engine(65534, seconds=10, cancel=cancel)
        require(self.engine.engine_uid == 65534 and
                time.monotonic() < self.engine.deadline <= time.monotonic() + 10, 'whole fixed peer/deadline')
        self.nonce = secrets.token_hex(16)
        self.name, self.cid, self.init = 'zryna410-cli-' + self.nonce, None, None
        self.expected = configuration(self.selected, self.nonce)
        self.attempted = self.stopped = self.admitted = False
        self.record = {'format': 'zryna.playground-cli-owner-observation.v1', 'version': 1,
            'qualificationOnly': True, 'operationId': identifier, 'name': self.name, 'nonce': self.nonce,
            'containerId': None, 'inspect': [], 'kernel': [], 'exec': {}, 'wait': None,
            'ownedVolumes': [], 'removed': False, 'cleanupConfirmed': False, 'ownershipUnconfirmed': False,
            'failure': None, 'cleanupFailure': None, 'operationStatus': 'incomplete'}

    def inspect(self, engine=None, cleanup=False):
        state = (engine or self.engine).json('GET', '/containers/' + self.cid + '/json')
        identity.container(state, self.cid, self.name, self.expected, cleanup=cleanup)
        self.record['inspectCount'] = self.record.get('inspectCount', 0) + 1
        if len(self.record['inspect']) < 32:
            self.record['inspect'].append(state)
        else:
            self.record['inspect'][-1] = state
        return state

    def observe(self, retry=False):
        while True:
            try:
                value = kernel.capture(self.inspect(), self.cid, self.engine.deadline,
                                       cancel=getattr(self.engine, 'cancel', None))
                break
            except kernel.ReapPending:
                require(retry and time.monotonic() < self.engine.deadline, 'bounded post-exec actual child reap')
                time.sleep(min(0.005, max(0, self.engine.deadline - time.monotonic())))
        if self.init is not None:
            require(all(value[key] == self.init[key] for key in ['pid', 'ticks', 'startedAt']), 'same original init')
        self.init = value
        self.record['kernel'].append(value)
        return value

    def retain_init(self, state, engine):
        pid = state['State']['Pid']
        require(type(pid) is int and pid > 1 and state['State']['Running'] is True, 'owned original init identity')
        value = {'pid': pid, 'cid': self.cid, 'ticks': kernel.ticks(pid, engine.deadline),
                 'membership': kernel.read(f'/proc/{pid}/cgroup', engine.deadline),
                 'startedAt': state['State']['StartedAt']}
        kernel.group_path(value['membership'], self.cid, engine.deadline)
        if self.init is not None:
            require(all(value[key] == self.init[key] for key in ['pid', 'ticks', 'startedAt']), 'same retained init')
        self.init = value

    def run(self):
        require(not self.attempted and not self.stopped, 'one fixed operation per owner')
        phase = 'image'
        try:
            image = self.engine.json('GET', '/images/' + self.selected['image'] + '/json')
            identity.image(image, self.selected)
            self.record['imageBefore'] = image
            phase, self.attempted = 'create', True
            created = self.engine.json('POST', '/containers/create?' + urlencode({'name': self.name}), self.expected)
            require(type(created.get('Id')) is str and re.fullmatch('[a-f0-9]{64}', created['Id']), 'full created ID')
            self.cid = self.record['containerId'] = created['Id']
            state = self.inspect()
            require(never_started(state) and state['State']['ExitCode'] == 0,
                    'zero-timestamp never-started original creation')
            phase = 'start'
            self.engine.request('POST', '/containers/' + self.cid + '/start', statuses=(204,))
            phase = 'before-kernel'
            self.retain_init(self.inspect(), self.engine)
            self.observe()
            phase = 'fixed-source-upload'
            self.record['sourceUpload'] = {}
            staged = project_stage(self.engine, self.cid, self.operation['id'], self.record['sourceUpload'])
            self.record['projectBeforeUpload'] = staged['before']
            self.record['projectAfterUpload'] = staged['after']
            phase = 'before-exec-kernel'
            self.observe(retry=True)
            phase = 'fixed-cli-exec'
            execute(self.engine, self.cid, self.operation['id'], self.record['exec'])
            phase = 'after-exec-kernel'
            self.observe(retry=True)
            image = self.engine.json('GET', '/images/' + self.selected['image'] + '/json')
            identity.image(image, self.selected)
            self.record['imageAfterExec'] = image
            self.admitted = True
        except BaseException as error:
            self.record['failure'] = {'phase': phase, 'type': type(error).__name__[:64]}
        finally:
            self.stop()
        if self.admitted and self.record['cleanupConfirmed']:
            self.record['operationStatus'] = 'captured-source-outcome'
        return self.record

    def stop(self):
        if self.stopped:
            return self.record
        self.stopped = True
        self.cleanup_deadline = time.monotonic() + 2
        if not self.attempted:
            self.record.update(removed=True, cleanupConfirmed=True)
            return self.record
        cleanup = Engine(65534, seconds=2)
        cleanup.deadline = min(cleanup.deadline, self.cleanup_deadline)
        try:
            if self.cid is None:
                raw, _ = cleanup.request('GET', '/containers/' + self.name + '/json', statuses=(200, 404))
                state = decode(raw)
                if cleanup.last_status == 404:
                    self.record['ambiguousName404'] = state
                    self.record.update(removed=True, cleanupConfirmed=True)
                    return self.record
                require(type(state.get('Id')) is str and re.fullmatch('[a-f0-9]{64}', state['Id']), 'ambiguous create ID')
                identity.container(state, state['Id'], self.name, self.expected, cleanup=True)
                self.cid = self.record['containerId'] = state['Id']
            state = self.inspect(cleanup, cleanup=True)
            created_only = never_started(state)
            if state['State']['Running'] and self.init is None:
                self.retain_init(state, cleanup)
            if state['State']['Running']:
                cleanup.request('POST', '/containers/' + self.cid + '/kill?signal=SIGKILL', statuses=(204,))
            if not created_only:
                waited = decode(cleanup.request('POST', '/containers/' + self.cid +
                    '/wait?condition=not-running', statuses=(200,))[0])
                require(type(waited.get('StatusCode')) is int and not waited.get('Error'), 'actual wait receipt')
                self.record['wait'] = waited
                state = self.inspect(cleanup, cleanup=True)
                require(state['State']['Running'] is False and state['State']['Pid'] == 0 and
                        state['State']['Status'] == 'exited' and state['State']['ExitCode'] == waited['StatusCode'],
                        'actual namespace init stopped before removal')
            cleanup.request('DELETE', '/containers/' + self.cid + '?force=0&v=0', statuses=(204,))
            raw, _ = cleanup.request('GET', '/containers/' + self.cid + '/json', statuses=(404,))
            self.record['removal404'] = decode(raw)
            if self.init is not None:
                self.record['reap'] = kernel.gone(self.init, cleanup.deadline)
            else:
                require(created_only, 'started init without retained process identity remains unknown')
            image = cleanup.json('GET', '/images/' + self.selected['image'] + '/json')
            identity.image(image, self.selected)
            self.record['imageAfterCleanup'] = image
            self.record.update(removed=True, cleanupConfirmed=True)
        except BaseException as error:
            self.record.update(cleanupFailure=type(error).__name__[:64], ownershipUnconfirmed=True)
        return self.record
