"""Raw record availability and exact five-case order controls; never native execution."""
from pathlib import Path
import unittest
import json
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from owned_packet import ORDER, digest32, parse_runtime_output, records, validate
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

    def runtime_output_fixture(self):
        root = Path('/owned/observations')
        paths = [str(root / ('rust-ordinary-37-' + str(i)) / ('evidence-' + str(i))) for i in range(6)]
        report = {'format': 'zryna.m5.ordinary-explicit-sanitizer-five-attempt-observation.v1',
                  'existing_assertions_completed': True, 'fresh_rust_evidence': paths[0],
                  'attempts': [{'case': case, 'evidence': path} for case, path in zip(ORDER, paths[1:])]}
        stdout = json.dumps(report).encode()
        stderr = ''.join('m5-ordinary-rust-evidence ' + path + '\n' for path in paths).encode()
        return root, report, stdout, stderr

    def test_runtime_output_accepts_exact_six_markers_without_receipt_reads(self):
        root, report, stdout, stderr = self.runtime_output_fixture()
        with patch('owned_packet.read') as read:
            self.assertEqual(parse_runtime_output(stdout, stderr, root), report)
            read.assert_not_called()

    def test_runtime_output_extra_missing_reordered_and_diagnostic_bytes_reject(self):
        root, _, stdout, stderr = self.runtime_output_fixture()
        lines = stderr.splitlines(keepends=True)
        mutations = [b'', stderr[:-1], stderr[:-12], b''.join(lines[:-1]),
                     stderr + lines[0], b''.join([lines[1], lines[0], *lines[2:]]),
                     stderr + b'AddressSanitizer: error\n', stderr.replace(b'\n', b'\r\n'),
                     stderr.replace(b'/owned/observations', b'/foreign/observations', 1),
                     stderr + b'\xff', stderr.replace(b'm5-ordinary-rust-evidence', b'unknown-marker', 1)]
        for changed in mutations:
            with self.subTest(stderr=changed), patch('owned_packet.read') as read:
                with self.assertRaises(AssertionError):
                    parse_runtime_output(stdout, changed, root)
                read.assert_not_called()

    def test_runtime_output_paths_are_unique_contained_canonical_and_control_free(self):
        root, report, _, stderr = self.runtime_output_fixture()
        invalid = ['/foreign/evidence', '/owned/observations/../outside', 'relative/evidence',
                   '/owned/observations//evidence', str(root), str(root / 'bad') + '\nmarker',
                   str(root / 'bad') + '\tmarker', report['fresh_rust_evidence'], None]
        for path in invalid:
            changed = json.loads(json.dumps(report))
            changed['attempts'][2]['evidence'] = path
            with self.subTest(path=path), patch('owned_packet.read') as read:
                with self.assertRaises(AssertionError):
                    parse_runtime_output(json.dumps(changed).encode(), stderr, root)
                read.assert_not_called()

    def test_runtime_output_incomplete_malformed_or_oversized_report_rejects(self):
        root, report, stdout, stderr = self.runtime_output_fixture()
        reports = [dict(report, existing_assertions_completed=False), dict(report, format='unknown'),
                   dict(report, attempts=report['attempts'][:-1]),
                   dict(report, attempts=[report['attempts'][1], report['attempts'][0], *report['attempts'][2:]])]
        raw = [json.dumps(value).encode() for value in reports]
        raw += [b'{}', b'[]', b'not-json', b'\xff', b' ' * (1024 * 1024 + 1),
                stdout[:-1] + b',"existing_assertions_completed":true}']
        for value in raw:
            with self.subTest(stdout=value[:64]), patch('owned_packet.read') as read:
                with self.assertRaises((AssertionError, KeyError, ValueError)):
                    parse_runtime_output(value, stderr, root)
                read.assert_not_called()

    def test_digest32_accepts_exact_rust_byte_vector_full_hash(self):
        vector = [106, 119, 243, 81, 222, 111, 235, 165, 15, 116, 56, 56, 84, 87, 148, 161, 75, 131, 49, 18, 149, 36, 85, 97, 78, 49, 216, 190, 65, 13, 16, 20]
        with patch('owned_packet.read') as read:
            self.assertEqual(digest32(vector), '6a77f351de6feba50f743838545794a14b833112952455614e31d8be410d1014')
            read.assert_not_called()

    def test_digest32_rejects_non_u8_or_non_exact_arrays_before_receipt_reads(self):
        vector = list(range(32))
        invalid = [[], vector[:-1], vector + [32], bytes(vector), vector.copy(),
                   vector.copy(), vector.copy(), vector.copy(), vector.copy(), '00' * 32, None]
        for row, value in zip(invalid[4:9], [-1, 256, True, '0', 0.0]):
            row[0] = value
        for value in invalid:
            with self.subTest(digest=value), patch('owned_packet.read') as read:
                with self.assertRaises(AssertionError):
                    digest32(value)
                read.assert_not_called()
