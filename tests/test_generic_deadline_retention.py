"""Inert filesystem controls for the fixed failure-original collector."""
import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import stat
import sys
import tempfile
import unittest
from unittest.mock import patch


@unittest.skipUnless(sys.platform == 'linux', 'Linux failure-original collector')
class RetentionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='deadline-copy-unit-')
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name)
        self.scope = self.base / 'tmp'
        self.scope.mkdir()
        source = Path(__file__).resolve().parents[1] / 'scripts/retain-generic-deadline-failures.py'
        spec = importlib.util.spec_from_file_location('fixed_failure_copy', source)
        self.owner = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.owner)
        # The production root stays fixed; unit controls never scan the real /tmp.
        self.owner.Path = lambda *parts: self.scope if parts == ('/tmp',) else Path(*parts)

    def case(self, parent=None, case='plain-and-reversed-arguments', family='nested-struct-v4'):
        directory = (parent or self.scope) / f'm416-{family}-driver-12-{case}-345'
        directory.mkdir()
        return directory

    def collect(self):
        output = self.base / 'retained'
        with contextlib.redirect_stdout(io.StringIO()):
            result = self.owner.collect(output)
        return result, output, json.loads((output / 'manifest.json').read_bytes())

    def test_exact_bytes_hashes_modes_and_one_m4_level(self):
        direct = self.case()
        data = b'original inert JavaScript bytes; never executed\n'
        original = direct / 'program.mjs'
        original.write_bytes(data)
        before = original.stat()
        m4 = self.scope / 'z4-Ab12c3'
        m4.mkdir()
        clone = self.case(m4, 'copy-struct', 'struct-clone-v5')
        (clone / 'program.zir').write_bytes(b'original wire')
        (self.scope / 'm416-process-deadline-12-34.json').write_bytes(b'{}')
        # Further roots are outside the explicit one-level selection.
        deeper = m4 / 'z4-Zz11yy'
        deeper.mkdir()
        (self.case(deeper) / 'program.mjs').write_bytes(b'outside selection')
        result, output, report = self.collect()
        self.assertEqual(result, 0)
        self.assertEqual(report['status'], 'complete-for-fixed-selection')
        self.assertEqual(len(report['directories']), 2)
        self.assertEqual(len(report['files']), 3)
        copied = output / 'tmp' / direct.name / 'program.mjs'
        self.assertEqual(copied.read_bytes(), data)
        self.assertEqual(stat.S_IMODE(copied.stat().st_mode), 0o600)
        row = next(row for row in report['files'] if row['source'] == str(original))
        self.assertEqual(row['sha256'], hashlib.sha256(data).hexdigest())
        self.assertEqual(self.owner.identity(before), self.owner.identity(original.stat()))
        self.assertFalse(report['globalM7'])

    def test_links_special_files_and_unknown_names_are_incomplete(self):
        directory = self.case()
        outside = self.base / 'sentinel'
        outside.write_bytes(b'outside original selection')
        (directory / 'program.mjs').symlink_to(outside)
        os.link(outside, directory / 'main.zry')
        os.mkfifo(directory / 'program.wasm')
        (directory / 'unknown.bin').write_bytes(b'unknown')
        (self.scope / 'm416-process-deadline-12-35.json').symlink_to(outside)
        alias = self.scope / 'm416-nested-struct-v4-driver-13-nested-copy-346'
        alias.symlink_to(directory, target_is_directory=True)
        (self.scope / 'z4-Xy11aa').symlink_to(directory, target_is_directory=True)
        result, output, report = self.collect()
        self.assertEqual(result, 1)
        self.assertEqual(report['status'], 'incomplete')
        self.assertGreaterEqual(len(report['issues']), 7)
        self.assertEqual(report['bytes'], 0)
        self.assertFalse((output / 'tmp' / directory.name / 'program.mjs').exists())
        self.assertEqual(outside.read_bytes(), b'outside original selection')

    def test_file_first_extra_and_total_byte_caps(self):
        directory = self.case()
        (directory / 'program.mjs').write_bytes(b'abcd')
        (directory / 'main.zry').write_bytes(b'xyz')
        self.owner.FILE_BYTES = 3
        self.owner.TOTAL_BYTES = 2
        result, _, report = self.collect()
        self.assertEqual(result, 1)
        self.assertEqual(report['bytes'], 0)
        self.assertEqual(len(report['issues']), 2)
        self.assertEqual((directory / 'program.mjs').read_bytes(), b'abcd')

    def test_snapshot_first_extra_is_recorded(self):
        for number in range(1, 34):
            (self.scope / f'm416-process-deadline-12-{number}.json').write_bytes(b'{}')
        result, _, report = self.collect()
        self.assertEqual(result, 1)
        self.assertEqual(len(report['files']), 32)
        self.assertTrue(any('snapshot first extra' in row['reason'] for row in report['issues']))

    def test_change_between_stat_and_open_is_not_copied(self):
        original = self.case() / 'program.mjs'
        original.write_bytes(b'before')
        real_open = os.open

        def replacing_open(path, flags, *args, **kwargs):
            if path == 'program.mjs' and flags & os.O_NOFOLLOW:
                original.write_bytes(b'changed after initial stat')
            return real_open(path, flags, *args, **kwargs)

        with patch.object(self.owner.os, 'open', side_effect=replacing_open):
            result, output, report = self.collect()
        self.assertEqual(result, 1)
        self.assertTrue(any('replaced before read' in row['reason'] for row in report['issues']))
        self.assertFalse((output / 'tmp' / original.parent.name / original.name).exists())


if __name__ == '__main__':
    unittest.main()
