"""Inert native socket controls; no daemon or application is contacted."""
import copy
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


if __name__ == '__main__':
    unittest.main()
