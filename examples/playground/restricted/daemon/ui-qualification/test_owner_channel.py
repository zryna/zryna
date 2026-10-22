"""No Engine connection: adversarial fixed-attach/audit and readiness-race fixtures."""
import selectors
import struct
import types
import unittest
from unittest import mock
import owner_attach
import owner_audit
import owner_relay


class Stream:
    def __init__(self, data):
        self.data, self.sent, self.closed, self.guards = data, b'', False, 0

    def guard(self):
        self.guards += 1

    def sendall(self, data):
        self.sent += data

    def recv(self, size):
        data, self.data = self.data[:size], self.data[size:]
        return data

    def close(self):
        self.closed = True


class ChannelTests(unittest.TestCase):
    def attach(self, stream):
        with mock.patch.object(owner_attach, 'Connection', return_value=types.SimpleNamespace(
                sock=stream, connect=lambda: None)):
            return owner_attach.attach(300, 'a' * 64)

    def test_attach_preserves_first_multiplex_byte_and_fixes_path(self):
        data = b'HTTP/1.1 101 UPGRADED\r\nConnection: Upgrade\r\nUpgrade: tcp\r\n\r\n'
        stream = Stream(data + b'\x01\x00')
        self.assertIs(self.attach(stream), stream)
        self.assertEqual(stream.data, b'\x01\x00')
        self.assertIn(b'/v1.49/containers/' + b'a' * 64 + b'/attach?', stream.sent)
        self.assertEqual(stream.guards, len(data) * 2)

    def test_attach_refuses_non_hijack_duplicate_headers_truncation_and_oversize(self):
        for raw in [b'HTTP/1.1 200 OK\r\nConnection: Upgrade\r\nUpgrade: tcp\r\n\r\n',
                    b'HTTP/1.1 101 OK\r\nConnection: close\r\nUpgrade: tcp\r\n\r\n',
                    b'HTTP/1.1 101 OK\r\nConnection: Upgrade\r\nUpgrade: tcp\r\nUpgrade: tcp\r\n\r\n',
                    b'HTTP/1.1 101 OK\r\n', b'x' * 16385]:
            stream = Stream(raw)
            with self.assertRaises(ValueError):
                self.attach(stream)
            self.assertTrue(stream.closed)

    def test_audit_rejects_stderr_truncated_and_unbounded_frames(self):
        good = struct.pack('>BBBBI', 1, 0, 0, 0, 2) + b'{}'
        self.assertEqual(owner_audit.streams(good), b'{}')
        for raw in [good[:7], good[:-1], good + struct.pack('>BBBBI', 2, 0, 0, 0, 1) + b'x',
                    struct.pack('>BBBBI', 1, 1, 0, 0, 2) + b'{}', b'x' * 4097]:
            with self.assertRaises(ValueError):
                owner_audit.streams(raw)

    def test_eagain_is_a_readiness_race_not_an_eof_or_false_result(self):
        selector = mock.Mock()
        selector.select.side_effect = [[(types.SimpleNamespace(data='input'), selectors.EVENT_READ)],
                                       [(types.SimpleNamespace(data='control'), selectors.EVENT_READ)]]
        with mock.patch.object(owner_relay.selectors, 'DefaultSelector', return_value=selector), \
                mock.patch.object(owner_relay.os, 'set_blocking'), \
                mock.patch.object(owner_relay.os, 'read', side_effect=[BlockingIOError(), b'']):
            self.assertEqual(owner_relay.run(types.SimpleNamespace(fileno=lambda: 7),
                                           owner_relay.time.monotonic() + 1), 'control-eof')

    def test_stdio_eof_and_exact_stop_are_cleanup_terminals_and_unknown_control_rejected(self):
        for channel, data, terminal in [('input', b'', 'stdin-eof'), ('control', b'', 'control-eof'),
                                        ('control', b'{"type":"stop","deadlineNs":"1000000000"}\n', 'stop')]:
            with mock.patch.object(owner_relay.os, 'read', return_value=data):
                result = owner_relay.transfer(types.SimpleNamespace(data=channel), selectors.EVENT_READ,
                    None, bytearray(), bytearray(), bytearray(), 0, 0, 100)
                self.assertEqual(result[0], terminal)
        for data in [b'{"type":"exec"}\n', b'{"type":"stop"}\n{}\n', b'x' * 1025]:
            with mock.patch.object(owner_relay.os, 'read', return_value=data):
                with self.assertRaises(ValueError):
                    owner_relay.transfer(types.SimpleNamespace(data='control'), selectors.EVENT_READ,
                        None, bytearray(), bytearray(), bytearray(), 0, 0, 100)

    def test_stop_clock_shares_original_deadline_and_cannot_renew_two_seconds(self):
        with mock.patch.object(owner_relay.time, 'monotonic_ns', return_value=100000000000):
            self.assertEqual(owner_relay.stop_deadline(b'{"type":"stop","deadlineNs":"101000000000"}'), 101)
            # Expired limits remain expired so cleanup cannot receive a renewed allowance.
            self.assertEqual(owner_relay.stop_deadline(b'{"type":"stop","deadlineNs":"99000000000"}'), 99)
            for raw in [b'{"type":"stop","deadlineNs":"102000000001"}',
                        b'{"type":"stop","deadlineNs":101000000000}',
                        b'{"type":"stop","deadlineNs":"NaN"}',
                        b'{"type":"stop","deadlineNs":"101000000000","extra":true}']:
                with self.assertRaises(ValueError):
                    owner_relay.stop_deadline(raw)


if __name__ == '__main__':
    unittest.main()
