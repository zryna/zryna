"""Synthetic kernel readback fixtures only; no container, browser or acquired code execution."""
import errno
import stat
import types
import re
import unittest
from unittest import mock
import owner_kernel as kernel

CID = 'a' * 64
PID = 321
MEMBERSHIP = '0::/owner/docker/' + CID + '\n'
GROUP = '/sys/fs/cgroup/owner/docker/' + CID


def status(pid, parent, initial=False):
    value = {'Pid': str(pid), 'Tgid': str(pid), 'PPid': str(parent), 'Threads': '1',
             'Uid': '65534\t65534\t65534\t65534', 'Gid': '65534\t65534\t65534\t65534',
             'CapInh': '0000000000000000', 'CapPrm': '0000000000000000',
             'CapEff': '0000000000000000', 'CapBnd': '0000000000000000',
             'CapAmb': '0000000000000000', 'NoNewPrivs': '1', 'Seccomp': '2',
             'Cpus_allowed_list': '0-1', 'NSpid': f'{pid}\t{1 if initial else 2}'}
    return ''.join(key + ':\t' + item + '\n' for key, item in value.items())


def proc_stat(pid, tick='100', parent=None, state='S'):
    parent = (10 if pid == 321 else 321) if parent is None else parent
    return f'{pid} (name with ) characters) {state} ' + ' '.join([str(parent)] + ['0'] * 17 + [tick]) + '\n'


def fixture():
    rows = {'/proc/321/stat': proc_stat(321), '/proc/322/stat': proc_stat(322),
            '/proc/321/status': status(321, 10, True), '/proc/322/status': status(322, 321),
            GROUP + '/memory.max': '536870912\n', GROUP + '/memory.swap.max': '0\n',
            GROUP + '/pids.max': '256\n', GROUP + '/cpu.max': '200000 100000\n',
            GROUP + '/cpuset.cpus.effective': '0-1\n', GROUP + '/pids.current': '3\n',
            GROUP + '/cgroup.procs': '321\n322\n', GROUP + '/cgroup.threads': '321\n322\n323\n',
            '/proc/321/net/dev':
            'Inter-| Receive | Transmit\n face |bytes packets|bytes packets\n lo: ' + '0 ' * 16 + '\n',
            '/proc/321/net/route': 'Iface\tDestination\tGateway\n', '/proc/321/net/ipv6_route':
            ' '.join(['0' * 32, '00', '0' * 32, '00', '0' * 32, 'ffffffff',
                      '00000001', '00000000', '00200200', 'lo']) + '\n'}
    for pid in [321, 322, 323]:
        rows[f'/proc/{pid}/cgroup'] = MEMBERSHIP
    rows['/proc/321/mountinfo'] = '\n'.join([
        '1 0 0:1 / / ro - overlay overlay ro',
        '2 1 0:2 /volume /materials ro - ext4 /dev/disk rw',
        '3 1 0:3 / /sys/fs/cgroup ro - cgroup2 cgroup rw',
        '4 1 0:4 / /tmp rw,nosuid,nodev,noexec - tmpfs tmpfs rw,size=131072k',
        '5 1 0:5 / /dev/shm rw,nosuid,nodev,noexec - tmpfs tmpfs rw,size=65536k',
        '6 1 0:6 / /proc rw,nosuid,nodev,noexec - proc proc rw',
        *[f'{index} 6 0:6 {path[5:]} {path} ro,nosuid,nodev,noexec - proc proc rw'
          for index, path in enumerate(kernel.owner_mounts.PROC_READONLY, 7)]]) + '\n'
    state = {'Id': CID, 'RestartCount': 0, 'State': {'Pid': 321, 'Running': True, 'Paused': False,
             'Restarting': False, 'OOMKilled': False, 'StartedAt': '2026-10-15T00:00:00Z'}}
    return rows, state


class KernelTests(unittest.TestCase):
    def capture(self, rows, state, link=None):
        def observe(path, deadline, maximum=16384):
            value = rows[path]
            if isinstance(value, Exception):
                raise value
            return value
        def namespace(path):
            if link:
                return link(path)
            subject, kind = path.split('/')[2], path.split('/')[-1]
            return kind + (':[101]' if subject == 'self' else ':[202]')
        with mock.patch.object(kernel, 'read', side_effect=observe), \
                mock.patch.object(kernel.owner_cgroup, 'mapping', return_value={'path': GROUP,
                                  'profile': 'full-root-canonical-docker-v1'}), \
                mock.patch.object(kernel.os, 'readlink', side_effect=namespace), \
                mock.patch.object(kernel.owner_proc, 'inventory', side_effect=lambda reader, deadline: {
                    int(re.fullmatch(r'/proc/([0-9]+)/stat', path)[1]): kernel.owner_proc.process(value,
                        int(re.fullmatch(r'/proc/([0-9]+)/stat', path)[1]))
                    for path, value in rows.items() if re.fullmatch(r'/proc/([0-9]+)/stat', path)}), \
                mock.patch.object(kernel.time, 'monotonic', return_value=100):
            return kernel.capture(state, CID, 399)

    def test_full_observed_profile_and_no_claimed_descendant_caps(self):
        rows, state = fixture()
        rows['/proc/322/status'] += 'NestedUserNamespaceFixture:\tcapabilities are not an init claim\n'
        value = self.capture(rows, state)
        self.assertEqual(value['pid'], 321)
        self.assertEqual(value['ticks'], '100')
        self.assertEqual(value['processes']['pidsCurrent'], 3)
        self.assertTrue(value['network']['privateNamespaceLinkVerified'])
        self.assertEqual(value['mounts']['/materials']['options'], ['ro'])

    def test_namespace_access_denial_is_retained_without_guessed_identity(self):
        rows, state = fixture()
        def denied(path):
            if '/321/' in path:
                raise PermissionError(errno.EACCES, 'fixture inaccessible namespace')
            return path.split('/')[-1] + ':[101]'
        value = self.capture(rows, state, denied)
        self.assertEqual(value['namespaceLinks']['321/net'], {'actualLink': None, 'unavailableErrno': errno.EACCES})
        self.assertFalse(value['network']['privateNamespaceLinkVerified'])
        self.assertTrue(value['network']['namespaceIdentityUnobserved'])

    def test_initial_security_negatives(self):
        changes = [('Uid:\t65534', 'Uid:\t0'), ('Gid:\t65534', 'Gid:\t0'),
                   ('NoNewPrivs:\t1', 'NoNewPrivs:\t0'), ('Seccomp:\t2', 'Seccomp:\t0'),
                   ('Cpus_allowed_list:\t0-1', 'Cpus_allowed_list:\t0-3'), ('NSpid:\t321\t1', 'NSpid:\t321\t2')]
        changes += [(key + ':\t0000000000000000', key + ':\t0000000000000001')
                    for key in ['CapInh', 'CapPrm', 'CapEff', 'CapBnd', 'CapAmb']]
        for before, after in changes:
            with self.subTest(field=before):
                rows, state = fixture()
                rows['/proc/321/status'] = rows['/proc/321/status'].replace(before, after)
                with self.assertRaises(ValueError):
                    self.capture(rows, state)

    def test_group_membership_budget_and_descendant_negatives(self):
        changes = [(GROUP + '/memory.max', 'max\n'), (GROUP + '/memory.swap.max', '1\n'),
                   (GROUP + '/pids.max', '257\n'), (GROUP + '/cpu.max', 'max 100000\n'),
                   (GROUP + '/cpuset.cpus.effective', '0-3\n'), (GROUP + '/pids.current', '257\n'),
                   (GROUP + '/cgroup.procs', '322\n'), (GROUP + '/cgroup.threads', '321\n321\n'),
                   ('/proc/322/cgroup', '0::/outside\n'), ('/proc/323/cgroup', '0::/outside\n'),
                   ('/proc/999/stat', proc_stat(999, parent=321)),
                   ('/proc/322/status', status(322, 999)),
                   ('/proc/321/cgroup', '0::/system.slice/docker-' + CID + '.scope\n')]
        for path, value in changes:
            with self.subTest(path=path):
                rows, state = fixture()
                rows[path] = value
                with self.assertRaises(ValueError):
                    self.capture(rows, state)

    def test_unrelated_zombies_are_counted_but_owned_unreaped_or_escaped_descendants_fail(self):
        rows, state = fixture()
        rows['/proc/999/stat'] = proc_stat(999, parent=1, state='Z')
        captured = self.capture(rows, state)
        self.assertEqual(captured['processes']['childObservation']['unrelatedZombiesObserved'], 1)
        for parent, process_state in [(321, 'Z'), (322, 'S')]:
            rows['/proc/999/stat'] = proc_stat(999, parent=parent, state=process_state)
            with self.assertRaises(ValueError):
                self.capture(rows, state)

    def test_mount_and_network_negatives(self):
        for before, after in [(' / ro -', ' / rw -'), (' /materials ro -', ' /materials rw -'),
                              ('size=131072k', 'size=262144k'), ('size=65536k', 'size=131072k'),
                              ('rw,nosuid,nodev,noexec', 'rw,nosuid,nodev'), ('cgroup2 cgroup', 'tmpfs cgroup')]:
            with self.subTest(mount=before):
                rows, state = fixture()
                rows['/proc/321/mountinfo'] = rows['/proc/321/mountinfo'].replace(before, after)
                with self.assertRaises(ValueError):
                    self.capture(rows, state)
        for path, extra in [('/proc/321/net/dev', 'eth0: ' + '0 ' * 16 + '\n'),
                            ('/proc/321/net/route', 'eth0 00000000 01000000\n'),
                            ('/proc/321/net/ipv6_route', 'external malformed route\n')]:
            rows, state = fixture()
            rows[path] += extra
            with self.assertRaises(ValueError):
                self.capture(rows, state)
        rows, state = fixture()
        with self.assertRaises(ValueError):
            self.capture(rows, state, lambda path: path.split('/')[-1] + ':[101]')
        rows, state = fixture()
        rows['/proc/321/mountinfo'] += '6 2 0:6 / /materials/subtree ro - tmpfs tmpfs ro\n'
        with self.assertRaises(ValueError):
            self.capture(rows, state)

    def test_original_init_change_and_finite_deadline(self):
        rows, state = fixture()
        calls = 0
        def changing(pid, deadline):
            nonlocal calls
            calls += 1
            return str(calls)
        with mock.patch.object(kernel, 'ticks', side_effect=changing), self.assertRaises(ValueError):
            self.capture(rows, state)
        with mock.patch.object(kernel.time, 'monotonic', return_value=100):
            for deadline in [99, 401, float('inf'), float('nan')]:
                with self.assertRaises(ValueError):
                    kernel.capture(state, CID, deadline)

    def test_old_init_absence_pid_reuse_empty_group_and_removal_race(self):
        init = {'pid': PID, 'ticks': '100', 'cid': CID, 'membership': MEMBERSHIP}
        empty = {GROUP + '/cgroup.events': 'populated 0\nfrozen 0\n', GROUP + '/cgroup.procs': '',
                 GROUP + '/cgroup.threads': '', GROUP + '/pids.current': '0\n'}
        with mock.patch.object(kernel.time, 'monotonic', return_value=100), \
                mock.patch.object(kernel, 'group_path', return_value=GROUP), \
                mock.patch.object(kernel, 'ticks', return_value='200'), \
                mock.patch.object(kernel, 'read', side_effect=lambda path, deadline: empty[path]):
            self.assertTrue(kernel.gone(init, 101.9)['pidReused'])
        with mock.patch.object(kernel.time, 'monotonic', return_value=100), \
                mock.patch.object(kernel, 'group_path', return_value=GROUP), \
                mock.patch.object(kernel, 'ticks', side_effect=FileNotFoundError), \
                mock.patch.object(kernel, 'read', side_effect=FileNotFoundError), \
                mock.patch.object(kernel.os.path, 'lexists', return_value=False):
            self.assertEqual(kernel.gone(init, 101.9)['cgroup'], 'removed')
        def removal_after_events(path, deadline):
            if path.endswith('/cgroup.events'):
                return empty[path]
            raise FileNotFoundError()
        for exists in [False, True]:
            with mock.patch.object(kernel.time, 'monotonic', return_value=100), \
                    mock.patch.object(kernel, 'group_path', return_value=GROUP), \
                    mock.patch.object(kernel, 'ticks', side_effect=FileNotFoundError), \
                    mock.patch.object(kernel, 'read', side_effect=removal_after_events), \
                    mock.patch.object(kernel.os.path, 'lexists', return_value=exists):
                if exists:
                    with self.assertRaises(ValueError):
                        kernel.gone(init, 101.9)
                else:
                    self.assertEqual(kernel.gone(init, 101.9)['cgroup'], 'removed')
        for key, value in [(GROUP + '/cgroup.events', 'populated 1\n'), (GROUP + '/pids.current', '1\n'),
                           (GROUP + '/cgroup.threads', '555\n')]:
            altered = {**empty, key: value}
            with mock.patch.object(kernel.time, 'monotonic', return_value=100), \
                    mock.patch.object(kernel, 'group_path', return_value=GROUP), \
                    mock.patch.object(kernel, 'ticks', side_effect=FileNotFoundError), \
                    mock.patch.object(kernel, 'read', side_effect=lambda path, deadline: altered[path]), \
                    self.assertRaises(ValueError):
                kernel.gone(init, 101.9)

    def test_original_init_alive_and_unknown_cleanup_are_not_success(self):
        init = {'pid': PID, 'ticks': '100', 'cid': CID, 'membership': MEMBERSHIP}
        with mock.patch.object(kernel.time, 'monotonic', return_value=100):
            with mock.patch.object(kernel, 'ticks', return_value='100'), self.assertRaises(ValueError):
                kernel.gone(init, 101.9)
            with mock.patch.object(kernel, 'ticks', side_effect=PermissionError), self.assertRaises(PermissionError):
                kernel.gone(init, 101.9)

    def test_bounded_real_reader_closes_descriptor_on_first_extra_byte(self):
        with mock.patch.object(kernel.time, 'monotonic', return_value=100), \
                mock.patch.object(kernel.os, 'open', return_value=7), \
                mock.patch.object(kernel.os, 'fstat', return_value=types.SimpleNamespace(st_mode=stat.S_IFREG)), \
                mock.patch.object(kernel.os, 'read', side_effect=[b'12345']), \
                mock.patch.object(kernel.os, 'close') as close, self.assertRaises(ValueError):
            kernel.read('/proc/321/status', 101, 4)
        close.assert_called_once_with(7)


if __name__ == '__main__':
    unittest.main()
