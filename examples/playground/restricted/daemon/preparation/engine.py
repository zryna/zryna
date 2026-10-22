"""Private operator preparation over one fixed, peer-authenticated local Engine socket."""
import http.client
import json
import os
import socket
import stat
import struct
import time
from plan import integer, require

SOCKET = '/var/run/docker.sock'
MAX_RESPONSE = 262144


class TimedSocket(socket.socket):
    def __init__(self, deadline):
        super().__init__(socket.AF_UNIX, socket.SOCK_STREAM | socket.SOCK_CLOEXEC)
        self.deadline = deadline

    def guard(self):
        remaining = self.deadline - time.monotonic()
        if remaining <= 0:
            raise TimeoutError('whole operator preparation deadline exceeded')
        self.settimeout(remaining)

    def send(self, data, flags=0):
        self.guard()
        result = super().send(data, flags)
        self.guard()
        return result

    def sendall(self, data, flags=0):
        view = memoryview(data)
        while view:
            count = self.send(view, flags)
            require(count > 0, 'preparation socket short write')
            view = view[count:]

    def recv_into(self, buffer, nbytes=0, flags=0):
        self.guard()
        count = super().recv_into(buffer, nbytes, flags)
        self.guard()
        return count


class Connection(http.client.HTTPConnection):
    def __init__(self, deadline, engine_uid):
        super().__init__('localhost')
        self.deadline, self.engine_uid = deadline, engine_uid

    def connect(self):
        for parent in ['/', '/var', '/var/run']:
            value = os.stat(parent)
            require(stat.S_ISDIR(value.st_mode) and value.st_uid in [0, self.engine_uid]
                    and not value.st_mode & 0o022, 'trusted fixed socket ancestry required')
        before = os.lstat(SOCKET)
        require(stat.S_ISSOCK(before.st_mode) and before.st_uid == self.engine_uid
                and not before.st_mode & 0o002, 'fixed socket identity required')
        stream = TimedSocket(self.deadline)
        try:
            stream.guard()
            stream.connect(SOCKET)
            stream.guard()
            after = os.lstat(SOCKET)
            fields = ['st_dev', 'st_ino', 'st_uid', 'st_gid', 'st_mode', 'st_ctime_ns']
            require(all(getattr(before, field) == getattr(after, field) for field in fields),
                    'fixed socket changed')
            _, peer_uid, _ = struct.unpack('3i', stream.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
            require(peer_uid == self.engine_uid and stream.getpeername() == SOCKET,
                    'fixed local daemon peer differs')
            self.sock = stream
        except BaseException:
            stream.close()
            raise


class Chunked:
    def __init__(self, connection, maximum):
        self.connection, self.maximum, self.bytes = connection, maximum, 0

    def write(self, data):
        require(0 < len(data) <= 65536 and self.bytes + len(data) <= self.maximum,
                'finite archive transport required')
        self.connection.send(f'{len(data):x}\r\n'.encode('ascii'))
        self.connection.send(data)
        self.connection.send(b'\r\n')
        self.bytes += len(data)
        return len(data)

    def flush(self):
        return None


def decode(raw):
    def pairs(items):
        output = {}
        for key, value in items:
            require(key not in output, 'duplicate daemon JSON key')
            output[key] = value
        return output
    return json.loads(raw.decode('utf8', errors='strict'), object_pairs_hook=pairs,
                      parse_constant=lambda _: (_ for _ in ()).throw(ValueError('nonfinite JSON')))


class Engine:
    def __init__(self, engine_uid, seconds=180):
        integer(engine_uid, 4294967295)
        integer(seconds, 300, 1)
        self.engine_uid, self.deadline = engine_uid, time.monotonic() + seconds

    def request(self, method, path, *, value=None, emitter=None, stream_maximum=0,
                statuses=(200, 201, 204), maximum_response=MAX_RESPONSE):
        require(method in ['GET', 'POST', 'PUT', 'DELETE'] and path.startswith('/')
                and len(path) <= 4096 and '\r' not in path and '\n' not in path, 'fixed preparation request')
        require(value is None or emitter is None, 'one request body')
        integer(maximum_response, MAX_RESPONSE, 1)
        connection = Connection(self.deadline, self.engine_uid)
        response = None
        try:
            if emitter is not None:
                integer(stream_maximum, 1075838976, 1)
                connection.putrequest(method, '/v1.49' + path)
                connection.putheader('Content-Type', 'application/x-tar')
                connection.putheader('Transfer-Encoding', 'chunked')
                connection.putheader('Connection', 'close')
                connection.endheaders()
                output = Chunked(connection, stream_maximum)
                receipt = emitter(output)
                # A failed emitter never sends a successful terminating chunk.
                connection.send(b'0\r\n\r\n')
            else:
                receipt = None
                body = b'' if value is None else json.dumps(value, separators=(',', ':')).encode('utf8')
                require(len(body) <= MAX_RESPONSE, 'bounded preparation request')
                connection.request(method, '/v1.49' + path, body=body,
                                   headers={'Content-Type': 'application/json', 'Connection': 'close'})
            response = connection.getresponse()
            raw = response.read(maximum_response + 1)
            require(len(raw) <= maximum_response, 'bounded daemon response')
            require(response.status in statuses, f'fixed daemon operation refused: HTTP {response.status}')
            require(time.monotonic() < self.deadline, 'whole preparation deadline exceeded')
            return raw, receipt
        finally:
            try:
                if response is not None:
                    response.close()
            finally:
                connection.close()

    def json(self, method, path, value=None):
        raw, _ = self.request(method, path, value=value, statuses=(200, 201))
        return decode(raw)
