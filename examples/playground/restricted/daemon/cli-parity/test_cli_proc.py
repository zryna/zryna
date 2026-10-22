"""Complete proc-scan failure semantics, including unreaped zombies and denied foreign reads."""
import time
import unittest
from unittest import mock
import cli_proc


def stat(pid, parent, state='S', ticks='123'):
    return str(pid) + ' (fixture (nested) name) ' + ' '.join([state, str(parent), *(['0'] * 17), ticks])


class Entries:
    def __init__(self, names):
        self.names = names
    def __enter__(self):
        return iter([type('Entry', (), {'name': str(value)})() for value in self.names])
    def __exit__(self, *_):
        return False


class ProcTests(unittest.TestCase):
    def scan(self, names, records, mount='1 0 0:1 / /proc rw - proc proc rw\n'):
        def read(path, deadline, maximum=16384):
            if path == '/proc/self/mountinfo':
                return mount
            pid = int(path.split('/')[2])
            value = records[pid]
            if isinstance(value, BaseException):
                raise value
            return value
        with mock.patch.object(cli_proc.os, 'readlink', return_value=str(cli_proc.os.getpid())), \
                mock.patch.object(cli_proc.os, 'scandir', return_value=Entries(names)):
            return cli_proc.children_reaped(100, '123', time.monotonic() + 1, read)

    def test_zombie_is_found_even_when_live_cgroup_task_count_would_be_one(self):
        value = self.scan(['self', 100, 200], {100: stat(100, 1), 200: stat(200, 100, 'Z')})
        self.assertFalse(value['childrenReaped']); self.assertEqual(value['children'][0]['state'], 'Z')

    def test_only_enoent_may_disappear_and_foreign_permission_or_malformed_stat_is_fatal(self):
        value = self.scan([100, 200], {100: stat(100, 1), 200: FileNotFoundError()})
        self.assertTrue(value['childrenReaped']); self.assertTrue(value['noDeniedReadsSkipped'])
        for other in [PermissionError(), OSError(), 'malformed', stat(201, 1)]:
            with self.assertRaises((ValueError, OSError)):
                self.scan([100, 200], {100: stat(100, 1), 200: other})

    def test_full_proc_mount_and_fixed_observer_identity_are_required(self):
        for mount in ['', '1 0 0:1 /sub /proc rw - proc proc rw\n',
                      '1 0 0:1 / /proc rw - proc proc rw,hidepid=2\n']:
            with self.assertRaises(ValueError):
                self.scan([100], {100: stat(100, 1)}, mount)
        self.assertEqual(cli_proc.process(stat(100, 1, 'Z'), 100), (1, 'Z', '123'))


if __name__ == '__main__':
    unittest.main()
