"""Synthetic transport controls only: no daemon, signature binary, Python candidate or guest."""
import copy
import io
import json
import tarfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch
import preparation_volume as volume_module
from preparation import image_state
from preparation_fixture import repin, selection
from preparation_plan import CONFIG, FLAGS, NODE, validate, document, PYTHON_LANDMARK_DIRECTORIES
from preparation_primitives import emit
from preparation_volume import holder, holder_config, no_writers


class PreparationTests(unittest.TestCase):
    def test_python_exec_prefix_landmark_is_an_empty_sealed_directory(self):
        image, _, _ = validate(selection())
        landmark = 'usr/lib/python3.12/lib-dynload'
        self.assertIn(landmark, image['directories'])
        self.assertFalse(any(row['path'] == landmark or row['path'].startswith(landmark + '/') for row in image['files']))
        self.assertEqual(image['bytes'], sum(row['bytes'] for row in image['files']))
        sources = SimpleNamespace(revalidate_files=lambda: None)
        output = io.BytesIO()
        receipt = emit(document([], PYTHON_LANDMARK_DIRECTORIES), sources, output)
        self.assertEqual((receipt['files'], receipt['sourceBytes']), (0, 0))
        with tarfile.open(fileobj=io.BytesIO(output.getvalue()), mode='r:') as archive:
            row = archive.getmember(landmark)
            self.assertTrue(row.isdir())
            self.assertEqual((row.uid, row.gid, row.mode, row.size), (0, 0, 0o555, 0))

    def test_complete_ten_materials_and_node_only_external(self):
        image, materials, manifest = validate(selection())
        self.assertEqual((len(materials['files']), materials['bytes']), (10, 133968848))
        self.assertNotIn(NODE.lstrip('/'), [row['path'] for row in image['files']])
        self.assertEqual(json.loads(manifest)['role'], 'compiler')
        self.assertEqual(image['directories'], sorted(set(image['directories'])))
        self.assertTrue(all(value is False for value in FLAGS.values()))

    def test_missing_provenance_and_foreign_material_fail_closed(self):
        for mutate in [lambda value: value.update(provenance={}),
                       lambda value: value['materialSources'].pop(),
                       lambda value: value['materialSources'][0].update(path='/materials/foreign'),
                       lambda value: value['plan'].update(runtimeApproved=True)]:
            value = selection(); mutate(value)
            with self.assertRaises((ValueError, KeyError)):
                validate(repin(value))

    def test_actual_image_shadow_missing_edge_and_disconnected_rows(self):
        def shadow(value):
            value['runtime']['files'][0]['path'] = NODE
        def missing(value):
            value['runtime']['elf'][-1]['needed'] = [{'name': 'libmissing.so', 'path': '/lib/libmissing.so'}]
        def foreign(value):
            value['runtime']['elf'].append(dict(value['runtime']['elf'][0], path='/materials/foreign'))
        for mutate in [shadow, missing, foreign]:
            value = selection(); mutate(value)
            with self.assertRaises(ValueError):
                validate(repin(value))

    def test_exact_artifact_and_manifest_are_not_arbitrary_support(self):
        for location in ['/build/release/zryna-playground-compiler', '/resources/materials-manifest.json']:
            value = selection()
            next(row for row in value['runtime']['files'] if row['path'] == location)['sha256'] = '0' * 64
            with self.assertRaises(ValueError):
                validate(repin(value))

    def test_exact_image_limit_and_first_extra(self):
        value = selection()
        file = value['runtime']['files'][0]
        file['bytes'] += 128 * 1024**2 - value['plan']['runtime']['bytes']
        value['plan']['prebuild']['artifacts']['compiler']['bytes'] = file['bytes']
        validate(repin(value))
        file['bytes'] += 1
        with self.assertRaises(ValueError):
            validate(repin(value))

    def test_closed_config_refuses_inherited_authority(self):
        identity, layer = 'sha256:' + '1' * 64, '2' * 64
        state = {'Id': identity, 'Os': 'linux', 'Architecture': 'amd64', 'Config': copy.deepcopy(CONFIG),
                 'RootFS': {'Type': 'layers', 'Layers': ['sha256:' + layer]}}
        image_state(state, identity, layer)
        for key, value in [('Env', ['PATH=/host']), ('OnBuild', ['RUN anything']), ('Unknown', False)]:
            changed = copy.deepcopy(state); changed['Config'][key] = value
            with self.assertRaises(ValueError):
                image_state(changed, identity, layer)

    def test_seed_is_never_started_and_runtime_mount_stays_distinct(self):
        config = holder_config('sha256:' + '1' * 64, 'volume', 'nonce')
        self.assertEqual(config['HostConfig']['Mounts'][0], {'Type': 'volume', 'Source': 'volume',
            'Target': '/materials', 'ReadOnly': False, 'VolumeOptions': {'NoCopy': False}})
        state = {'Name': '/holder', 'Image': config['Image'], 'Config': config,
            'HostConfig': config['HostConfig'], 'State': {'Status': 'created', 'Running': False, 'Pid': 0,
            'StartedAt': '0001-01-01T00:00:00Z', 'FinishedAt': '0001-01-01T00:00:00Z'}, 'RestartCount': 0,
            'Mounts': [{'Type': 'volume', 'Name': 'volume', 'Destination': '/materials', 'Driver': 'local', 'RW': True}]}
        holder(state, config['Image'], 'volume', 'nonce', 'holder')
        for key, value in [('Pid', 10), ('Running', True), ('StartedAt', '2026-10-28T00:00:00Z')]:
            changed = copy.deepcopy(state); changed['State'][key] = value
            with self.assertRaises(ValueError):
                holder(changed, config['Image'], 'volume', 'nonce', 'holder')

    def test_no_writer_handoff_uses_complete_inventory_and_actual_mount(self):
        class Engine:
            def __init__(self, writable):
                self.writable, self.calls = writable, []
            def request(self, method, path, **options):
                self.calls.append((method, path))
                return json.dumps([{'Id': 'a' * 64}]).encode(), None
            def json(self, method, path):
                self.calls.append((method, path))
                return {'Id': 'a' * 64, 'Mounts': [{'Type': 'volume', 'Name': 'owned',
                    'Source': '/engine/volumes/owned/_data', 'RW': self.writable}]}
        # Synthetic backing metadata only; never inspect the test host.
        metadata = SimpleNamespace(st_dev=1, st_ino=1, st_mode=0o40755)
        with patch.object(volume_module.os.path, 'realpath', side_effect=lambda path, strict: path), patch.object(volume_module.os, 'lstat', return_value=metadata):
            engine = Engine(False)
            self.assertEqual(no_writers(engine, 'owned', '/engine/volumes/owned/_data'), ['a' * 64])
            self.assertEqual(len(engine.calls), 2)
            with self.assertRaises(ValueError):
                no_writers(Engine(True), 'owned', '/engine/volumes/owned/_data')
            class Bind(Engine):
                def json(self, method, path):
                    return {'Id': 'a' * 64, 'Mounts': [{'Type': 'bind', 'Source': '/engine/volumes', 'RW': True}]}
            with self.assertRaises(ValueError):
                no_writers(Bind(True), 'owned', '/engine/volumes/owned/_data')



if __name__ == '__main__':
    unittest.main()
