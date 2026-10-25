"""Serialization controls are inert data, never counterfeit Git/source authority."""
from pathlib import Path
import json
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from capture_io import Directory, REPORT_LIMIT, parse_json, sha, write_json
from source_snapshot import read_snapshot, write_snapshot


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, separators=(',', ':')) + '\n').encode()


def snapshot(count=6000):
    rows = [{'path': f'crates/long_original_source_namespace/module_{i:05}.rs',
             'mode': '100755' if i % 7 == 0 else '100644',
             'blob': f'{i:040x}', 'bytes': i + 1, 'sha256': f'{i:064x}'}
            for i in range(count)]
    rows[-1]['path'] = 'tests/utf8/é😀/original.rs'
    return {'candidate': 'a' * 40, 'checkoutCommit': 'b' * 40,
            'tree': 'c' * 40, 'clean': True, 'files': rows}


class SourceSnapshotShards(unittest.TestCase):
    def test_small_snapshot_preserves_original_compact_bytes_and_descriptor(self):
        value = snapshot(20)
        with tempfile.TemporaryDirectory() as directory, Directory(directory) as held:
            pin = write_snapshot(held, 'SOURCE_BEFORE.json', value)
            self.assertEqual(held.read(pin['path'], REPORT_LIMIT), encoded(value))
            self.assertEqual(pin, {'path': 'SOURCE_BEFORE.json',
                                  'bytes': len(encoded(value)), 'sha256': sha(encoded(value))})
            self.assertEqual(read_snapshot(held, pin['path']), value)
            self.assertEqual(sorted(p.name for p in Path(directory).iterdir()), [pin['path']])
            with self.assertRaises(FileExistsError):
                write_snapshot(held, pin['path'], value)

    def test_large_snapshot_retains_every_real_row_shape_and_full_canonical_hash(self):
        value = snapshot()
        raw = encoded(value)
        self.assertGreater(len(raw), REPORT_LIMIT)
        with tempfile.TemporaryDirectory() as directory, Directory(directory) as held:
            pin = write_snapshot(held, 'SOURCE_BEFORE.json', value)
            index = parse_json(held.read(pin['path'], REPORT_LIMIT))
            self.assertEqual(index['format'], 'zryna.m5.source-snapshot-shards.v1')
            self.assertEqual(index['snapshot'], {'bytes': len(raw), 'sha256': sha(raw),
                                                 'files': 6000})
            self.assertGreater(len(index['shards']), 1)
            rows = []
            for shard in index['shards']:
                data = held.read(shard['path'], REPORT_LIMIT)
                self.assertEqual((len(data), sha(data)), (shard['bytes'], shard['sha256']))
                part = parse_json(data)
                self.assertEqual(set(part), {'files'})
                self.assertEqual(len(part['files']), shard['files'])
                rows.extend(part['files'])
            self.assertEqual(rows, value['files'])
            self.assertEqual(read_snapshot(held, pin['path']), value)
            write_snapshot(held, 'SOURCE_AFTER.json', value)
            self.assertEqual(read_snapshot(held, 'SOURCE_AFTER.json'), value)
            self.assertEqual(value, snapshot(), 'capture cannot mutate source records')

    def test_missing_or_changed_shard_or_index_never_reconstructs_a_snapshot(self):
        for attack in ('missing', 'bytes', 'order', 'omit', 'repeat', 'count', 'fullhash'):
            with self.subTest(attack=attack), tempfile.TemporaryDirectory() as directory:
                with Directory(directory) as held:
                    write_snapshot(held, 'SOURCE_BEFORE.json', snapshot())
                    root = Path(directory) / 'SOURCE_BEFORE.json'
                    index = parse_json(root.read_bytes())
                    shard = Path(directory) / index['shards'][0]['path']
                    if attack == 'missing':
                        shard.unlink()
                    elif attack == 'bytes':
                        shard.write_bytes(shard.read_bytes().replace(b'module_', b'changed', 1))
                    else:
                        if attack == 'order':
                            index['shards'].reverse()
                        elif attack == 'omit':
                            index['shards'].pop()
                        elif attack == 'repeat':
                            index['shards'][1] = index['shards'][0]
                        elif attack == 'count':
                            index['snapshot']['files'] -= 1
                        else:
                            index['snapshot']['sha256'] = '0' * 64
                        root.write_bytes(encoded(index))
                    with self.assertRaises((AssertionError, FileNotFoundError)):
                        read_snapshot(held, 'SOURCE_BEFORE.json')

    def test_resealed_row_change_still_fails_full_snapshot_hash(self):
        with tempfile.TemporaryDirectory() as directory, Directory(directory) as held:
            write_snapshot(held, 'SOURCE_BEFORE.json', snapshot())
            root = Path(directory) / 'SOURCE_BEFORE.json'
            index = parse_json(root.read_bytes())
            pin = index['shards'][0]
            shard = Path(directory) / pin['path']
            value = parse_json(shard.read_bytes())
            value['files'][0]['sha256'] = 'f' * 64
            raw = encoded(value)
            shard.write_bytes(raw)
            pin.update(bytes=len(raw), sha256=sha(raw))
            root.write_bytes(encoded(index))
            with self.assertRaises(AssertionError):
                read_snapshot(held, 'SOURCE_BEFORE.json')

    def test_oversized_single_entry_fails_before_any_output_and_reports_remain_bounded(self):
        value = snapshot(1)
        value['files'][0]['path'] = 'x' * REPORT_LIMIT
        with tempfile.TemporaryDirectory() as directory, Directory(directory) as held:
            with self.assertRaises(AssertionError):
                write_snapshot(held, 'SOURCE_BEFORE.json', value)
            self.assertEqual(list(Path(directory).iterdir()), [])
            with self.assertRaises(AssertionError):
                write_json(held, 'REPORT.json', {'x': 'x' * REPORT_LIMIT}, compact=True)
            self.assertEqual(list(Path(directory).iterdir()), [])

    def test_duplicate_unsorted_or_malformed_source_rows_fail_before_creation(self):
        for attack in ('duplicate', 'unsorted', 'mode', 'blob', 'size', 'sha', 'extra'):
            with self.subTest(attack=attack), tempfile.TemporaryDirectory() as directory:
                value = snapshot(20)
                if attack == 'duplicate':
                    value['files'][1] = dict(value['files'][0])
                elif attack == 'unsorted':
                    value['files'][0], value['files'][1] = value['files'][1], value['files'][0]
                elif attack == 'mode':
                    value['files'][0]['mode'] = '120000'
                elif attack == 'blob':
                    value['files'][0]['blob'] = '0' * 39
                elif attack == 'size':
                    value['files'][0]['bytes'] = 2 * 1024 * 1024 + 1
                elif attack == 'sha':
                    value['files'][0]['sha256'] = 'x' * 64
                else:
                    value['files'][0]['passed'] = True
                with Directory(directory) as held:
                    with self.assertRaises(AssertionError):
                        write_snapshot(held, 'SOURCE_BEFORE.json', value)
                    self.assertEqual(list(Path(directory).iterdir()), [])


if __name__ == '__main__':
    unittest.main()
