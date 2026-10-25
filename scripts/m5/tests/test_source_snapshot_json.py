"""Inert serialization fixtures exercise bounds; they supply no Git/source authority."""
from pathlib import Path
import json
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from capture_io import Directory, REPORT_LIMIT, parse_json, sha, write_json


def encoded(value, compact=False):
    options = {'ensure_ascii': False}
    if compact:
        options['separators'] = (',', ':')
    else:
        options['indent'] = 2
    return (json.dumps(value, **options) + '\n').encode()


def snapshot():
    # Full-width metadata, not a selected subset or a counterfeit checkout/receipt.
    rows = []
    for index in range(3864):
        rows.append({
            'path': f'crates/zryna-ir/src/generic_v1/owned_v2/source/fixture_{index:05}.rs',
            'mode': '100755' if index % 7 == 0 else '100644',
            'blob': f'{index:040x}',
            'bytes': index + 1,
            'sha256': f'{index:064x}',
        })
    rows[-1]['path'] = 'tests/utf8/é😀/source.rs'
    return {'candidate': 'a' * 40, 'checkoutCommit': 'b' * 40,
            'tree': 'c' * 40, 'clean': True, 'files': rows}


class SourceSnapshotJson(unittest.TestCase):
    def test_full_snapshot_compacts_without_losing_any_source_identity(self):
        value = snapshot()
        legacy = encoded(value)
        compact = encoded(value, compact=True)
        self.assertGreater(len(legacy), REPORT_LIMIT)
        self.assertLessEqual(len(compact), REPORT_LIMIT)
        with tempfile.TemporaryDirectory() as directory:
            with Directory(directory) as held:
                with self.assertRaises(AssertionError):
                    write_json(held, 'legacy.json', value)
                self.assertFalse((Path(directory) / 'legacy.json').exists())
                pin = write_json(held, 'SOURCE_BEFORE.json', value, compact=True)
                after = write_json(held, 'SOURCE_AFTER.json', value, compact=True)
                actual = held.read(pin['path'], REPORT_LIMIT)
                self.assertEqual(actual, compact)
                self.assertEqual(pin, {'path': 'SOURCE_BEFORE.json',
                                      'bytes': len(actual), 'sha256': sha(actual)})
                self.assertEqual(held.read(after['path'], REPORT_LIMIT), actual)
                restored = parse_json(actual)
                self.assertEqual(restored, parse_json(legacy))
                self.assertEqual(restored, value)
                self.assertEqual(len(restored['files']), 3864)
                self.assertEqual(restored['files'], value['files'])
                for row in restored['files']:
                    self.assertEqual(set(row), {'path', 'mode', 'blob', 'bytes', 'sha256'})
                self.assertEqual(value, snapshot(), 'serialization cannot mutate source state')
                with self.assertRaises(FileExistsError):
                    write_json(held, 'SOURCE_BEFORE.json', value, compact=True)
                self.assertEqual(held.read(pin['path'], REPORT_LIMIT), actual)

    def test_default_report_bytes_remain_exactly_legacy_pretty_json(self):
        value = {'text': 'é😀', 'nested': [{'exact': True, 'items': [1, 2, None]}]}
        expected = (json.dumps(value, indent=2, ensure_ascii=False) + '\n').encode()
        with tempfile.TemporaryDirectory() as directory:
            with Directory(directory) as held:
                for name, options in [('default.json', {}), ('explicit.json', {'compact': False})]:
                    pin = write_json(held, name, value, **options)
                    self.assertEqual(held.read(name, REPORT_LIMIT), expected)
                    self.assertEqual(pin, {'path': name, 'bytes': len(expected),
                                           'sha256': sha(expected)})

    def test_compact_exact_byte_limit_and_first_extra_reject_before_creation(self):
        overhead = len(encoded({'padding': ''}, compact=True))
        exact = {'padding': 'x' * (REPORT_LIMIT - overhead)}
        self.assertEqual(len(encoded(exact, compact=True)), REPORT_LIMIT)
        extra = {'padding': exact['padding'] + 'x'}
        self.assertEqual(len(encoded(extra, compact=True)), REPORT_LIMIT + 1)
        with tempfile.TemporaryDirectory() as directory:
            with Directory(directory) as held:
                pin = write_json(held, 'exact.json', exact, compact=True)
                self.assertEqual(pin['bytes'], REPORT_LIMIT)
                self.assertEqual(parse_json(held.read(pin['path'], REPORT_LIMIT)), exact)
                with self.assertRaises(AssertionError):
                    write_json(held, 'first-extra.json', extra, compact=True)
                self.assertFalse((Path(directory) / 'first-extra.json').exists())
                self.assertEqual(parse_json(held.read(pin['path'], REPORT_LIMIT)), exact)


if __name__ == '__main__':
    unittest.main()
