"""Original failed-run topology with explicitly synthetic retained-handle metadata."""
import copy
import json
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch
from owner_test_fixture import OWNER

MOUNTS = OWNER._load('lifecycle_mounts')
FIXTURE = json.loads((Path(__file__).parent / 'native_mount_fixture.json').read_text())
CID = FIXTURE['cid']


class Sources:
    def __init__(self):
        paths = ['/', '/var', '/var/lib', '/var/lib/docker', '/var/lib/docker/containers',
                 '/var/lib/docker/containers/' + CID]
        self.directories = {path: (index + 1, None) for index, path in enumerate(paths)}
        self.metadata = {fd: self.state(fd, 0o40755) for fd, _ in self.directories.values()}
        self.opened, self.finished, self.closed, self.revalidated, self.changed = [], [], False, False, False

    def state(self, inode, mode):
        return SimpleNamespace(st_dev=2049, st_ino=inode, st_uid=0, st_gid=0,
                               st_mode=mode, st_nlink=1, st_mtime_ns=123, st_ctime_ns=456)

    def open(self, path):
        fd = len(self.metadata) + 1
        self.metadata[fd] = self.state(fd, 0o100644)
        self.opened.append(path)
        return fd, path

    def finish(self, path, fd, before):
        if self.changed:
            raise ValueError('synthetic named identity changed')
        assert path == before
        self.finished.append(path)

    def revalidate(self):
        self.revalidated = True

    def close(self):
        self.closed = True


class MountTests(unittest.TestCase):
    def resolve(self, host=None, container=None, fault=None):
        sources = Sources()
        def stated(fd):
            value = copy.copy(sources.metadata[fd])
            if fault:
                fault(fd, value, sources)
            return value
        with patch.object(MOUNTS, 'Sources', return_value=sources), \
                patch.object(MOUNTS.os, 'fstat', side_effect=stated), \
                patch.object(MOUNTS.os, 'close'), \
                patch.object(MOUNTS.os, 'major', side_effect=lambda device: 8 if device == 2049 else 9, create=True), \
                patch.object(MOUNTS.os, 'minor', return_value=1, create=True):
            try:
                result = MOUNTS.resolve(container or FIXTURE['container'], CID, host or FIXTURE['host'], '/var/lib/docker')
            finally:
                if sources.opened:
                    self.assertTrue(sources.closed)
        self.assertEqual(sources.opened, sources.finished)
        self.assertEqual(len(sources.finished), 3)
        self.assertTrue(sources.revalidated)
        return result

    def test_actual_ext4_topology_retains_original_bases_and_null_masks(self):
        with self.assertRaisesRegex(ValueError, 'same-dev null'):
            MOUNTS.original_resolve(FIXTURE['container'], CID)
        result = self.resolve()
        for name in MOUNTS.FILES:
            row = result['/etc/' + name]
            self.assertEqual(row['stack']['base']['filesystem'], 'ext4')
            self.assertEqual(row['stack']['base']['source'], '/dev/root')
            self.assertEqual(row['stack']['base']['root'], '/var/lib/docker/containers/' + CID + '/' + name)
            self.assertEqual(row['root'], '/null')
            self.assertEqual(row['device'], result['/dev']['device'])
            self.assertEqual(len(row['nativeBacking']['metadata']), 9)

    def test_native_root_device_source_and_covering_bind_are_required(self):
        root, rest = FIXTURE['host'].split('\n', 1)
        hosts = [root.replace(' - ext4 /dev/root ', ' - xfs /dev/root ') + '\n' + rest,
                 root.replace('/dev/root', '/dev/other') + '\n' + rest,
                 root.replace('8:1', '9:1') + '\n' + rest,
                 FIXTURE['host'] + '999 28 8:1 /var/lib/docker /var/lib/docker rw - ext4 /dev/root rw\n']
        for host in hosts:
            with self.subTest(host=host[:80]), self.assertRaises(ValueError):
                self.resolve(host=host)

    def test_foreign_cid_missing_masks_and_swapped_null_device_refuse(self):
        containers = [FIXTURE['container'].replace('/containers/' + CID, '/containers/' + 'c' * 64),
                      '\n'.join(line for line in FIXTURE['container'].splitlines() if '/null /etc/hostname' not in line) + '\n',
                      FIXTURE['container'].replace('0:64 /null /etc/hostname', '0:66 /null /etc/hostname')]
        for container in containers:
            with self.subTest(container=container[:80]), self.assertRaises(ValueError):
                self.resolve(container=container)

    def test_unsafe_or_changed_backing_never_produces_admission(self):
        faults = [lambda fd, value, sources: setattr(value, 'st_dev', 2050),
                  lambda fd, value, sources: setattr(value, 'st_uid', 1),
                  lambda fd, value, sources: setattr(value, 'st_mode', 0o120777),
                  lambda fd, value, sources: setattr(value, 'st_mode', 0o100664),
                  lambda fd, value, sources: setattr(value, 'st_nlink', 2) if fd > 6 else None,
                  lambda fd, value, sources: setattr(sources, 'changed', True)]
        for fault in faults:
            with self.subTest(fault=fault), self.assertRaises(ValueError):
                self.resolve(fault=fault)


if __name__ == '__main__':
    unittest.main()
