"""Peer-authentication and JSON negatives over inert socket/metadata doubles."""
import stat
import struct
import time
import types
import unittest
from unittest import mock
import cli_engine


class Stream:
    def __init__(self, uid=65534):
        self.uid, self.closed = uid, False
    def guard(self):
        return None
    def connect(self, path):
        self.path = path
    def getsockopt(self, *_):
        return struct.pack('3i', 123, self.uid, 1000)
    def getpeername(self):
        return cli_engine.SOCKET
    def close(self):
        self.closed = True


class EngineTests(unittest.TestCase):
    def socket(self, stream, changed=False):
        parent = types.SimpleNamespace(st_mode=stat.S_IFDIR | 0o755, st_uid=0)
        before = types.SimpleNamespace(st_mode=stat.S_IFSOCK | 0o660, st_uid=65534,
                                       st_gid=1000, st_dev=1, st_ino=2, st_ctime_ns=3)
        after = types.SimpleNamespace(**vars(before))
        if changed:
            after.st_ino = 9
        with mock.patch.object(cli_engine.os, 'stat', return_value=parent), \
             mock.patch.object(cli_engine.os, 'lstat', side_effect=[before, after]), \
             mock.patch.object(cli_engine, 'TimedSocket', return_value=stream):
            selected = cli_engine.Connection(time.monotonic() + 1, 65534)
            selected.connect()
            return selected

    def test_fixed_socket_and_peer_uid_both_required_before_connection_is_exposed(self):
        stream = Stream()
        self.assertIs(self.socket(stream).sock, stream)
        for uid, changed in [(0, False), (1000, False), (65534, True)]:
            stream = Stream(uid)
            with self.assertRaises(ValueError):
                self.socket(stream, changed)
            self.assertTrue(stream.closed)
        for peer in [0, 1000, True]:
            with self.assertRaises(ValueError):
                cli_engine.Engine(peer, seconds=10)

    def test_duplicate_nonfinite_and_invalid_utf8_never_enter_engine_observations(self):
        self.assertEqual(cli_engine.decode(b'{"finite":1}'), {'finite': 1})
        for raw in [b'{"id":1,"id":2}', b'{"n":NaN}', b'{"n":Infinity}', b'\xff']:
            with self.assertRaises((ValueError, UnicodeError)):
                cli_engine.decode(raw)


if __name__ == '__main__':
    unittest.main()
