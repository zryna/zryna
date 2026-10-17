"""Fixed trusted operator entrypoint: one bounded approval/opID, EOF cancellation, bounded receipt."""
import json
import os
import select
import signal
import stat
import sys
import time

# This is the independently reviewed operator source directory, never an uploaded package path.
ROOT = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, ROOT)
from cli_common import exact, require
from cli_engine import decode
from cli_owner import Owner


def cancelled():
    if select.select([3], [], [], 0)[0]:
        raise InterruptedError('CLI owner control closed or cancelled')


def main():
    mode = os.fstat(3).st_mode
    require(len(sys.argv) == 1 and (stat.S_ISFIFO(mode) or stat.S_ISSOCK(mode)),
            'fixed owner invocation and creation-owned control pipe/socketpair')
    os.closerange(4, 65536)
    deadline, raw = time.monotonic() + 2, bytearray()
    while not raw.endswith(b'\n'):
        cancelled()
        require(time.monotonic() < deadline and len(raw) < 16384, 'bounded initial approval')
        if select.select([0], [], [], min(0.05, max(0, deadline - time.monotonic())))[0]:
            chunk = os.read(0, 1)
            require(chunk, 'initial approval EOF')
            raw.extend(chunk)
    value = decode(bytes(raw))
    exact(value, ['approval', 'operationId'])
    owner = Owner(value['approval'], value['operationId'], cancel=cancelled)
    record = owner.run()
    data = json.dumps(record, separators=(',', ':'), ensure_ascii=True).encode() + b'\n'
    require(len(data) <= 1048576, 'bounded owner receipt')
    os.set_blocking(1, False)
    while data:
        require(time.monotonic() < owner.cleanup_deadline, 'shared cleanup receipt deadline')
        if select.select([], [1], [], min(0.05, max(0, owner.cleanup_deadline - time.monotonic())))[1]:
            count = os.write(1, data)
            require(count > 0, 'owner receipt short write')
            data = data[count:]
    return 0 if record['operationStatus'] == 'captured-source-outcome' and record['cleanupConfirmed'] else 70


if __name__ == '__main__':
    try:
        # Signals interrupt active sockets through the same run/finally cleanup path.
        signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(InterruptedError('operator termination')))
        sys.exit(main())
    except BaseException as error:
        if isinstance(error, SystemExit):
            raise
        os.write(2, b'PLAYGROUND-CLI-OWNER-FAILED\n')
        sys.exit(70)
