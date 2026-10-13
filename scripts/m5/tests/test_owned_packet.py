"""Raw record availability and exact five-case order controls; never native execution."""
from pathlib import Path
import unittest
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from owned_packet import ORDER, records, validate
from collect_owned_sanitizer_runtime import provider_table
from unittest.mock import patch


def record(name, success=True):
    return {'name': name, 'diagnostic': None, 'error_raw_fields_available': True,
            'success': success, 'child_env_cleared': True, 'directory': '/owned/temporary',
            'child_env': {'LANG': 'C', 'LC_ALL': 'C', 'TZ': 'UTC', 'SOURCE_DATE_EPOCH': '0',
                          'PATH': '/usr/bin:/bin', 'TMPDIR': '/owned/temporary'}}


class OwnedRuntimeRecords(unittest.TestCase):
    def test_only_negative_run_exit_may_be_unsuccessful(self):
        records([record('compile-client'), record('run', False)], ('compile-client', 'run'), True)
        with self.assertRaises(AssertionError):
            records([record('compile-client', False), record('run', False)],
                    ('compile-client', 'run'), True)
        with self.assertRaises(AssertionError):
            records([record('run', False)], ('run',), False)

    def test_helper_error_and_absent_output_never_count_as_negative(self):
        for field, value in [('diagnostic', 'ZRYNA-C4102'), ('error_raw_fields_available', False)]:
            row = record('run', False)
            row[field] = value
            with self.assertRaises(AssertionError):
                records([row], ('run',), True)

    def test_order_and_native_environment_are_closed(self):
        self.assertEqual(ORDER, ('sanitizer-positive', 'sanitizer-heap-overrun',
                                'sanitizer-positive', 'sanitizer-signed-overflow', 'sanitizer-positive'))
        with self.assertRaises(AssertionError):
            records([record('run'), record('compile-client')], ('compile-client', 'run'))
        row = record('run')
        row['child_env']['PATH'] = '/ambient/other'
        with self.assertRaises(AssertionError):
            records([row], ('run',))

    def test_unknown_or_duplicate_case_catalog_rejects_before_receipt_reads(self):
        for names in [list(ORDER[:3]), ['unknown-case', ORDER[1], ORDER[3]]]:
            with patch('owned_packet.read') as read:
                with self.assertRaises(AssertionError):
                    validate({}, Path('/never-opened'), {'cases': [{'case': n} for n in names]})
                read.assert_not_called()

    def test_missing_or_changed_provider_cannot_pass_material_comparison(self):
        pins = [{'canonicalPath': '/usr/lib/provider-' + str(i), 'bytes': 1,
                 'sha256': 'a' * 64} for i in range(15)]
        material = {'retainedMaterials': list(pins), 'uniqueMaterialCount': 15,
                    'uniqueMaterialBytes': 15}
        provider_table(material, {'providers': pins})
        for changed in [pins[:-1], [dict(p, sha256='b' * 64) for p in pins]]:
            with patch('collect_owned_sanitizer_runtime.compile_owned') as compile_owned:
                with patch('collect_owned_sanitizer_runtime.run') as run:
                    with self.assertRaises(AssertionError):
                        provider_table(dict(material, retainedMaterials=changed), {'providers': pins})
                    compile_owned.assert_not_called()
                    run.assert_not_called()
