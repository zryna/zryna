"""Distinct plan/import negatives with inert materialization/emission, no daemon calls or CLI execution."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock
import cli_image_import as producer
from cli_image_plan import validate
from cli_image_sources import Sources
from test_cli_image_helpers import document, ImageEngine


class ImageTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix='zryna-cli-image-controls-')
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.plan = self.root / 'plan.json'
        self.value = document()
        self.raw = (json.dumps(self.value, separators=(',', ':')) + '\n').encode()
        self.plan.write_bytes(self.raw)
        self.sha = hashlib.sha256(self.raw).hexdigest()
        self.result = self.root / 'result.json'

    def test_separate_plan_refuses_production_roles_incomplete_release_changed_original_modes_or_runtime(self):
        validate(self.value)
        for mutate in [lambda x: x.update(format='zryna.playground-preparation-plan.v1'),
                       lambda x: x['files'].pop(), lambda x: x['files'][0].update(mode=0o444),
                       lambda x: x['runtimeFiles'][0].update(sha256='f' * 64),
                       lambda x: x['directories'].append('extra'), lambda x: x['config'].update(Volumes={}),
                       lambda x: x['archive'].update(sha256='f' * 64)]:
            changed = copy.deepcopy(self.value); mutate(changed)
            with self.assertRaises(ValueError):
                validate(changed)

    def test_external_plan_mismatch_precedes_all_source_hashing_emission_or_engine_access(self):
        engine = ImageEngine()
        with mock.patch.object(Sources, 'verify') as verify:
            result = producer.import_image(str(self.plan), len(self.raw), 'f' * 64, str(self.result), engine)
        self.assertEqual(result['failure']['phase'], 'selected-plan'); verify.assert_not_called()
        self.assertEqual(engine.calls, []); self.assertIsNone(result['cache']); self.assertFalse(result['mutationAttempted'])
        self.assertTrue(self.result.is_file()); self.assertEqual(self.result.stat().st_mode & 0o777, 0o400)

    def test_existing_result_is_never_overwritten_and_no_material_or_daemon_operation_occurs(self):
        self.result.write_bytes(b'original result\n')
        engine = ImageEngine()
        result = producer.import_image(str(self.plan), len(self.raw), self.sha, str(self.result), engine)
        self.assertEqual(result['failure']['phase'], 'reserve-record')
        self.assertEqual(self.result.read_bytes(), b'original result\n'); self.assertEqual(engine.calls, [])

    def simulate(self, wrong_layer=False, collision=False):
        engine = ImageEngine(wrong_layer, collision)
        def prepare(sources, plan_path, plan_bytes, plan_sha, *_):
            # Deliberate inert fixtures: this branch proves orchestration only.
            return self.raw, self.value, {'files': [], 'directories': [], 'bytes': 0}
        emission = {'archiveSha256': 'd' * 64, 'archiveBytes': 10240, 'sourceBytes': 160488297, 'files': 360}
        with mock.patch.object(producer, 'prepare', side_effect=prepare), \
             mock.patch.object(producer, 'emit_selected', return_value=emission), \
             mock.patch.object(Sources, 'verify', return_value=None):
            result = producer.import_image(str(self.plan), len(self.raw), self.sha, str(self.result), engine)
        return result, engine

    def test_actual_rootfs_diffid_must_equal_complete_emitted_stream_and_failed_import_is_retained(self):
        result, engine = self.simulate(wrong_layer=True)
        self.assertEqual(result['failure']['phase'], 'actual-image-readback')
        self.assertTrue(result['mutationAttempted']); self.assertIsNotNone(result['image'])
        self.assertFalse(result['runtimeApproved']); self.assertFalse(result['cliOrProviderExecuted'])
        self.assertNotIn('sourceIdentitiesBeforeAfterEqual', result)

    def test_random_tag_absence_is_checked_before_import_and_no_existing_image_is_deleted(self):
        result, engine = self.simulate(collision=True)
        self.assertFalse(result['mutationAttempted']); self.assertEqual(result['failure']['phase'], 'create-only-tag-absence')
        self.assertFalse(any(method != 'GET' for method, _ in engine.calls))

    def test_inert_success_still_remains_unapproved_and_preserves_reserved_record_and_cache(self):
        result, engine = self.simulate()
        self.assertIsNone(result['failure']); self.assertTrue(result['sourceIdentitiesBeforeAfterEqual'])
        self.assertEqual(result['status'], 'unapproved-operator-image'); self.assertFalse(result['runtimeApproved'])
        saved = json.loads(self.result.read_text()); self.assertEqual(saved['image'], result['image'])
        self.assertTrue(Path(saved['cache']).is_dir())
        self.assertTrue(all('/containers/' not in path for _, path in engine.calls))


if __name__ == '__main__':
    unittest.main()
