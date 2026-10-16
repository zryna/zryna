"""Synthetic kernel policy negatives; no real process/container/cgroup qualification credit."""
import copy
import time
import unittest
from unittest import mock
import cli_kernel as kernel

CID = 'b' * 64
GROUP = '/sys/fs/cgroup/docker/' + CID
STATUS = '\n'.join(['Pid:\t321', 'Threads:\t1', 'Uid:\t65534 65534 65534 65534',
    'Gid:\t65534 65534 65534 65534', 'NoNewPrivs:\t1', 'Seccomp:\t2', 'Cpus_allowed_list:\t0-1',
    'NSpid:\t321 1', *[name + ':\t0000000000000000' for name in
                     ['CapInh', 'CapPrm', 'CapEff', 'CapBnd', 'CapAmb']]]) + '\n'


def mountinfo():
    rows = ['1 0 0:1 / / ro - overlay overlay ro',
            '2 1 0:2 /docker/' + CID + ' /sys/fs/cgroup ro - cgroup2 cgroup2 rw']
    for index, (path, size) in enumerate([('/work/project', 2048), ('/input', 2048), ('/output', 2048),
                                        ('/tmp', 65536), ('/dev/shm', 1024), ('/dev', 1024)], 3):
        options = 'rw,nosuid,noexec' if path == '/dev' else 'rw,nosuid,nodev,noexec'
        rows.append(f'{index} 1 0:{index} / {path} {options} - tmpfs tmpfs rw,size={size}k')
    rows.append('9 1 0:9 / /proc ro,nosuid,nodev,noexec - proc proc rw')
    return '\n'.join(rows) + '\n'


class KernelTests(unittest.TestCase):
    def capture(self, changes=None):
        values = {'/proc/321/status': STATUS, '/proc/321/cgroup': '0::/docker/' + CID + '\n',
            '/proc/321/mountinfo': mountinfo(), GROUP + '/memory.max': '536870912\n',
            GROUP + '/memory.swap.max': '0\n', GROUP + '/pids.max': '32\n',
            GROUP + '/cpu.max': '200000 100000\n', GROUP + '/cpuset.cpus.effective': '0-1\n',
            GROUP + '/cgroup.procs': '321\n', GROUP + '/cgroup.threads': '321\n', GROUP + '/pids.current': '1\n'}
        values.update(changes or {})
        state = {'Id': CID, 'RestartCount': 0, 'State': {'Pid': 321, 'Running': True,
            'Paused': False, 'Restarting': False, 'OOMKilled': False, 'StartedAt': 'fixed-start'}}
        with mock.patch.object(kernel, 'read', side_effect=lambda path, *_: values[path]), \
             mock.patch.object(kernel, 'ticks', return_value='100'), \
             mock.patch.object(kernel.cli_cgroup, 'mapping', return_value={'path': GROUP, 'profile': 'fixture'}), \
             mock.patch.object(kernel, 'namespace_links', return_value={}), \
             mock.patch.object(kernel, 'network', return_value={'synthetic': True}), \
             mock.patch.object(kernel, 'children_reaped', return_value={'childrenReaped': True}):
            return kernel.capture(state, CID, time.monotonic() + 1)

    def test_actual_numeric_reads_all_must_equal_fixed_original_controls(self):
        value = self.capture()
        self.assertEqual(value['cgroup']['pids.max'], '32\n')
        for field, replacement in [('memory.max', 'max\n'), ('memory.swap.max', '1\n'),
                                   ('pids.max', '256\n'), ('cpu.max', 'max 100000\n'),
                                   ('cpuset.cpus.effective', '0-2\n')]:
            with self.assertRaises(ValueError):
                self.capture({GROUP + '/' + field: replacement})

    def test_status_security_and_private_mount_root_are_independently_required(self):
        for before, after in [('NoNewPrivs:\t1', 'NoNewPrivs:\t0'), ('Seccomp:\t2', 'Seccomp:\t0'),
                              ('CapBnd:\t0000000000000000', 'CapBnd:\t0000000000000001'),
                              ('NSpid:\t321 1', 'NSpid:\t321'), ('Uid:\t65534 65534 65534 65534', 'Uid:\t0 0 0 0')]:
            with self.assertRaises(ValueError):
                self.capture({'/proc/321/status': STATUS.replace(before, after)})
        with self.assertRaises(ValueError):
            self.capture({'/proc/321/mountinfo': mountinfo().replace('/docker/' + CID, '/docker/' + 'c' * 64)})

    def test_writable_root_overmounted_installation_and_inflated_tmpfs_are_rejected(self):
        for text in [mountinfo().replace('/ / ro - overlay', '/ / rw - overlay'),
                     mountinfo().replace('size=65536k', 'size=131072k'),
                     mountinfo() + '99 1 0:99 / /opt/zryna-release rw - tmpfs tmpfs rw\n']:
            with self.assertRaises(ValueError):
                kernel.mounts(text, CID)

    def test_reap_pending_is_distinct_from_a_security_failure(self):
        with self.assertRaises(kernel.ReapPending):
            self.capture({GROUP + '/cgroup.procs': '321\n322\n', GROUP + '/pids.current': '2\n'})
        with self.assertRaises(ValueError):
            self.capture({GROUP + '/pids.current': '33\n'})


if __name__ == '__main__':
    unittest.main()
