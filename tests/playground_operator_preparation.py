"""Real filesystem/tar transport with an inert Engine double; no runtime qualification."""
import hashlib
import copy
import io
from pathlib import Path
import sys
import tempfile
import tarfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'examples/playground/restricted/daemon/preparation'))
import install
from compose import candidate, encode, material_manifest
from engine import Chunked

IMAGE = 'sha256:' + 'a' * 64


class Engine:
    def __init__(self, fault=None):
        self.calls, self.config, self.volume, self.fault = [], None, None, fault

    def json(self, method, path, value=None):
        self.calls.append((method, path))
        if path.startswith('/images/'):
            return {'Id': IMAGE, 'Os': 'linux', 'Architecture': 'amd64', 'Config': install.CONFIG}
        if path == '/volumes/create':
            self.volume = dict(value, Scope='local', Options=None, CreatedAt='2026-10-15T19:30:00Z')
            return self.volume.copy()
        if path.startswith('/containers/create?'):
            self.config = value
            return {'Id': 'b' * 64}
        if path.startswith('/containers/'):
            return {'Name': '/zryna410-provisioner-' + self.volume['Labels']['zryna410.materials-nonce'],
                    'Config': copy.deepcopy({key: value for key, value in self.config.items() if key != 'HostConfig'}),
                    'HostConfig': copy.deepcopy(self.config['HostConfig']), 'RestartCount': 0,
                    'State': {'Status': 'created', 'Running': False, 'Pid': 0,
                              'StartedAt': '0001-01-01T00:00:00Z', 'FinishedAt': '0001-01-01T00:00:00Z'},
                    'Image': IMAGE,
                    'Mounts': [{'Type': 'volume', 'Driver': 'local', 'Name': self.volume['Name'],
                                'Destination': '/materials', 'RW': True}]}
        if path.startswith('/volumes/'):
            value = self.volume.copy()
            if self.fault == 'recreated':
                value['CreatedAt'] = '2026-10-15T19:30:01Z'
            return value
        raise AssertionError('unexpected endpoint: ' + path)

    def request(self, method, path, **options):
        self.calls.append((method, path))
        if method == 'PUT':
            output = io.BytesIO()
            if self.fault == 'emission':
                raise OSError('inert stream transport failure')
            result = options['emitter'](output)
            assert result['sourceInventoryBeforeAfterEqual'] is True
            assert result['archiveBytes'] == len(output.getvalue())
            return b'', result
        if method == 'DELETE':
            return b'', None
        if method == 'GET':
            if '/archive?' in path:
                header = tarfile.TarInfo('materials')
                header.type = tarfile.DIRTYPE
                header.mode = 365
                assert options['maximum_response'] == 1536
                return header.tobuf(format=tarfile.USTAR_FORMAT) + bytes(1024), None
            return b'{"message":"No such container"}', None
        raise AssertionError('unexpected endpoint: ' + path)


class Cases(unittest.TestCase):
    def make_plan(self, root):
        data = b'operator-selected inert Node stand-in; never executed'
        source = root / 'node'
        source.write_bytes(data)
        item = {'path': 'runtime/node/bin/node', 'sourcePath': str(source),
                'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest(), 'mode': 365}
        document = candidate('materials', 'compiler', [item])
        raw, pin = encode(document)
        plan = root / 'plan.json'
        plan.write_bytes(raw)
        return str(plan), pin['bytes'], pin['sha256'], hashlib.sha256(material_manifest(document)).hexdigest()

    def test_never_started_writer_removed_before_unapproved_material_observation(self):
        with tempfile.TemporaryDirectory(prefix='zryna-operator-plan-') as temp:
            engine = Engine()
            result = install.populate_materials(engine, IMAGE, *self.make_plan(Path(temp)))
            self.assertIsNone(result['failure'])
            self.assertTrue(result['holderRemoved'])
            self.assertEqual(result['status'], 'unapproved-operator-preparation')
            self.assertFalse(any('/start' in path for _, path in engine.calls))
            self.assertTrue(engine.config['HostConfig']['ReadonlyRootfs'])
            self.assertEqual(engine.config['HostConfig']['CapDrop'], ['ALL'])
            self.assertEqual(engine.config['HostConfig']['NetworkMode'], 'none')

    def test_failed_upload_retains_exact_writer_and_volume_without_approval(self):
        with tempfile.TemporaryDirectory(prefix='zryna-operator-plan-') as temp:
            engine = Engine('emission')
            result = install.populate_materials(engine, IMAGE, *self.make_plan(Path(temp)))
            self.assertIn('OSError', result['failure'])
            self.assertFalse(result['holderRemoved'])
            self.assertEqual(result['holderId'], 'b' * 64)
            self.assertFalse(any(method == 'DELETE' for method, _ in engine.calls))

    def test_volume_recreation_is_a_failure_after_writer_removal(self):
        with tempfile.TemporaryDirectory(prefix='zryna-operator-plan-') as temp:
            result = install.populate_materials(Engine('recreated'), IMAGE, *self.make_plan(Path(temp)))
            self.assertTrue(result['holderRemoved'])
            self.assertIn('creation identity changed', result['failure'])

    def test_wrong_external_plan_pin_has_no_daemon_effect(self):
        with tempfile.TemporaryDirectory(prefix='zryna-operator-plan-') as temp:
            selected = list(self.make_plan(Path(temp)))
            selected[2] = '0' * 64
            engine = Engine()
            with self.assertRaisesRegex(ValueError, 'plan digest'):
                install.populate_materials(engine, IMAGE, *selected)
            self.assertEqual(engine.calls, [])

    def test_runtime_image_rejects_inherited_or_unknown_authority(self):
        for key, value in [('Healthcheck', {'Test': ['CMD', '/surprise']}),
                           ('OnBuild', ['RUN surprise']), ('ExposedPorts', {'1234/tcp': {}}),
                           ('Volumes', {'/unreviewed': {}}), ('FutureAuthority', None)]:
            state = {'Id': IMAGE, 'Os': 'linux', 'Architecture': 'amd64',
                     'Config': dict(install.CONFIG, **{key: value})}
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, 'authority refused'):
                install.validate_runtime_image(state, IMAGE)

    def test_chunk_writer_rejects_first_extra_before_sending(self):
        class Wire:
            def __init__(self):
                self.output = bytearray()
            def send(self, data):
                self.output.extend(data)
        wire = Wire()
        output = Chunked(wire, 3)
        output.write(b'abc')
        before = bytes(wire.output)
        with self.assertRaises(ValueError):
            output.write(b'x')
        self.assertEqual(bytes(wire.output), before)
        self.assertNotIn(b'0\r\n\r\n', before)


if __name__ == '__main__':
    unittest.main()
