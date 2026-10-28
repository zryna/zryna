"""Synthetic observations on either host; no kernel, Engine, namespace or approval execution."""
import errno
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import py_compile
from tempfile import TemporaryDirectory
import unittest

SOURCE = Path(__file__).resolve().parents[1] / 'scripts' / 'stability-gates'


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, SOURCE / file)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


CORE = load('test_native_host_core', 'native_host_facts.py')
MOUNT = b'20 1 0:28 / /sys/fs/cgroup rw,nosuid,nodev,noexec - cgroup2 cgroup rw\n'


def proc_stat(pid, ticks='99'):
    return f'{pid} (synthetic process) '.encode() + (' '.join(['S'] + ['0'] * 18 + [ticks]) + '\n').encode()


class FakeIO:
    def __init__(self):
        self.pid, self.clock, self.calls, self.counts = 1234, 0, [], {}
        self.overrides = {}
        self.env = {'GITHUB_RUN_ID': '123', 'GITHUB_RUN_ATTEMPT': '1',
                    'ZRYNA_NATIVE_RUNNER_ENVIRONMENT': 'github-hosted', 'GITHUB_TOKEN': 'do-not-export'}
        self.after_read = lambda: None

    def monotonic(self):
        return self.clock

    def environment(self, key):
        return self.env.get(key)

    def value(self, path, default):
        self.calls.append(path)
        index = self.counts.get(path, 0)
        self.counts[path] = index + 1
        value = self.overrides.get(path, default)
        if isinstance(value, list):
            value = value[min(index, len(value) - 1)]
        if isinstance(value, OSError):
            raise value
        self.after_read()
        return value

    def read(self, path, maximum):
        if path.startswith('/sys/fs/cgroup/'):
            leaf = path.rsplit('/', 1)[1]
            values = {'cpu.max': b'200000 100000\n', 'memory.max': b'536870912\n',
                      'memory.swap.max': b'0\n', 'pids.max': b'32\n', 'cpuset.cpus.effective': b'0-1\n',
                      'cpuset.mems.effective': b'0\n', 'cgroup.type': b'domain\n'}
            return self.value(path, values[leaf])
        parts = path.split('/')
        pid, leaf = int(parts[2]), parts[3]
        values = {'status': b'Name:\tsynthetic\nUid:\t1000 1000 1000 1000\n', 'stat': proc_stat(pid),
                  'cgroup': b'0::/runner/leaf\n' if pid == self.pid else b'0::/\n', 'mountinfo': MOUNT,
                  'uid_map': b'0 0 4294967295\n', 'gid_map': b'0 0 4294967295\n'}
        return self.value(path, values[leaf])

    def readlink(self, path):
        kind = path.rsplit('/', 1)[1]
        return self.value(path, kind + ':[100]')


class NativeHostFactsTests(unittest.TestCase):
    def test_closed_two_phase_raw_observations_never_approve_host_or_ancestry(self):
        io = FakeIO()
        result = CORE.collect_native_host(io)
        for name in ('sourceAuthenticated', 'hostAuthenticated', 'runtimeApproved', 'executionApproved',
                     'proofQualified', 'performanceQualified'):
            self.assertIs(result[name], False)
        self.assertIsNone(result['thresholds'])
        self.assertNotIn('ancestryComplete', result)
        self.assertNotIn('do-not-export', json.dumps(result))
        self.assertEqual(result['processEpochs']['self']['comparison'], 'same')
        for phase in ('before', 'after'):
            observed = result['phases'][phase]
            self.assertEqual([row['group'] for row in observed['ancestorControls']], ['/', '/runner', '/runner/leaf'])
            self.assertIn('not proof of global root', observed['mapping']['scope'])
            raw = observed['subjects']['self']['files']['status']
            self.assertEqual(bytes.fromhex(raw['rawHex']), b'Name:\tsynthetic\nUid:\t1000 1000 1000 1000\n')
        self.assertLessEqual(result['operations'], CORE.LIMITS['operations'])
        self.assertLessEqual(result['rawBytes'], CORE.LIMITS['rawBytes'])

    def test_namespace_errno_before_after_is_original_and_does_not_trigger_traversal(self):
        io = FakeIO()
        io.overrides['/proc/1/ns/mnt'] = [OSError(errno.EACCES, 'synthetic'), OSError(errno.EPERM, 'synthetic')]
        result = CORE.collect_native_host(io)
        self.assertEqual([row['errno'] for row in result['namespaceDenials']], [errno.EACCES, errno.EPERM])
        self.assertFalse(any(path.startswith('/sys/fs/cgroup/') for path in io.calls))
        for phase, code in [('before', errno.EACCES), ('after', errno.EPERM)]:
            row = result['phases'][phase]['subjects']['pid1']['namespaces']['mnt']
            self.assertEqual(row['errno'], code)

    def test_traversal_double_slash_escapes_and_legacy_memberships_are_retained_but_never_walked(self):
        for path in ('/../docker/test', '/runner/../leaf', '/runner//leaf', '/runner/.', '/runner/leaf/', '/bad\\040name'):
            io = FakeIO()
            original = ('0::' + path + '\n').encode()
            io.overrides['/proc/1234/cgroup'] = original
            result = CORE.collect_native_host(io)
            self.assertFalse(any(value.startswith('/sys/fs/cgroup/') for value in io.calls))
            self.assertEqual(bytes.fromhex(result['phases']['before']['subjects']['self']['files']['cgroup']['rawHex']), original)
        io = FakeIO()
        io.overrides['/proc/1234/cgroup'] = b'2:cpu:/legacy\n0::/runner\n'
        self.assertEqual(CORE.collect_native_host(io)['phases']['before']['mapping']['status'], 'unavailable')

    def test_rebased_ambiguous_or_different_mounts_refuse_all_control_walks(self):
        for raw in (MOUNT.replace(b' / /sys', b' /rebased /sys'), MOUNT.replace(b' / /sys', b' /.. /sys'),
                    MOUNT + MOUNT.replace(b'/sys/fs/cgroup', b'/another'), b'broken mount\n'):
            io = FakeIO()
            io.overrides['/proc/1234/mountinfo'] = raw
            result = CORE.collect_native_host(io)
            self.assertEqual(result['phases']['before']['mapping']['status'], 'unavailable')
            self.assertFalse(any(path.startswith('/sys/fs/cgroup/') for path in io.calls))
        io = FakeIO()
        io.overrides['/proc/1/ns/cgroup'] = 'cgroup:[200]'
        self.assertEqual(CORE.collect_native_host(io)['phases']['before']['mapping']['status'], 'unavailable')

    def test_visible_root_membership_is_only_a_single_mount_boundary_observation(self):
        io = FakeIO()
        io.overrides['/proc/1234/cgroup'] = b'0::/\n'
        result = CORE.collect_native_host(io)
        self.assertEqual(len(result['phases']['before']['ancestorControls']), 1)
        self.assertFalse(result['hostAuthenticated'])
        self.assertNotIn('ancestryComplete', result)

    def test_each_control_errno_survives_before_and_after_without_guessing_limit_values(self):
        io = FakeIO()
        io.overrides['/sys/fs/cgroup/runner/memory.max'] = [OSError(errno.ENOENT, 'synthetic'), OSError(errno.EPERM, 'synthetic')]
        result = CORE.collect_native_host(io)
        for phase, code in [('before', errno.ENOENT), ('after', errno.EPERM)]:
            group = next(row for row in result['phases'][phase]['ancestorControls'] if row['group'] == '/runner')
            self.assertEqual(group['files']['memory.max']['errno'], code)
            self.assertNotIn('rawHex', group['files']['memory.max'])
            self.assertEqual(group['files']['cpu.max']['status'], 'observed')

    def test_first_extra_byte_is_retained_as_oversize_not_a_complete_original(self):
        io = FakeIO()
        original = b'x' * (CORE.FILES['status'] + 1)
        io.overrides['/proc/1234/status'] = original
        row = CORE.collect_native_host(io)['phases']['before']['subjects']['self']['files']['status']
        self.assertEqual(row['status'], 'oversize')
        self.assertEqual(row['sha256'], hashlib.sha256(original).hexdigest())
        self.assertEqual(bytes.fromhex(row['rawHex']), original)

    def test_deadline_and_operation_limits_stop_real_adapter_calls(self):
        io = FakeIO()
        io.after_read = lambda: setattr(io, 'clock', 9)
        result = CORE.collect_native_host(io)
        self.assertEqual(len(io.calls), 1)
        self.assertEqual(result['phases']['after']['subjects']['self']['files']['stat']['reason'], 'deadline')
        io = FakeIO()
        budget = CORE._Budget(io)
        for _ in range(CORE.LIMITS['operations'] + 1):
            row = budget.sample('/proc/1234/stat', 8192)
        self.assertEqual(row['reason'], 'operation-budget')
        self.assertEqual(len(io.calls), CORE.LIMITS['operations'])

    def test_total_bytes_and_depth_are_bounded_without_traversal_normalization(self):
        io = FakeIO()
        budget = CORE._Budget(io)
        io.overrides['/proc/1234/mountinfo'] = b'x' * 262145
        for _ in range(10):
            row = budget.sample('/proc/1234/mountinfo', 262144)
        self.assertEqual(row['reason'], 'raw-byte-budget')
        self.assertLessEqual(budget.raw_bytes, CORE.LIMITS['rawBytes'])
        io = FakeIO()
        io.overrides['/proc/1234/cgroup'] = ('0::/' + '/'.join(['deep'] * 17) + '\n').encode()
        self.assertEqual(CORE.collect_native_host(io)['phases']['before']['mapping']['status'], 'unavailable')

    def test_invalid_utf8_and_namespace_text_stay_raw_unknown(self):
        io = FakeIO()
        io.overrides['/proc/1234/cgroup'] = b'0::/\xff\n'
        result = CORE.collect_native_host(io)
        self.assertEqual(result['phases']['before']['mapping']['status'], 'unavailable')
        self.assertEqual(bytes.fromhex(result['phases']['before']['subjects']['self']['files']['cgroup']['rawHex']), b'0::/\xff\n')
        io = FakeIO()
        io.overrides['/proc/1/ns/mnt'] = 'invented namespace'
        self.assertEqual(CORE.collect_native_host(io)['phases']['before']['mapping']['status'], 'unavailable')

    def test_process_birth_change_and_projection_unavailability_are_not_approvals(self):
        io = FakeIO()
        io.overrides['/proc/1234/stat'] = [proc_stat(1234, '1'), proc_stat(1234, '2')]
        io.env['ImageVersion'] = 'x' * 2049
        result = CORE.collect_native_host(io)
        self.assertEqual(result['processEpochs']['self']['comparison'], 'changed')
        self.assertEqual(result['phases']['after']['projections']['ImageVersion']['status'], 'unavailable')
        with self.assertRaises(TypeError):
            CORE.collect_native_host(io, approved=True)

    def test_default_driver_refuses_arbitrary_selectors_before_any_kernel_read(self):
        driver = load('test_native_host_driver', 'collect-native-host-facts.py')
        io = driver._KernelIO()
        for path in ('/etc/passwd', '/proc/0/stat', '/sys/fs/cgroup/../memory.max'):
            with self.assertRaises(ValueError):
                io.read(path, 4096)
        with self.assertRaises(ValueError):
            io.readlink('/proc/1/exe')

    def test_raw_membership_mount_and_namespace_comparisons_never_infer_authority(self):
        io = FakeIO()
        io.overrides['/proc/1234/cgroup'] = [b'0::/runner/leaf\n', b'0::/runner/other\n']
        io.overrides['/proc/1234/mountinfo'] = [MOUNT, MOUNT.replace(b'20 1 ', b'21 1 ')]
        io.overrides['/proc/1234/ns/net'] = ['net:[100]', 'net:[200]']
        io.overrides['/proc/1/ns/pid'] = OSError(errno.EACCES, 'synthetic')
        result = CORE.collect_native_host(io)
        self.assertEqual(result['rawComparisons']['self']['cgroup'], 'changed')
        self.assertEqual(result['rawComparisons']['self']['mountinfo'], 'changed')
        self.assertEqual(result['rawComparisons']['self']['namespaces']['net'], 'changed')
        self.assertEqual(result['rawComparisons']['pid1']['namespaces']['pid'], 'unavailable')
        self.assertFalse(result['hostAuthenticated'])

    def test_driver_compiles_fixed_core_source_even_with_valid_conflicting_timestamp_pyc(self):
        with TemporaryDirectory(prefix='native-host-source-custody-') as temporary:
            root = Path(temporary)
            core = root / 'native_host_facts.py'
            cached, source = b"MARKER = 'cached'\n", b"MARKER = 'source'\n"
            self.assertEqual(len(cached), len(source))
            core.write_bytes(cached)
            original = core.stat()
            py_compile.compile(str(core), doraise=True, invalidation_mode=py_compile.PycInvalidationMode.TIMESTAMP)
            core.write_bytes(source)
            os.utime(core, ns=(original.st_atime_ns, original.st_mtime_ns))
            # Independent positive control: ordinary loading really selects the valid stale pyc.
            ordinary_spec = importlib.util.spec_from_file_location('cached_marker_control', core)
            ordinary = importlib.util.module_from_spec(ordinary_spec)
            ordinary_spec.loader.exec_module(ordinary)
            self.assertEqual(ordinary.MARKER, 'cached')
            driver = root / 'collect-native-host-facts.py'
            driver.write_bytes((SOURCE / driver.name).read_bytes())
            spec = importlib.util.spec_from_file_location('driver_source_custody_test', driver)
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)  # Not __main__: no collector/kernel observation.
            self.assertEqual(module._CORE.MARKER, 'source')


if __name__ == '__main__':
    unittest.main()
