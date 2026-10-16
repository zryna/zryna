"""Real tiny archives and inert Engine doubles for daemon-only material root preparation."""
import base64
import copy
import hashlib
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import playground_operator_preparation as baseline
import engine as transport
import install
from material_seed import empty_root


def archive(mode=365, uid=0, gid=0, kind=tarfile.DIRTYPE, name='materials', size=0):
    header = tarfile.TarInfo(name)
    header.mode, header.uid, header.gid, header.type, header.size = mode, uid, gid, kind, size
    return header.tobuf(format=tarfile.USTAR_FORMAT) + bytes(1024)


class Engine(baseline.Engine):
    def __init__(self, raw=None, normalized=False):
        super().__init__()
        self.raw = archive() if raw is None else raw
        self.normalized = normalized

    def json(self, method, path, value=None):
        state = super().json(method, path, value)
        if self.normalized and method == 'GET' and path.startswith('/containers/'):
            del state['HostConfig']['Mounts'][0]['ReadOnly']
            state['HostConfig']['Mounts'][0]['VolumeOptions'] = {}
        return state

    def request(self, method, path, **options):
        if method == 'GET' and '/archive?' in path:
            self.calls.append((method, path))
            assert options['maximum_response'] == 1536
            return self.raw, None
        return super().request(method, path, **options)


class Cases(unittest.TestCase):
    def populate(self, engine):
        with tempfile.TemporaryDirectory(prefix='zryna-empty-material-') as temporary:
            return install.populate_materials(engine, baseline.IMAGE,
                *baseline.Cases().make_plan(Path(temporary)))

    def test_actual_archive_bytes_preserved_for_independent_reparse(self):
        engine = Engine(normalized=True)
        result = self.populate(engine)
        self.assertIsNone(result['failure'])
        self.assertTrue(result['holderRemoved'])
        self.assertEqual(result['status'], 'unapproved-operator-preparation')
        root = result['initialEmptyRoot']
        raw = base64.b64decode(root['rawTarBase64'], validate=True)
        self.assertEqual(raw, archive())
        self.assertEqual(root['tarSha256'], hashlib.sha256(raw).hexdigest())
        self.assertTrue(empty_root(raw)['actualEmptyRoot0555'])
        self.assertIs(engine.config['HostConfig']['Mounts'][0]['VolumeOptions']['NoCopy'], False)
        self.assertFalse(any('/start' in path for _, path in engine.calls))
        read = next(index for index, (method, path) in enumerate(engine.calls)
                    if method == 'GET' and '/archive?' in path)
        write = next(index for index, (method, _) in enumerate(engine.calls) if method == 'PUT')
        self.assertLess(read, write)

    def test_wrong_root_or_extra_archive_bytes_never_reach_put(self):
        variants = [archive(mode=493), archive(uid=1), archive(gid=1),
                    archive(kind=tarfile.REGTYPE), archive(kind=tarfile.SYMTYPE),
                    archive(kind=tarfile.XHDTYPE), archive(name='../materials'),
                    archive(size=1), archive()[:-1], archive() + bytes(512),
                    archive()[:512] + archive()]
        for raw in variants:
            with self.subTest(raw=hashlib.sha256(raw).hexdigest()):
                engine = Engine(raw)
                result = self.populate(engine)
                self.assertIsNotNone(result['failure'])
                self.assertEqual(result['holderId'], 'b' * 64)
                self.assertFalse(result['holderRemoved'])
                self.assertFalse(any(method in ['PUT', 'DELETE'] for method, _ in engine.calls))

    def test_corrupt_header_and_nonzero_terminal_are_rejected(self):
        raw = bytearray(archive());raw[148] ^= 1
        with self.assertRaises((ValueError, tarfile.TarError)):
            empty_root(bytes(raw))
        raw = bytearray(archive());raw[-1] = 1
        with self.assertRaises(ValueError):
            empty_root(bytes(raw))
        self.assertTrue(empty_root(archive(name='materials/'))['actualEmptyRoot0555'])

    def test_nocopy_true_or_extra_mount_authority_rejected_before_put(self):
        for field, value in [('NoCopy', True), ('NoCopy', None), ('Subpath', 'hidden')]:
            class Changed(Engine):
                def json(self, method, path, data=None):
                    state = super().json(method, path, data)
                    if method == 'GET' and path.startswith('/containers/'):
                        state['HostConfig']['Mounts'][0]['VolumeOptions'][field] = value
                    return state
            with self.subTest(field=field, value=value):
                engine = Changed()
                result = self.populate(engine)
                self.assertIsNotNone(result['failure'])
                self.assertFalse(any(method == 'PUT' for method, _ in engine.calls))

    def test_engine_first_extra_closes_response_and_connection(self):
        class Response:
            status = 200
            closed = False
            def read(self, maximum):
                self.maximum = maximum
                return bytes(1537)
            def close(self):
                self.closed = True
        class Connection:
            closed = False
            def request(self, *_, **__):
                return None
            def getresponse(self):
                return response
            def close(self):
                self.closed = True
        response, connection = Response(), Connection()
        with patch.object(transport, 'Connection', return_value=connection):
            with self.assertRaisesRegex(ValueError, 'bounded daemon response'):
                transport.Engine(65534, 5).request('GET', '/fixed-readonly-metadata', maximum_response=1536)
        self.assertEqual(response.maximum, 1537)
        self.assertTrue(response.closed and connection.closed)

    def test_response_close_failure_still_closes_connection(self):
        class Response:
            status = 200
            def read(self, _):
                return b'{}'
            def close(self):
                raise OSError('unknown response close')
        class Connection:
            closed = False
            def request(self, *_, **__):
                return None
            def getresponse(self):
                return Response()
            def close(self):
                self.closed = True
        connection = Connection()
        with patch.object(transport, 'Connection', return_value=connection):
            with self.assertRaisesRegex(OSError, 'unknown response close'):
                transport.Engine(65534, 5).request('GET', '/fixed-readonly-metadata')
        self.assertTrue(connection.closed)


if __name__ == '__main__':
    unittest.main()
