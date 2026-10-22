"""Synthetic exact observed mount coordinates, never generic '..' normalization."""
import unittest
from unittest import mock
import owner_cgroup as cgroup
from owner_kernel import numbers

CID = 'a' * 64
MOUNT = '136 135 0:24 /.. /sys/fs/cgroup ro,nosuid,nodev,noexec,relatime master:8 - cgroup2 cgroup2 rw,nsdelegate\n'


class CgroupTests(unittest.TestCase):
    def mapping(self, target=None, mount=MOUNT, own='0::/\n', procs='123\n'):
        values = {'/proc/123/mountinfo': mount, '/proc/123/cgroup': own,
                  '/sys/fs/cgroup/init/cgroup.procs': procs, '/sys/fs/cgroup/owner/init/cgroup.procs': procs}
        def read(path, deadline, maximum=16384):
            return values[path]
        with mock.patch.object(cgroup.os, 'getpid', return_value=123):
            return cgroup.mapping(read, numbers, target or '0::/../docker/' + CID + '\n', CID, 100)

    def test_exact_managed_mount_root_and_observer_pid_map_only_one_literal_sibling(self):
        result = self.mapping()
        self.assertEqual(result['path'], '/sys/fs/cgroup/docker/' + CID)
        self.assertEqual(result['profile'], 'managed-init-sibling-docker-v1')
        self.assertEqual(result['observerMembership'], '0::/\n')
        self.assertEqual(result['observerProcs'], [123])
        self.assertEqual(result['mount']['root'], '/..')

    def test_no_extra_parent_steps_wrong_id_wrong_observer_or_unapproved_mount_root(self):
        for target in ['0::/../../docker/' + CID + '\n', '0::/../other/' + CID + '\n',
                       '0::/../docker/' + 'b' * 64 + '\n', '0::/../docker/' + CID + '/extra\n',
                       '0::/docker/' + CID + '\n']:
            with self.assertRaises(ValueError):
                self.mapping(target=target)
        for changed in [{'own': '0::/init\n'}, {'procs': '999\n'},
                        {'mount': MOUNT.replace(' /.. ', ' /../.. ')}, {'mount': MOUNT + MOUNT},
                        {'mount': MOUNT.replace(' ro,nosuid', ' rw,nosuid')},
                        {'mount': MOUNT.replace(' - cgroup2 ', ' - tmpfs ')}]:
            with self.assertRaises(ValueError):
                self.mapping(**changed)

    def test_canonical_full_root_profile_stays_supported_without_any_parent_traversal(self):
        result = self.mapping(target='0::/owner/docker/' + CID + '\n', own='0::/owner/init\n',
                              mount=MOUNT.replace(' /.. ', ' / '))
        self.assertEqual(result['path'], '/sys/fs/cgroup/owner/docker/' + CID)
        self.assertEqual(result['profile'], 'full-root-canonical-docker-v1')
        with self.assertRaises(ValueError):
            self.mapping(target='0::/owner/../docker/' + CID + '\n', own='0::/owner/init\n',
                         mount=MOUNT.replace(' /.. ', ' / '))


if __name__ == '__main__':
    unittest.main()
