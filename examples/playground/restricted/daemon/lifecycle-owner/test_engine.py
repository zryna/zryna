"""Inert native socket controls; no daemon or application is contacted."""
import copy
import base64
import json
import struct
import time
import unittest
from types import SimpleNamespace
from unittest.mock import Mock, patch
from owner_test_fixture import OWNER

ENGINE = OWNER._load('lifecycle_engine')


class ConnectionTests(unittest.TestCase):
    def connect(self, peer='/run/docker.sock', uid=0, mutate=None, changed=False):
        state = SimpleNamespace(st_dev=1, st_ino=2, st_uid=0, st_gid=999,
                                st_mode=0o140660, st_ctime_ns=123)
        directory = copy.copy(state)
        directory.st_mode = 0o40755
        states = [copy.copy(state) for _ in range(4)]
        if mutate:
            mutate(directory, states)
        if changed:
            states[3].st_ino += 1
        stream = Mock()
        stream.getsockopt.return_value = struct.pack('3i', 1, uid, 0)
        stream.getpeername.return_value = peer
        connection = ENGINE.Connection(time.monotonic() + 1, 0)
        with patch.object(ENGINE.os, 'stat', return_value=directory), \
                patch.object(ENGINE.os, 'lstat', side_effect=states), \
                patch.object(ENGINE.socket, 'SO_PEERCRED', 17, create=True), \
                patch.object(ENGINE, 'TimedSocket', return_value=stream) as factory:
            try:
                connection.connect()
            except ValueError:
                json.dumps(connection.observation)  # Original refusal diagnostics must survive.
                self.assertIsNone(connection.sock)
                if factory.called:
                    stream.close.assert_called_once()
                raise
        stream.connect.assert_called_once_with('/var/run/docker.sock')
        stream.close.assert_not_called()
        self.assertIs(connection.sock, stream)
        self.assertEqual(connection.observation['peer']['name'], peer)
        self.assertEqual(len(connection.observation['parents']), 4)
        self.assertEqual(connection.observation['before'], connection.observation['after'])

    def test_both_fixed_names_require_same_authenticated_socket(self):
        for name in ['/var/run/docker.sock', '/run/docker.sock']:
            with self.subTest(name=name):
                self.connect(peer=name)

    def test_foreign_peer_name_and_nonroot_peer_refused(self):
        for options in [{'peer': '/tmp/docker.sock'}, {'peer': ''}, {'peer': b'\0foreign'}, {'uid': 65534}]:
            with self.subTest(options=options), self.assertRaises(ValueError):
                self.connect(**options)

    def test_alias_replacement_and_unsafe_socket_ancestry_refused(self):
        changes = [lambda directory, states: setattr(directory, 'st_mode', 0o40777),
                   lambda directory, states: setattr(directory, 'st_uid', 1001),
                   lambda directory, states: setattr(states[0], 'st_mode', 0o140666),
                   lambda directory, states: setattr(states[0], 'st_uid', 1001),
                   lambda directory, states: setattr(states[1], 'st_ino', 3),
                   lambda directory, states: setattr(states[1], 'st_mode', 0o120777)]
        for change in changes:
            with self.subTest(change=change), self.assertRaises(ValueError):
                self.connect(mutate=change)
        with self.assertRaises(ValueError):
            self.connect(changed=True)


class RequestTests(unittest.TestCase):
    def request(self, status=200, emitter=None):
        connection, response = Mock(), Mock()
        connection.observation = {'synthetic': True}
        connection.sock.getsockopt.return_value = struct.pack('3i', 1, 0, 0)
        response.status, response.length = status, 0
        response.read.side_effect = [b'{"message":"original daemon response"}', b'']
        connection.getresponse.return_value = response
        state = SimpleNamespace(st_dev=1, st_ino=2, st_uid=0, st_gid=0,
                                st_mode=0o140660, st_ctime_ns=123)
        engine = ENGINE.Engine({}, time.monotonic() + 1)
        with patch.object(ENGINE, 'Connection', return_value=connection), \
                patch.object(ENGINE.os, 'lstat', return_value=state), \
                patch.object(ENGINE.socket, 'SO_PEERCRED', 17, create=True), \
                patch.object(ENGINE, 'ticks', return_value='60'):
            try:
                engine.request('POST' if emitter else 'GET', '/images/create' if emitter else '/info',
                               emitter=emitter, stream_maximum=1024 if emitter else 0, statuses=(200,))
            except ValueError:
                self.assertEqual(engine.record[0]['status'], status)
                self.assertEqual(base64.b64decode(engine.record[0]['rawBase64']),
                                 b'{"message":"original daemon response"}')
                self.assertIsNotNone(engine.record[0]['failure'])
                connection.close.assert_called_once()
                raise
        connection.close.assert_called_once()
        return connection

    def test_native_json_and_chunked_transport_pin_compatible_api(self):
        connection = self.request()
        self.assertEqual(connection.request.call_args.args[:2], ('GET', '/v1.48/info'))
        connection = self.request(emitter=lambda output: output.write(b'abc'))
        connection.putrequest.assert_called_once_with('POST', '/v1.48/images/create')
        self.assertEqual(connection.send.call_args.args, (b'0\r\n\r\n',))

    def test_original_api_refusal_is_retained_and_remains_failure(self):
        with self.assertRaises(ValueError):
            self.request(status=400)


if __name__ == '__main__':
    unittest.main()
