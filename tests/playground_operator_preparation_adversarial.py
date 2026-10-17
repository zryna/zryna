"""Adversarial operator metadata/transport gates with inert local Engine doubles."""
import copy
import io
import json
from pathlib import Path
import tempfile
import unittest
import playground_operator_preparation as fixtures
from compose import candidate, encode
import install
from plan import BASE, CONFIG


class MaterialEngine(fixtures.Engine):
    def __init__(self, change):
        super().__init__()
        self.change, self.reads = change, 0

    def json(self, method, path, value=None):
        state = super().json(method, path, value)
        if method == 'GET' and path.startswith('/containers/'):
            self.reads += 1
            if self.change == 'running-before' or self.change == 'running-after' and self.reads > 2:
                state['State'] = {'Status': 'running', 'Running': True, 'Pid': 123}
            if self.change == 'extra-mount':
                state['Mounts'].append({'Name': 'unowned', 'Destination': '/extra', 'RW': True})
        return state


class ImageEngine:
    def __init__(self, change):
        self.change, self.calls, self.emission = change, [], None

    def request(self, method, path, **options):
        self.calls.append((method, path))
        output = io.BytesIO()
        self.emission = options['emitter'](output)
        if self.change == 'import-error':
            return b'{"error":"inert import failure"}\n', self.emission
        return json.dumps({'status': fixtures.IMAGE}).encode() + b'\n', self.emission

    def json(self, method, path, value=None):
        self.calls.append((method, path))
        config = copy.deepcopy(CONFIG)
        root = 'sha256:' + self.emission['archiveSha256']
        if self.change == 'wrong-layer':
            root = 'sha256:' + '0' * 64
        if self.change == 'wrong-config':
            config['Env'] = ['UNAPPROVED=value']
        return {'Id': fixtures.IMAGE, 'Os': 'linux', 'Architecture': 'amd64',
                'RootFS': {'Type': 'layers', 'Layers': [root]}, 'Config': config}


class Cases(unittest.TestCase):
    def material(self, change):
        with tempfile.TemporaryDirectory(prefix='zryna-operator-negative-') as temp:
            engine = MaterialEngine(change)
            result = install.populate_materials(engine, fixtures.IMAGE,
                *fixtures.Cases().make_plan(Path(temp)))
            return engine, result

    def test_running_holder_cannot_receive_archive(self):
        engine, result = self.material('running-before')
        self.assertIn('never-started', result['failure'])
        self.assertFalse(any(method in ['PUT', 'DELETE'] for method, _ in engine.calls))

    def test_extra_writable_mount_cannot_receive_archive(self):
        engine, result = self.material('extra-mount')
        self.assertIn('sole explicitly writable', result['failure'])
        self.assertFalse(any(method in ['PUT', 'DELETE'] for method, _ in engine.calls))

    def test_unexpected_started_writer_is_retained_as_failed_ownership(self):
        engine, result = self.material('running-after')
        self.assertIn('never execute', result['failure'])
        self.assertFalse(result['holderRemoved'])
        self.assertFalse(any(method == 'DELETE' for method, _ in engine.calls))

    def image(self, change):
        with tempfile.TemporaryDirectory(prefix='zryna-operator-image-') as temp:
            root = Path(temp)
            source = root / 'inert'
            source.write_bytes(b'inert image fixture; never executed')
            prototype = fixtures.Cases().make_plan(root)
            source_item = json.loads(Path(prototype[0]).read_bytes())['files'][0]
            files = [dict(source_item, path=name,
                          mode=365 if name in ['supervisor', 'app/compiler', 'lib64/ld-linux-x86-64.so.2'] else 292)
                     for name in BASE]
            raw, pin = encode(candidate('runtime', 'compiler', files))
            plan = root / 'runtime.json'
            plan.write_bytes(raw)
            engine = ImageEngine(change)
            result = install.import_runtime(engine, str(plan), pin['bytes'], pin['sha256'])
            return engine, result

    def test_exact_import_records_unapproved_full_stream_observation(self):
        _, result = self.image(None)
        self.assertIsNone(result['failure'])
        self.assertEqual(result['status'], 'unapproved-operator-preparation')
        self.assertEqual(result['rootFs']['Layers'], ['sha256:' + result['emission']['archiveSha256']])

    def test_import_error_has_no_image_inspection_or_success(self):
        engine, result = self.image('import-error')
        self.assertIn('image import failed', result['failure'])
        self.assertEqual(len(engine.calls), 1)
        self.assertIsNone(result['image'])

    def test_different_root_or_environment_cannot_finish_preparation(self):
        for fault, reason in [('wrong-layer', 'complete root'), ('wrong-config', 'configuration')]:
            with self.subTest(fault=fault):
                _, result = self.image(fault)
                self.assertIn(reason, result['failure'])


if __name__ == '__main__':
    unittest.main()
