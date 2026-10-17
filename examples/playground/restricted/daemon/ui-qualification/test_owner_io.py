"""Escaping-I/O diagnostics only: complete synthetic epochs, no Engine or browser."""
import errno
import json
import stat
import types
import unittest
from unittest import mock
import owner_kernel as kernel
import owner_cgroup as cgroup
import test_owner_acquisition as acquisition


def last_membership_failure(operation):
    case = acquisition.AcquisitionTests('runTest')
    case.setUp()
    case.retire = 999
    count = 0
    original_read = kernel.read
    error = FileNotFoundError(errno.ENOENT, 'private fixture text must not escape')
    def observe(path, deadline, maximum=16384):
        nonlocal count
        if path == '/proc/323/cgroup':
            count += 1
            if count == 2:
                with mock.patch.object(kernel.os, 'open', return_value=91), \
                        mock.patch.object(kernel.os, 'fstat', side_effect=error if operation == 'FSTAT' else None,
                                          return_value=types.SimpleNamespace(st_mode=stat.S_IFREG)), \
                        mock.patch.object(kernel.os, 'read', side_effect=error), \
                        mock.patch.object(kernel.os, 'close') as close:
                    try:
                        return original_read(path, deadline, maximum)
                    finally:
                        assert close.call_count == 1
        return case.observe(path, deadline, maximum)
    try:
        case.run_capture(change=observe)
    except FileNotFoundError as observed:
        assert observed is error
    else:
        raise AssertionError('mandatory read must fail')
    assert count == 2 and case.inspect_calls == 1 and len(case.record['attempts']) == 1
    return case.record


class IoDiagnosticTests(unittest.TestCase):
    def test_filename_none_fstat_read_projects_actual_last_membership_and_original_exception(self):
        for operation in ('FSTAT', 'READ'):
            with self.subTest(operation=operation):
                phase = last_membership_failure(operation)
                row = phase['attempts'][0]
                self.assertEqual(phase['status'], 'failed')
                self.assertEqual((row['status'], row['task'], row['failure'], row['after']), ('fatal', None, None, None))
                self.assertEqual(row['ioFailure'], {'errno': 2, 'operation': operation, 'kind': 'PROC',
                    'field': 'CGROUP', 'subject': 'OWNED_TID', 'pid': 323, 'site': 'TREE_LAST_MEMBERSHIPS'})
                self.assertNotIn('private fixture', json.dumps(phase))
                self.assertIsNone(cgroup._ACTIVE)

    def test_actual_open_close_errno_and_original_exception_annotations(self):
        for operation in ('OPEN', 'CLOSE'):
            error = PermissionError(errno.EACCES, 'private path fixture', '/proc/323/status')
            with mock.patch.object(kernel.time, 'monotonic', return_value=100), \
                    mock.patch.object(kernel.os, 'open', side_effect=error if operation == 'OPEN' else None, return_value=91), \
                    mock.patch.object(kernel.os, 'fstat', return_value=types.SimpleNamespace(st_mode=stat.S_IFREG)), \
                    mock.patch.object(kernel.os, 'read', return_value=b''), \
                    mock.patch.object(kernel.os, 'close', side_effect=error if operation == 'CLOSE' else None) as close:
                with self.assertRaises(PermissionError) as observed:
                    kernel.read('/proc/323/status', 101)
            self.assertIs(observed.exception, error)
            self.assertEqual(error._ui_kernel_io, (operation, '/proc/323/status'))
            self.assertEqual(close.call_count, 0 if operation == 'OPEN' else 1)

    def candidate(self):
        candidate = cgroup._Candidate()
        candidate.original = {'pid': 321}
        candidate.tree = {'group': '/sys/fs/cgroup/docker/' + 'a' * 64,
                          'processes': [321, 322], 'threads': [321, 322, 323]}
        candidate.site = 'TREE_FINAL_GROUP'
        return candidate

    def test_closed_fields_captured_roles_and_no_raw_path_or_foreign_pid(self):
        candidate = self.candidate()
        cases = [('/proc/321/status', 'PROC', 'STATUS', 'INIT', 321),
                 ('/proc/322/stat', 'PROC', 'STAT', 'OWNED_PID', 322),
                 ('/proc/323/cgroup', 'PROC', 'CGROUP', 'OWNED_TID', 323),
                 ('/proc/999/cgroup', 'PROC', 'CGROUP', 'UNKNOWN', None),
                 (candidate.tree['group'] + '/pids.current', 'CGROUP', 'PIDS_CURRENT', 'UNKNOWN', None),
                 ('/proc/323/secret-capability', 'UNKNOWN', 'UNKNOWN', 'UNKNOWN', None),
                 ('/proc/323/status?secret', 'UNKNOWN', 'UNKNOWN', 'UNKNOWN', None),
                 ('/proc/4294967296/status', 'UNKNOWN', 'UNKNOWN', 'UNKNOWN', None),
                 ('/sys/fs/cgroup/foreign/pids.current', 'UNKNOWN', 'UNKNOWN', 'UNKNOWN', None)]
        for path, kind, field, subject, pid in cases:
            error = OSError(errno.EIO, 'private raw text', path)
            value = cgroup.io_failure(error, candidate)
            self.assertEqual((value['kind'], value['field'], value['subject'], value['pid']), (kind, field, subject, pid))
            self.assertEqual(value['operation'], 'UNKNOWN')
            self.assertNotIn(path, json.dumps(value))
            self.assertNotIn('private raw text', json.dumps(value))
        error = OSError(True, 'private')
        candidate.site = 'arbitrary private string'
        self.assertIsNone(cgroup.io_failure(error, candidate)['errno'])
        self.assertEqual(cgroup.io_failure(error, candidate)['site'], 'UNKNOWN')

    def test_diagnostic_projection_failure_cannot_replace_error_or_retry(self):
        with mock.patch.object(cgroup, 'io_failure', side_effect=RuntimeError('diagnostic failure')):
            phase = last_membership_failure('READ')
        self.assertEqual(phase['status'], 'failed')
        self.assertNotIn('ioFailure', phase['attempts'][0])

    def test_non_os_failure_and_success_do_not_gain_io_diagnostics(self):
        case = acquisition.AcquisitionTests('runTest');case.setUp();case.retire = 999
        case.run_capture()
        self.assertNotIn('ioFailure', case.record['attempts'][0])
        case = acquisition.AcquisitionTests('runTest');case.setUp();case.retire = 999
        error = ValueError('mandatory security failure')
        def broken(path, deadline, maximum=16384):
            if path == '/proc/323/cgroup':
                raise error
            return case.observe(path, deadline, maximum)
        with self.assertRaises(ValueError) as observed:
            case.run_capture(change=broken)
        self.assertIs(observed.exception, error)
        self.assertNotIn('ioFailure', case.record['attempts'][0])


if __name__ == '__main__':
    unittest.main()
