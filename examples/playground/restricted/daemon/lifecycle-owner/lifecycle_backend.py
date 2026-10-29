"""Fixed local Engine and kernel operations; no application/argv selection surface."""
import os
from pathlib import Path
import sys
import time
from cli_common import require
from cli_engine import decode
from lifecycle_engine import Engine
from lifecycle_execute import run as execute
from lifecycle_inputs import image_identity, volume_identity
from preparation_volume import no_writers
from lifecycle_kernel import Kernel, Pending


class Backend:
    synthetic = False

    def __init__(self, selected):
        require(sys.platform == 'linux' and os.getuid() == os.geteuid() == 0, 'native Linux UID0 observer required')
        self.selected, self.anchor = selected, []
        fixed = Path(__file__).resolve().parent.parent / 'cli-parity' / 'compiler-seccomp.json'
        with fixed.open('rb') as stream:
            raw = stream.read(5460)
        self.seccomp = raw.decode('ascii')  # configuration() checks exact bytes and reviewed SHA.

    def clock(self):
        return time.monotonic()

    def pause(self, deadline):
        require(self.clock() < deadline, 'original deadline cannot renew')
        time.sleep(min(0.005, deadline - self.clock()))

    def engine(self, deadline, record):
        return Engine(self.selected['engine'], deadline, record, self.anchor)

    def kernel(self, record):
        return Kernel(self.selected, record)

    def audit(self, engine, record):
        record['engine'] = engine.audit()
        record['image'] = engine.json('GET', '/images/' + self.selected['image']['id'] + '/json')
        image_identity(record['image'], self.selected)
        record['volume'] = engine.json('GET', '/volumes/' + self.selected['materials']['name'])
        volume_identity(record['volume'], self.selected)
        self.writers(engine, record)

    def writers(self, engine, record):
        material = self.selected['materials']
        root = self.selected['engine']['dockerRootDir'] + '/volumes/' + material['name'] + '/_data'
        no_writers(engine, material['name'], root, record)

    def observe(self, engine, kernel, cid, inspect, deadline):
        # Only quiescence/reaping may wait; no application or creation is retried.
        for _ in range(64):
            require(self.clock() < deadline, 'original whole phase deadline')
            state = engine.json('GET', '/containers/' + cid + '/json')
            inspect(state)
            try:
                return kernel.capture(state, cid, deadline)
            except Pending:
                self.pause(deadline)
        raise TimeoutError('bounded quiescence observations exhausted')

    def execute(self, engine, cid, mode, step, record):
        execute(engine, cid, self.selected, mode, step, record)

    def lookup(self, engine, selector):
        raw, _ = engine.request('GET', '/containers/' + selector + '/json', statuses=(200, 404))
        value = decode(raw)
        require(type(value) is dict, 'actual inspect response object')
        if engine.last_status == 404:
            require(set(value) == {'message'} and type(value['message']) is str and 0 < len(value['message']) <= 4096,
                    'complete actual not-found response')
            return None
        return value
