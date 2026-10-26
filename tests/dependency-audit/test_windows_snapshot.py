"""Cross-stat semantic mocks and genuine file reads only; no executable/SDK is launched."""
import copy
import hashlib
from pathlib import Path
import stat
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'scripts/stability-gates'))
import dependency_audit_inputs as inputs

PAYLOAD = b'DO_NOT_INCLUDE_FILE_BYTES_IN_DIAGNOSTICS'


def facts(mode=stat.S_IFREG | 0o666, ctime=100, birth=100):
    return SimpleNamespace(st_dev=1, st_ino=2, st_size=len(PAYLOAD), st_mode=mode,
        st_nlink=1, st_mtime_ns=50, st_ctime_ns=ctime, st_birthtime_ns=birth)


class WindowsSnapshotCompatibility(unittest.TestCase):
    def capture(self, named, opened, named_after=None, fd_after=None, suffix='.exe', platform='win32'):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / ('payload' + suffix); path.write_bytes(PAYLOAD)
            with patch.object(Path, 'is_symlink', return_value=False), \
                 patch.object(Path, 'lstat', side_effect=[named, named_after or named]), \
                 patch.object(inputs.os, 'fstat', side_effect=[opened, fd_after or opened]), \
                 patch.object(sys, 'platform', platform):
                return inputs.snapshot(path)

    def test_documented_windows_name_execute_bits_only(self):
        for suffix in ['.exe', '.EXE', '.bat', '.cmd', '.com']:
            result = self.capture(facts(stat.S_IFREG | 0o777), facts(), suffix=suffix)
            self.assertEqual(result['bytes'], PAYLOAD)
            self.assertEqual(result['identity'][3], stat.S_IFREG | 0o777)

    def test_windows_creation_and_change_ctime_keep_matching_birth_semantics(self):
        result = self.capture(facts(stat.S_IFREG | 0o777, ctime=100), facts(ctime=200))
        self.assertEqual(result['identity'][6:], (100, 100))
        self.assertEqual(self.capture(facts(ctime=100), facts(ctime=200), suffix='.txt')['bytes'], PAYLOAD)

    def test_cross_common_identity_or_birth_replacement_rejects(self):
        for field in ['st_dev', 'st_ino', 'st_size', 'st_nlink', 'st_mtime_ns', 'st_birthtime_ns']:
            opened = facts(); setattr(opened, field, getattr(opened, field) + 1)
            with self.assertRaises(inputs.AuditError, msg=field): self.capture(facts(), opened)

    def test_missing_nonpositive_or_bool_windows_birth_rejects(self):
        for value in [None, 0, -1, True]:
            with self.assertRaises(inputs.AuditError): self.capture(facts(), facts(birth=value))
        opened = facts(); del opened.st_birthtime_ns
        with self.assertRaises(inputs.AuditError): self.capture(facts(), opened)

    def test_readwrite_filetype_and_unknown_mode_differences_reject(self):
        for mode in [stat.S_IFREG | 0o444, stat.S_IFDIR | 0o666,
                     stat.S_IFREG | 0o6766, stat.S_IFREG | 0o766]:
            with self.assertRaises(inputs.AuditError): self.capture(facts(), facts(mode))

    def test_name_execute_exception_is_not_global_mode_mask(self):
        for suffix in ['.txt', '.dll', '.exe.txt', '']:
            with self.assertRaises(inputs.AuditError):
                self.capture(facts(stat.S_IFREG | 0o777), facts(), suffix=suffix)
        with self.assertRaises(inputs.AuditError):
            self.capture(facts(stat.S_IFREG | 0o777), facts(stat.S_IFREG | 0o667))

    def test_named_full_identity_still_rejects_ctime_birth_or_mode_changes(self):
        for field, value in [('st_ctime_ns', 101), ('st_birthtime_ns', 101), ('st_mode', stat.S_IFREG | 0o766)]:
            named = facts(stat.S_IFREG | 0o777); after = copy.copy(named); setattr(after, field, value)
            with self.assertRaises(inputs.AuditError): self.capture(named, facts(ctime=200), named_after=after)

    def test_fd_full_identity_still_rejects_ctime_birth_or_mode_changes(self):
        for field, value in [('st_ctime_ns', 201), ('st_birthtime_ns', 101), ('st_mode', stat.S_IFREG | 0o444)]:
            opened = facts(ctime=200); after = copy.copy(opened); setattr(after, field, value)
            with self.assertRaises(inputs.AuditError):
                self.capture(facts(stat.S_IFREG | 0o777), opened, fd_after=after)

    def test_linux_seven_field_full_identity_stays_strict(self):
        self.assertEqual(len(self.capture(facts(), facts(), platform='linux')['identity']), 7)
        for field in ['st_dev', 'st_ino', 'st_size', 'st_nlink', 'st_mtime_ns', 'st_ctime_ns', 'st_mode']:
            opened = facts(); setattr(opened, field, getattr(opened, field) + 1)
            with self.assertRaises(inputs.AuditError): self.capture(facts(), opened, platform='linux')

    def test_cross_failure_has_bounded_fields_path_and_no_file_bytes(self):
        opened = facts(); opened.st_ino = 999
        with self.assertRaises(inputs.AuditError) as caught: self.capture(facts(), opened)
        text = str(caught.exception)
        self.assertIn('payload.exe', text); self.assertIn('st_ino', text)
        self.assertIn('999', text); self.assertNotIn(PAYLOAD.decode(), text)
        self.assertLess(len(text), 4096)

    def test_real_exe_named_regular_file_read_without_execution(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'regular-bytes.exe'; path.write_bytes(PAYLOAD)
            captured = inputs.snapshot(path)
            self.assertEqual(captured['bytes'], PAYLOAD)
            self.assertEqual(captured['sha256'], hashlib.sha256(PAYLOAD).hexdigest())

    def test_real_current_python_executable_read_without_execution(self):
        path = Path(sys.executable).resolve()
        captured = inputs.snapshot(path, 256 * 1024 * 1024)
        self.assertEqual(captured['path'], str(path))
        self.assertEqual(captured['sha256'], hashlib.sha256(captured['bytes']).hexdigest())


if __name__ == '__main__':
    unittest.main()
