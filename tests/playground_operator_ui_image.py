"""Trusted UI preparation uses a separate closed role; these tests execute no guest."""
import copy
import hashlib
import io
import json
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] /
                       'examples/playground/restricted/daemon/preparation'))
import plan
import ui_image
import ui_plan


class Cases(unittest.TestCase):
    def fixture(self, root):
        source = root / 'ordinary-test-data'
        source.write_bytes(b'qualification archive test data only\n')
        data = source.read_bytes()
        files = [{'path': name, 'sourcePath': str(source), 'bytes': len(data),
                  'sha256': hashlib.sha256(data).hexdigest(),
                  'mode': 365 if name in ['supervisor', 'ui-supervisor', 'app/compiler',
                                          'lib64/ld-linux-x86-64.so.2'] else 292}
                 for name in sorted(ui_plan.REQUIRED)]
        document = ui_plan.candidate(files)
        raw = json.dumps(document, separators=(',', ':')).encode()
        location = root / 'ui-image-plan.json'
        location.write_bytes(raw)
        return document, location, len(raw), hashlib.sha256(raw).hexdigest()

    def test_real_archive_exact_inventory_and_metadata(self):
        with tempfile.TemporaryDirectory(prefix='zryna-trusted-ui-image-') as temporary:
            document, location, size, sha = self.fixture(Path(temporary))
            output = io.BytesIO()
            receipt = ui_image.run(str(location), size, sha, output)
            with tarfile.open(fileobj=io.BytesIO(output.getvalue())) as archive:
                members = archive.getmembers()
                self.assertEqual([item.name for item in members],
                                 ['.', *document['directories'], *(item['path'] for item in document['files'])])
                self.assertTrue(all(item.uid == 0 and item.gid == 0 and item.mtime == 0 for item in members))
                self.assertTrue(all(item.isdir() or item.isreg() for item in members))
            self.assertEqual(receipt['archiveSha256'], hashlib.sha256(output.getvalue()).hexdigest())
            self.assertTrue(receipt['sourceInventoryBeforeAfterEqual'])

    def test_production_plan_does_not_admit_ui_role(self):
        with tempfile.TemporaryDirectory(prefix='zryna-trusted-ui-image-') as temporary:
            document, *_ = self.fixture(Path(temporary))
            with self.assertRaises(ValueError):
                plan.validate(document)

    def test_unknown_fields_selectors_or_missing_runtime_rejected(self):
        with tempfile.TemporaryDirectory(prefix='zryna-trusted-ui-image-') as temporary:
            document, *_ = self.fixture(Path(temporary))
            for mutate in [lambda item: item.update(arbitraryCommand=True),
                           lambda item: item['config'].update(Cmd=['sh']),
                           lambda item: item['config'].update(Entrypoint=['/supervisor']),
                           lambda item: item['files'].pop(),
                           lambda item: item['files'][0].update(path='app/unknown.mjs'),
                           lambda item: item['files'][0].update(mode=365),
                           lambda item: item.update(bytes=2**40),
                           lambda item: item['directories'].append('undeclared')]:
                candidate = copy.deepcopy(document)
                mutate(candidate)
                with self.assertRaises(ValueError):
                    ui_plan.validate(candidate)

    def test_bad_external_pin_releases_no_archive(self):
        with tempfile.TemporaryDirectory(prefix='zryna-trusted-ui-image-') as temporary:
            _, location, size, _ = self.fixture(Path(temporary))
            output = io.BytesIO()
            with self.assertRaises(ValueError):
                ui_image.run(str(location), size, '0' * 64, output)
            self.assertEqual(output.getvalue(), b'')

    def test_candidate_configuration_cannot_mutate_fixed_policy(self):
        with tempfile.TemporaryDirectory(prefix='zryna-trusted-ui-image-') as temporary:
            document, *_ = self.fixture(Path(temporary))
            document['config']['Env'].append('UNAPPROVED=1')
            self.assertEqual(len(ui_plan.CONFIG['Env']), 7)
            with self.assertRaises(ValueError):
                ui_plan.validate(document)

    def test_closed_actual_image_configuration_cmd_normalization(self):
        identity, layer = 'sha256:' + 'a' * 64, 'b' * 64
        state = {'Id': identity, 'Os': 'linux', 'Architecture': 'amd64',
                 'Config': copy.deepcopy(ui_plan.CONFIG), 'RootFS': {'Type': 'layers', 'Layers': ['sha256:' + layer]}}
        for cmd in [[], None]:
            state['Config']['Cmd'] = cmd
            ui_image.image(state, identity, layer)
        for key, value in [('Cmd', ['sh']), ('Healthcheck', {'Test': ['CMD', 'sh']}),
                           ('Volumes', {'/other': {}}), ('unknown', None), ('User', '0:0')]:
            candidate = copy.deepcopy(state)
            candidate['Config'][key] = value
            with self.assertRaises(ValueError):
                ui_image.image(candidate, identity, layer)
        state['RootFS']['Layers'].append('sha256:' + 'c' * 64)
        with self.assertRaises(ValueError):
            ui_image.image(state, identity, layer)


if __name__ == '__main__':
    unittest.main()
