"""Ephemeral bounded duplex forwarding only; no raw channel byte is persisted."""
import os
import selectors
import time
from plan import require, exact
from engine import decode

MAXIMUM = 67108864


def stop_deadline(raw):
    value = decode(raw)
    exact(value, ['type', 'deadlineNs'])
    require(value['type'] == 'stop' and type(value['deadlineNs']) is str and
            value['deadlineNs'].isascii() and value['deadlineNs'].isdigit() and
            1 <= len(value['deadlineNs']) <= 20, 'UI closed internal stop clock')
    deadline = int(value['deadlineNs'])
    require(0 < deadline <= time.monotonic_ns() + 2000000000, 'UI fixed2second shared cleanup limit')
    return deadline / 1000000000


def run(stream, deadline, cleanup=None):
    selector = selectors.DefaultSelector()
    for fd in [0, 1, 3, stream.fileno()]:
        os.set_blocking(fd, False)
    incoming, outgoing = bytearray(), bytearray()
    total_in, total_out, control = 0, 0, bytearray()
    try:
        while time.monotonic() < deadline:
            selector.close()
            selector = selectors.DefaultSelector()
            selector.register(3, selectors.EVENT_READ, 'control')
            if len(incoming) < 4194304:
                selector.register(0, selectors.EVENT_READ, 'input')
            events = (selectors.EVENT_READ if len(outgoing) < 4194304 else 0) | (selectors.EVENT_WRITE if incoming else 0)
            if events:
                selector.register(stream, events, 'socket')
            if outgoing:
                selector.register(1, selectors.EVENT_WRITE, 'output')
            for key, mask in selector.select(min(0.1, deadline - time.monotonic())):
                try:
                    terminal, total_in, total_out = transfer(key, mask, stream, incoming, outgoing,
                        control, total_in, total_out, deadline, cleanup)
                    if terminal:
                        return terminal
                except BlockingIOError:
                    # Readiness can race a peer close or another readiness edge.
                    continue
        raise TimeoutError('UI300second lifetime')
    finally:
        selector.close()


def transfer(key, mask, stream, incoming, outgoing, control, total_in, total_out, deadline, cleanup=None):
    if key.data == 'control':
        data = os.read(3, 1024)
        if not data:
            return 'control-eof', total_in, total_out
        control.extend(data)
        require(len(control) <= 1024, 'UI finite owner control')
        if b'\n' in control:
            require(control.count(b'\n') == 1 and control.endswith(b'\n'), 'UI single closed stop record')
            selected = stop_deadline(bytes(control))
            if cleanup is not None:
                cleanup['deadline'] = selected
            return 'stop', total_in, total_out
    elif key.data == 'input':
        data = os.read(0, 65536)
        if not data:
            return 'stdin-eof', total_in, total_out
        total_in += len(data)
        require(total_in <= MAXIMUM, 'UI inbound aggregate')
        incoming.extend(data)
    elif key.data == 'socket':
        if mask & selectors.EVENT_WRITE:
            count = os.write(stream.fileno(), incoming)
            require(count > 0, 'UI attached write')
            del incoming[:count]
        if mask & selectors.EVENT_READ:
            data = os.read(stream.fileno(), 65536)
            if not data:
                while outgoing:
                    require(time.monotonic() < deadline, 'UI final output bounded flush')
                    import select
                    if select.select([], [1], [], min(0.1, deadline - time.monotonic()))[1]:
                        try:
                            count = os.write(1, outgoing)
                            require(count > 0, 'UI final output progress')
                            del outgoing[:count]
                        except BlockingIOError:
                            continue
                return 'engine-eof', total_in, total_out
            total_out += len(data)
            require(total_out <= MAXIMUM, 'UI outbound aggregate')
            outgoing.extend(data)
    elif key.data == 'output':
        count = os.write(1, outgoing)
        require(count > 0, 'UI bounded outer output')
        del outgoing[:count]
    return None, total_in, total_out
