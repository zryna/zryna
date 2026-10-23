"""Private snapshot retention/path attacks confer no source or execution authority."""
from pathlib import Path
import json
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from capture_io import Directory, REPORT_LIMIT, SELECTED_LIMIT, sha
from source_snapshot import read_snapshot, write_snapshot
from scripts.m5.tests.test_source_snapshot_shards import encoded, snapshot


class RootOnly:
    def __init__(self, value):
        self.raw = encoded(value)
        self.calls = []

    def read(self, name, maximum):
        self.calls.append((name, maximum))
        assert len(self.raw) <= maximum
        assert name == 'SOURCE_BEFORE.json', 'metadata must reject before any shard read'
        return self.raw


def index():
    return {'format': 'zryna.m5.source-snapshot-shards.v1',
            'header': {'candidate': 'a' * 40, 'checkoutCommit': 'b' * 40,
                       'tree': 'c' * 40, 'clean': True},
            'snapshot': {'bytes': REPORT_LIMIT + 1, 'sha256': '0' * 64, 'files': 1},
            'shards': [{'path': 'SOURCE_BEFORE.files-00000.json', 'bytes': 200,
                        'sha256': '0' * 64, 'files': 1}]}


class SourceSnapshotShardBounds(unittest.TestCase):
    def test_untrusted_names_counts_and_aggregate_reject_before_shard_open(self):
        for attack in ('escape', 'alias', 'boolbytes', 'boolcount', 'boolfull',
                       'aggregate', 'snapshotextra', 'countmismatch', 'extrakey'):
            with self.subTest(attack=attack):
                value = index()
                pin = value['shards'][0]
                if attack == 'escape':
                    pin['path'] = '../SOURCE_BEFORE.files-00000.json'
                elif attack == 'alias':
                    pin['path'] = 'SOURCE_AFTER.files-00000.json'
                elif attack == 'boolbytes':
                    pin['bytes'] = True
                elif attack == 'boolcount':
                    pin['files'] = True
                elif attack == 'boolfull':
                    value['snapshot']['files'] = True
                elif attack == 'aggregate':
                    value['snapshot'].update(bytes=SELECTED_LIMIT, files=97)
                    value['shards'] = [dict(pin, path=f'SOURCE_BEFORE.files-{i:05d}.json',
                                            bytes=REPORT_LIMIT) for i in range(97)]
                elif attack == 'snapshotextra':
                    value['snapshot']['bytes'] = SELECTED_LIMIT + 1
                elif attack == 'countmismatch':
                    value['snapshot']['files'] = 2
                else:
                    pin['verified'] = True
                held = RootOnly(value)
                with self.assertRaises(AssertionError):
                    read_snapshot(held, 'SOURCE_BEFORE.json')
                self.assertEqual(held.calls, [('SOURCE_BEFORE.json', REPORT_LIMIT)])

    def test_shard_symlink_is_rejected_even_when_target_bytes_match_descriptor(self):
        with tempfile.TemporaryDirectory() as directory, Directory(directory) as held:
            write_snapshot(held, 'SOURCE_BEFORE.json', snapshot())
            root = Path(directory) / 'SOURCE_BEFORE.json'
            value = json.loads(root.read_bytes())
            path = Path(directory) / value['shards'][0]['path']
            data = path.read_bytes()
            self.assertEqual(sha(data), value['shards'][0]['sha256'])
            target = Path(directory) / 'saved-shard.json'
            path.rename(target)
            path.symlink_to(target.name)
            with self.assertRaises(AssertionError):
                read_snapshot(held, 'SOURCE_BEFORE.json')

    def test_shard_collision_is_detected_before_any_new_output(self):
        with tempfile.TemporaryDirectory() as directory, Directory(directory) as held:
            name = 'SOURCE_BEFORE.files-00001.json'
            held.write(name, b'preserved diagnostic data')
            with self.assertRaises(FileExistsError):
                write_snapshot(held, 'SOURCE_BEFORE.json', snapshot())
            self.assertEqual(sorted(p.name for p in Path(directory).iterdir()), [name])
            self.assertEqual(held.read(name, REPORT_LIMIT), b'preserved diagnostic data')


if __name__ == '__main__':
    unittest.main()
