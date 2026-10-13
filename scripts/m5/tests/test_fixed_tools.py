import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import fixed_tools
from capture_io import sha
from capture_providers import tool_identity


class FixedToolSelectionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.files, self.pins, self.seen = [], [], []
        for index, alias in enumerate(fixed_tools.ALIASES):
            file = Path(self.temporary.name) / ('tool-' + str(index))
            data = b'\x7fELFtest tool data' + bytes([index])
            file.write_bytes(data)
            self.files.append(file)
            self.pins.append({'path': alias, 'bytes': len(data),
                'sha256': sha(data), 'identity': tool_identity(file.stat())})

    def selected_from_fixed_alias(self, alias, roots):
        self.assertIn(alias, fixed_tools.ALIASES)
        self.seen.append(alias)
        file = self.files[fixed_tools.ALIASES.index(alias)]
        return os.open(file, os.O_RDONLY), alias, [{'kind': 'test-only-fixed-alias'}]

    def test_only_three_source_fixed_aliases_are_resolved(self):
        with patch.object(fixed_tools, 'walk', self.selected_from_fixed_alias):
            result = fixed_tools.select(self.pins)
        self.assertEqual(self.seen, list(fixed_tools.ALIASES))
        self.assertEqual([x['sourceAlias'] for x in result], list(fixed_tools.ALIASES))

    def test_forged_report_path_cannot_nominate_executable(self):
        self.pins[0]['path'] = '/tmp/forged-executable'
        with patch.object(fixed_tools, 'walk', self.selected_from_fixed_alias), self.assertRaises(AssertionError):
            fixed_tools.select(self.pins)
        self.assertEqual(self.seen, ['/usr/bin/gcc'])

    def test_substituted_tool_bytes_or_epoch_reject(self):
        for field, bad in [('sha256', '0' * 64), ('identity', 'invented-epoch'), ('bytes', 1)]:
            original = self.pins[0][field]
            self.pins[0][field] = bad
            with patch.object(fixed_tools, 'walk', self.selected_from_fixed_alias), self.assertRaises(AssertionError):
                fixed_tools.select(self.pins)
            self.pins[0][field] = original


if __name__ == '__main__':
    unittest.main()
