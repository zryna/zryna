"""Creation-owned UI lifecycle; ready means full audit passed, never browser/result acceptance."""
import re
import secrets
import time
import json
from urllib.parse import urlencode
from engine import Engine, decode
from plan import require
from owner_config import approval, configuration
import owner_identity as identity
import owner_kernel as kernel
from owner_attach import attach
from owner_audit import run as audit, run_proc, proc_description


class Owner:
    def __init__(self, selected, engine=None):
        self.selected = approval(selected)
        self.engine = engine if engine is not None else Engine(65534, seconds=300)
        require(self.engine.engine_uid == 65534 and
                time.monotonic() < self.engine.deadline <= time.monotonic() + 300, 'UI fixed peer/deadline')
        self.nonce = secrets.token_hex(16)
        self.name = 'zryna410-ui-' + self.nonce
        self.expected = configuration(self.selected, self.nonce)
        self.cid, self.init, self.stream = None, None, None
        self.attempted, self.ready, self.stopped, self.start_attempted = False, False, False, False
        self.record = {'format': 'zryna.playground-ui-owner-observation.v1', 'version': 1,
                       'qualificationOnly': True, 'name': self.name, 'nonce': self.nonce,
                       'containerId': None, 'audit': None, 'kernel': [], 'inspect': [], 'wait': None,
                       'removed': False, 'dockerRemoved': False, 'kernelReapProven': False,
                       'failure': None, 'cleanupFailure': None, 'ownershipUnconfirmed': False, 'acquisition': []}

    def inspect(self, engine=None):
        state = (engine or self.engine).json('GET', '/containers/' + self.cid + '/json')
        identity.container(state, self.cid, self.name, self.expected, cleanup=engine is not None)
        if len(self.record['inspect']) < 32:
            self.record['inspect'].append(state)
        else:
            self.record['inspect'][-1] = state
        return state

    def volume(self, engine=None):
        selected_engine = engine or self.engine
        pin = self.selected['material']
        actual = selected_engine.json('GET', '/volumes/' + pin['name'])
        identity.volume(actual, pin)
        query = urlencode({'all': '1', 'filters': json.dumps({'volume': [pin['name']]}, separators=(',', ':'))})
        aliases = selected_engine.json('GET', '/containers/json?' + query)
        require(type(aliases) is list and len(aliases) <= 1, 'UI material alias absence')
        for alias in aliases:
            require(self.cid is not None and alias['Id'] == self.cid, 'UI only original owner material mount')
            self.inspect(selected_engine)
        return actual

    def capture_phase(self, proc_proof, phase, anchors=None):
        require(not self.ready and phase in ('POST_PROC_AUDIT', 'POST_MATERIAL_AUDIT'),
                'kernel acquisition only before private capability')
        previous = self.engine.deadline
        limit = min(previous, time.monotonic() + 1)
        self.engine.deadline = limit
        try:
            record = {}
            self.record['acquisition'].append(record)
            return kernel.acquire(self.inspect, self.cid, limit, proc_proof, self.record['initialIdentity'], phase, record, anchors)
        finally:
            # Restore the same absolute owner deadline, never a newly computed allowance.
            self.engine.deadline = previous

    def start(self):
        require(not self.attempted and not self.stopped, 'UI owner one admission')
        image = self.engine.json('GET', '/images/' + self.selected['image'] + '/json')
        identity.image(image, self.selected)
        self.record['imageBefore'] = image
        self.record['volumeBefore'] = self.volume()
        self.attempted = True
        created = self.engine.json('POST', '/containers/create?' + urlencode({'name': self.name}), self.expected)
        require(type(created.get('Id')) is str and re.fullmatch('[a-f0-9]{64}', created['Id']), 'UI full actual created ID')
        self.cid = created['Id']
        self.record['containerId'] = self.cid
        initial = self.inspect()
        require(initial['State']['Status'] == 'created' and initial['State']['Pid'] == 0 and
                initial['State']['Running'] is False and
                initial['State']['StartedAt'] == initial['State']['FinishedAt'] == '0001-01-01T00:00:00Z',
                'UI never-started ownership')
        # The duplex channel exists before start. No start/capability bytes have been sent.
        self.stream = attach(self.engine.deadline, self.cid)
        self.start_attempted = True
        self.engine.request('POST', '/containers/' + self.cid + '/start', statuses=(204,))
        state = self.inspect()
        self.init = kernel.initial(state, self.cid, self.engine.deadline)
        self.record['initialIdentity'] = self.init
        proc_proof = run_proc(self.engine, self.cid)
        self.record['procAudit'] = proc_description(proc_proof, self.cid)
        audited = self.capture_phase(proc_proof, 'POST_PROC_AUDIT')
        require(all(audited[key] == self.init[key] for key in ['pid', 'ticks', 'cid', 'membership', 'startedAt']),
                'UI original init unchanged across native proc audit')
        self.init = audited
        self.record['kernel'].append(self.init)
        self.record['audit'] = audit(self.engine, self.cid, self.selected['material'])
        self.record['volumeAfterAudit'] = self.volume()
        self.record['kernel'].append(self.capture_phase(proc_proof, 'POST_MATERIAL_AUDIT',
            self.record['kernel'][0]['processes']['processes']))
        require(self.record['kernel'][0]['pid'] == self.record['kernel'][1]['pid'] and
                self.record['kernel'][0]['ticks'] == self.record['kernel'][1]['ticks'], 'UI stable audited init')
        self.ready = True
        return self.stream

    def stop(self, deadline=None):
        if self.stopped:
            return self.record
        self.stopped = True
        self.cleanup_deadline = min(time.monotonic() + 2, self.engine.deadline + 2)
        if deadline is not None:
            require(type(deadline) in [float, int] and deadline <= time.monotonic() + 2,
                    'UI internal shared monotonic cleanup deadline')
            self.cleanup_deadline = min(self.cleanup_deadline, deadline)
        if self.stream is not None:
            try:
                self.stream.close()
            except BaseException:
                self.record['failure'] = self.record['failure'] or 'UI attached channel close failed'
        if not self.attempted:
            self.record['reap'] = {'noCreateAttempt': True}
            self.record['removed'] = True
            return self.record
        cleanup = Engine(65534, seconds=2)
        cleanup.deadline = min(cleanup.deadline, self.cleanup_deadline)
        try:
            if self.cid is None:
                # A possibly accepted create cannot be forgotten just because its response was lost.
                raw, _ = cleanup.request('GET', '/containers/' + self.name + '/json', statuses=(200, 404))
                state = decode(raw)
                require(type(state.get('Id')) is str and re.fullmatch('[a-f0-9]{64}', state['Id']),
                        'UI ambiguous create ownership retained for recovery')
                identity.container(state, state['Id'], self.name, self.expected, cleanup=True)
                self.cid = state['Id']
                self.record['containerId'] = self.cid
            state = self.inspect(cleanup)
            if self.init is None and state['State']['Running']:
                try:
                    self.init = kernel.initial(state, self.cid, cleanup.deadline)
                    self.record['initialIdentity'] = self.init
                except BaseException as error:
                    self.record['kernelIdentityFailure'] = type(error).__name__
            grace = min(cleanup.deadline - 1, time.monotonic() + 0.25)
            while state['State']['Running'] and time.monotonic() < grace:
                time.sleep(0.005)
                state = self.inspect(cleanup)
            created_only = state['State']['Status'] == 'created' and state['State']['Pid'] == 0
            if state['State']['Running']:
                cleanup.request('POST', '/containers/' + self.cid + '/kill?signal=SIGKILL', statuses=(204,))
            if not created_only:
                raw, _ = cleanup.request('POST', '/containers/' + self.cid + '/wait?condition=not-running', statuses=(200,))
                waited = decode(raw)
                require(type(waited.get('StatusCode')) is int and not waited.get('Error'), 'UI actual wait receipt')
                self.record['wait'] = waited
                state = self.inspect(cleanup)
                require(state['State']['Running'] is False and state['State']['Pid'] == 0 and
                        state['State']['Status'] == 'exited' and state['State']['ExitCode'] == waited['StatusCode'],
                        'UI actual stopped init before removal')
            cleanup.request('DELETE', '/containers/' + self.cid + '?force=0&v=0', statuses=(204,))
            raw, _ = cleanup.request('GET', '/containers/' + self.cid + '/json', statuses=(404,))
            require(type(decode(raw)) is dict, 'UI removal readback')
            self.record['dockerRemoved'] = True
            if self.init is not None:
                self.record['reap'] = kernel.gone(self.init, cleanup.deadline)
                self.record['kernelReapProven'] = True
            else:
                require(created_only and not self.start_attempted and
                        state['State']['StartedAt'] == state['State']['FinishedAt'] == '0001-01-01T00:00:00Z',
                        'Docker removal does not establish missing kernel reap identity')
                self.record['reap'] = {'notStarted': True}
            self.record['volumeAfterCleanup'] = self.volume(cleanup)
            image = cleanup.json('GET', '/images/' + self.selected['image'] + '/json')
            identity.image(image, self.selected)
            self.record['imageAfter'] = image
            self.record['removed'] = True
        except BaseException as error:
            self.record['cleanupFailure'] = type(error).__name__
            self.record['ownershipUnconfirmed'] = True
        return self.record
