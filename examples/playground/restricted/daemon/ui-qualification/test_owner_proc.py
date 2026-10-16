"""Full visible PPid enumeration fixtures; never assumes missing children files exist."""
import types
import unittest
from unittest import mock
import owner_proc as proc

MOUNT = '1 0 0:1 / /proc rw,nosuid,nodev,noexec - proc proc rw\n'


def record(pid=321, parent=1, state='S'):
    return f'{pid} (name ) with parentheses) {state} ' + ' '.join([str(parent)] + ['0'] * 17 + ['100']) + '\n'


class Entries:
    def __init__(self, names):
        self.names = names

    def __enter__(self):
        return (types.SimpleNamespace(name=str(name)) for name in self.names)

    def __exit__(self, *_):
        return False


class ProcTests(unittest.TestCase):
    def inventory(self, names, values):
        def read(path, deadline, maximum=16384):
            value = values[path]
            if isinstance(value, Exception):
                raise value
            return value
        with mock.patch.object(proc.os, 'scandir', return_value=Entries(names)), \
                mock.patch.object(proc.os, 'getpid', return_value=123), \
                mock.patch.object(proc.os, 'readlink', return_value='123'), \
                mock.patch.object(proc.time, 'monotonic', return_value=100):
            return proc.inventory(read, 101)

    def test_complete_numeric_inventory_includes_unreaped_zombies_and_ignores_metadata_names(self):
        values = {'/proc/123/mountinfo': MOUNT, '/proc/321/stat': record(),
                  '/proc/322/stat': record(322, 321, 'Z')}
        result = self.inventory(['self', 'net', 321, 322], values)
        self.assertEqual(result[322], {'pid': 322, 'parent': 321, 'ticks': '100', 'state': 'Z'})

    def test_only_actual_disappearance_is_skipped_never_denied_or_incomplete_records(self):
        values = {'/proc/123/mountinfo': MOUNT, '/proc/321/stat': FileNotFoundError()}
        self.assertEqual(self.inventory([321], values), {})
        for value in [PermissionError(), OSError(), '321 (truncated)', record(322), record(parent=4294967296)]:
            values['/proc/321/stat'] = value
            with self.assertRaises((ValueError, OSError)):
                self.inventory([321], values)

    def test_full_proc_mount_and_actual_self_namespace_required(self):
        for text in [MOUNT.replace('rw,nosuid', 'rw,hidepid=1,nosuid'),
                     MOUNT.replace('proc proc rw', 'proc proc rw,hidepid=2'),
                     MOUNT.replace(' / /proc ', ' /subtree /proc '), MOUNT + MOUNT, '']:
            with mock.patch.object(proc.os, 'readlink', return_value='123'), self.assertRaises(ValueError):
                proc.visible(text, 123)
        with mock.patch.object(proc.os, 'readlink', return_value='999'), self.assertRaises(ValueError):
            proc.visible(MOUNT, 123)

    def test_numeric_limit_applies_even_when_processes_disappear(self):
        values = {'/proc/123/mountinfo': MOUNT}
        values.update({f'/proc/{pid}/stat': FileNotFoundError() for pid in range(1, 8194)})
        with self.assertRaises(ValueError):
            self.inventory(range(1, 8194), values)
        with self.assertRaises(ValueError):
            self.inventory(['0001'], values)


if __name__ == '__main__':
    unittest.main()
