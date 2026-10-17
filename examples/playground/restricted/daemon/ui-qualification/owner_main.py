"""Fixed isolated interpreter entrypoint. No executable/URL/socket/path command selectors."""
import os
import select
import sys
import time

ROOT = os.path.dirname(os.path.abspath(__file__))
STDLIB = os.path.join(ROOT, 'lib', 'python3.12')
# Require the captured interpreter's private prefix; never fall back to ambient modules.
if sys.prefix != ROOT or sys.exec_prefix != ROOT:
    raise ValueError('UI private Python prefix required')
sys.path[:] = [ROOT, STDLIB, os.path.join(STDLIB, 'lib-dynload')]
from engine import decode
from plan import require, exact
from owner_lifecycle import Owner
from owner_relay import run, stop_deadline
import json


def control(value, deadline):
    raw = json.dumps(value, separators=(',', ':')).encode() + b'\n'
    require(len(raw) <= 2097152, 'UI bounded sanitized owner receipt')
    os.set_blocking(3, False)
    while raw:
        require(time.monotonic() < deadline, 'UI owner control deadline')
        ready = select.select([], [3], [], min(0.1, deadline - time.monotonic()))[1]
        if ready:
            count = os.write(3, raw)
            require(count > 0, 'UI owner control write')
            raw = raw[count:]


def main():
    deadline = time.monotonic() + 300
    raw = bytearray()
    while not raw.endswith(b'\n'):
        require(time.monotonic() < deadline and len(raw) <= 262144, 'UI bounded initial approval')
        if not select.select([0, 3], [], [], min(0.1, deadline - time.monotonic()))[0]:
            continue
        if select.select([3], [], [], 0)[0]:
            raise ValueError('UI initial control interrupted')
        data = os.read(0, 1)
        require(data, 'UI approval EOF')
        raw.extend(data)
    initial = decode(bytes(raw))
    exact(initial, ['version', 'approval', 'deadlineNs'])
    require(type(initial['version']) is int and initial['version'] == 1 and
            type(initial['deadlineNs']) is str and initial['deadlineNs'].isascii() and
            initial['deadlineNs'].isdigit() and 1 <= len(initial['deadlineNs']) <= 20,
            'UI internal monotonic startup envelope')
    selected_deadline = int(initial['deadlineNs'])
    require(time.monotonic_ns() < selected_deadline <= time.monotonic_ns() + 300000000000,
            'UI shared finite300second limit')
    deadline = min(deadline, selected_deadline / 1000000000)
    owner = Owner(initial['approval'])
    owner.engine.deadline = min(owner.engine.deadline, deadline)
    cleanup = {}
    try:
        # Parent learns exact recovery name before the first Engine mutation.
        control({'type': 'reserved', 'nonce': owner.nonce, 'name': owner.name}, deadline)
        stream = owner.start()
        control({'type': 'ready', 'nonce': owner.nonce, 'containerId': owner.cid,
                 'audit': owner.record['audit']['receipt']}, deadline)
        owner.record['channelTerminal'] = run(stream, deadline, cleanup)
    except BaseException as error:
        owner.record['failure'] = type(error).__name__
    finally:
        try:
            if 'deadline' not in cleanup and select.select([3], [], [], 0)[0]:
                pending = os.read(3, 1024)
                if pending:
                    cleanup['deadline'] = stop_deadline(pending)
        except BaseException:
            owner.record['failure'] = owner.record['failure'] or 'UI invalid cleanup control'
        receipt = owner.stop(cleanup.get('deadline'))
        try:
            control({'type': 'closed', 'receipt': receipt}, owner.cleanup_deadline)
        except BaseException:
            pass
    return 0 if receipt['removed'] and not receipt['cleanupFailure'] else 70


if __name__ == '__main__':
    try:
        sys.exit(main())
    except BaseException as error:
        if isinstance(error, SystemExit):
            raise
        # No raw exception or stdin/stdio values enter stderr or evidence.
        os.write(2, b'PLAYGROUND-UI-OWNER-FAILED\n')
        sys.exit(70)
