"""Pure state/record controls. These never execute a guest or authenticate observations."""
import copy
import struct
import unittest
from types import SimpleNamespace
from unittest.mock import patch
from owner_test_fixture import OWNER, INPUT, OPERATION, EXECUTE, FakeBackend, selected

CGROUP = OWNER._load('lifecycle_cgroup')


def repin(value):
    value['planSha256'] = INPUT.sha256(INPUT.canonical(value['plan']))
    return value


def transcript(mode):
    if mode['id'] == 'default':
        names = [(name, 'ok') for name in INPUT.ORDINARY] + [(INPUT.NAME, 'ignored, requires independently authenticated installed materials and owned native parent envelope')]
        header, footer = 'running 3 tests', '2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out'
    else:
        names = [(INPUT.NAME, 'ok')]
        header, footer = 'running 1 test', '1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out'
    return ('\n' + header + '\n' + ''.join('test ' + name + ' ... ' + status + '\n' for name, status in names)
            + '\ntest result: ok. ' + footer + '; finished in 0.01s\n\n').encode()


class OwnerTests(unittest.TestCase):
    def test_pair_is_two_fresh_exact_modes_and_no_admission(self):
        value = selected(); backend = FakeBackend(value)
        self.assertEqual(INPUT.sha256(INPUT.canonical(value['plan']['parent'])), 'c41024eede48bb3a5f411770517b0c64a44219adeaec21cae264046932e2f5d9')
        result = OPERATION.run_pair(value, backend)
        self.assertTrue(result['completed'])
        self.assertEqual(result['observationMode'], 'synthetic-adapter')
        for key, field in INPUT.FLAGS.items():
            self.assertIs(result[key], field)
        self.assertEqual([row['mode'] for row in result['phases']], ['default', 'positive'])
        self.assertEqual(len({row['cid'] for row in result['phases']}), 2)
        self.assertEqual([(mode, step) for mode, step, argv in backend.apps],
            [(mode, step) for mode in ['default', 'positive'] for step in ['audit-before', 'python-bootstrap', 'test', 'audit-after']])
        self.assertEqual(backend.apps[2][2], ['/lifecycle/test', '--test-threads=1', '--color', 'never'])
        self.assertEqual(backend.apps[6][2], ['/lifecycle/test', '--ignored', '--exact', INPUT.NAME, '--test-threads=1', '--color', 'never'])
        self.assertEqual(backend.objects, {})
        self.assertTrue(all(row['cleanup']['confirmed'] for row in result['phases']))
        deletes = [path for method, path, end in backend.events if method == 'DELETE']
        self.assertTrue(all(path.endswith('?force=0&v=0') for path in deletes))
        self.assertEqual(len(deletes), 2)

    def test_repin_does_not_authorize_changed_profile_command_or_material(self):
        mutations = [lambda row: row['plan']['parent'].update(tasks=33),
                     lambda row: row['plan']['parent'].update(cleanupMs=3000),
                     lambda row: row['plan']['parent']['inheritedLimits'].update(nofile=1024),
                     lambda row: row['plan']['parent']['materialMount'].update(noCopy=False),
                     lambda row: row['plan']['modes'][0]['args'].append('--ignored'),
                     lambda row: row['plan']['modes'][1]['counts'].update(filtered=0),
                     lambda row: row['plan']['materials']['files'][0].update(sha256='f' * 64),
                     lambda row: row['plan'].update(sourceAuthenticated=True),
                     lambda row: row.update(approved=True)]
        for change in mutations:
            with self.subTest(change=change):
                value = selected(); change(value); repin(value)
                backend = FakeBackend(value)
                with self.assertRaises((ValueError, KeyError)):
                    OPERATION.run_pair(value, backend)
                self.assertEqual(backend.events, [])

    def test_binding_mismatches_refuse_before_engine(self):
        for change in [lambda row: row['capture']['source'].update(tree='c' * 40),
                       lambda row: row['capture']['compiler'].update(path='/tmp/release/zryna-playground-compiler'),
                       lambda row: row['runtime']['files'][0].update(sha256='f' * 64),
                       lambda row: row['materials'].update(manifestSha256='f' * 64),
                       lambda row: row['nativeVm'].update(raw=b'changed')]:
            value = selected(); change(value); backend = FakeBackend(value)
            with self.assertRaises(ValueError):
                OPERATION.run_pair(value, backend)
            self.assertEqual(backend.events, [])

    def test_pre_application_failures_never_run_apps(self):
        for fault in ['peer', 'image', 'writer', 'namespace', 'ancestor', 'zombie']:
            with self.subTest(fault=fault):
                value = selected(); backend = FakeBackend(value, fault)
                result = OPERATION.run_pair(value, backend)
                self.assertFalse(result['completed']); self.assertEqual(len(result['phases']), 1)
                self.assertEqual(backend.apps, [])
                if fault in ['namespace', 'ancestor', 'zombie']:
                    self.assertFalse(result['phases'][0]['cleanup']['confirmed'])
                    self.assertEqual(backend.objects, {})  # Safe removal is attempted; it cannot manufacture proof.

    def test_failed_test_stops_positive_even_after_confirmed_cleanup(self):
        value = selected(); backend = FakeBackend(value, 'app')
        result = OPERATION.run_pair(value, backend)
        self.assertEqual(len(result['phases']), 1)
        self.assertTrue(result['phases'][0]['cleanup']['confirmed'])
        self.assertFalse(result['completed'])
        self.assertEqual([step for mode, step, argv in backend.apps], ['audit-before', 'python-bootstrap', 'test'])

    def test_unknown_creation_name404_is_not_cleanup_authority(self):
        value = selected(); backend = FakeBackend(value, 'create-unknown')
        result = OPERATION.run_pair(value, backend)
        self.assertEqual(len(result['phases']), 1)
        self.assertFalse(result['phases'][0]['cleanup']['confirmed'])
        self.assertTrue(result['phases'][0]['createAttempted'])
        self.assertEqual(backend.apps, [])

    def test_cleanup_failure_or_original_kernel_survivor_stops_pair(self):
        for fault in ['remove', 'gone']:
            value = selected(); backend = FakeBackend(value, fault)
            result = OPERATION.run_pair(value, backend)
            self.assertEqual(len(result['phases']), 1)
            self.assertTrue(result['phases'][0]['completed'])
            self.assertFalse(result['phases'][0]['cleanup']['confirmed'])
            self.assertFalse(result['completed'])

    def test_active_deadline_is_not_renewed_by_observation_or_cleanup(self):
        value = selected(); backend = FakeBackend(value, 'deadline')
        result = OPERATION.run_pair(value, backend)
        phase = result['phases'][0]
        self.assertEqual(phase['deadline'], 110)
        self.assertEqual(phase['cleanup']['deadline'], 112.001)
        self.assertFalse(result['completed']); self.assertEqual(backend.apps, [])
        self.assertTrue(all(end in [110, 112.001] for method, path, end in backend.events))

    def test_original_transcripts_and_mutants(self):
        for mode in INPUT.modes():
            raw = transcript(mode); EXECUTE.libtest(raw, mode)
            mutants = [raw[:-1], raw.replace(b' passed;', b' failed;', 1), raw.replace(b'test result: ok.', b'test result: FAILED.'),
                       raw.replace(b' ... ok', b' ... ignored', 1), raw + b'extra\n', raw.replace(b'finished in 0.01s', b'finished in NaNs')]
            first_row = next(line for line in raw.splitlines(keepends=True) if line.startswith(b'test ') and b' ... ' in line)
            mutants.append(raw.replace(first_row, first_row + first_row, 1))
            for mutant in mutants:
                with self.assertRaises(ValueError):
                    EXECUTE.libtest(mutant, mode)

    def test_attached_wire_complete_streams_and_exact_byte_ceiling(self):
        frame = lambda channel, body: struct.pack('>BBBBI', channel, 0, 0, 0, len(body)) + body
        self.assertEqual(EXECUTE.streams(frame(1, b'a') + frame(2, b'b')), (b'a', b'b'))
        self.assertEqual(len(EXECUTE.streams(frame(1, b'x' * 65536))[0]), 65536)
        for raw in [frame(1, b'x' * 65536) + frame(1, b'x'), frame(1, b'x')[:-1], b'\x01', frame(3, b'x'), frame(1, b'')]:
            with self.assertRaises(ValueError):
                EXECUTE.streams(raw)

    def test_actual_config_readonly_nocopy_and_namespace_mutants(self):
        value = selected(); backend = FakeBackend(value)
        expected = INPUT.configuration(value, 'a' * 32, backend.seccomp)
        engine = backend.engine(110, [])
        cid = engine.json('POST', '/containers/create?name=owned', expected)['Id']
        original = backend.objects[cid]
        INPUT.container_identity(original, cid, 'owned', expected, value)
        for change in [lambda row: row['Mounts'][0].update(RW=True),
                       lambda row: row['HostConfig'].update(PidMode='host'),
                       lambda row: row['HostConfig']['Mounts'][0]['VolumeOptions'].update(NoCopy=False),
                       lambda row: row['Config']['Env'].append('RUSTFLAGS=-Cpanic=abort')]:
            mutant = copy.deepcopy(original); change(mutant)
            with self.assertRaises(ValueError):
                INPUT.container_identity(mutant, cid, 'owned', expected, value)

    def test_listed_writers_reject_name_ancestor_resolved_and_inode_aliases(self):
        module = OWNER._load('lifecycle_backend')
        value = selected(); backend = module.Backend.__new__(module.Backend); backend.selected = value
        root = '/var/lib/docker/volumes/' + value['materials']['name'] + '/_data'
        nodes = {root: 1, '/': 2}
        def metadata(name):
            if name not in nodes: nodes[name] = len(nodes) + 10
            return SimpleNamespace(st_dev=7, st_ino=nodes[name], st_mode=0o40755)
        row = {'Id': 'a' * 64, 'Mounts': [{'RW': False, 'Type': 'volume', 'Name': value['materials']['name'], 'Source': root}]}
        engine = SimpleNamespace(request=lambda method, path, **options: (b'[{"Id":"' + b'a' * 64 + b'"}]', None),
                                 json=lambda method, path: row)
        aliases = ['/elsewhere', root.rsplit('/', 1)[0], '/resolved', '/inode']
        def realpath(name, strict=False): return root if name == '/resolved' else name
        def stated(name): return metadata(root) if name == '/inode' else metadata(name)
        with patch.object(module.os.path, 'realpath', side_effect=realpath), patch.object(module.os, 'lstat', side_effect=metadata), patch.object(module.os, 'stat', side_effect=stated):
            backend.writers(engine, {})
            for name in aliases:
                row['Mounts'][0].update(RW=True, Source=name, Name=value['materials']['name'] if name == '/elsewhere' else 'different-volume')
                with self.assertRaises(ValueError): backend.writers(engine, {})

    def test_complete_ancestor_controls_and_root_only_enoent(self):
        value = selected(); cid = 'c' * 64; leaf = '/system.slice/docker-' + cid + '.scope'
        controls = {'memory.max': '536870912\n', 'memory.swap.max': '0\n', 'pids.max': '32\n', 'cpu.max': '200000 100000\n',
                    'cpuset.cpus.effective': '0-1\n', 'cpuset.mems.effective': '0\n', 'cgroup.type': 'domain\n'}
        def observed(text):
            raw = text.encode()
            return {'status': 'observed', 'rawHex': raw.hex(), 'bytes': len(raw), 'sha256': INPUT.sha256(raw)}
        namespaces = {kind: observed(kind + ':[7]') for kind in ['pid', 'mnt', 'cgroup', 'user']}
        root_files = {key: {'status': 'unavailable', 'errno': 2} if key in CGROUP.ROOT_ONLY else observed(raw) for key, raw in controls.items()}
        phase = {'mapping': {'status': 'visible-canonical-coordinates', 'mount': {'root': '/', 'mountpoint': '/sys/fs/cgroup', 'mountId': '1'}},
                 'projections': {'ZRYNA_NATIVE_RUNNER_ENVIRONMENT': {'status': 'observed-environment', 'value': 'github-hosted'}},
                 'ancestorControls': [{'group': '/', 'files': root_files}],
                 'subjects': {'pid1': {'files': {key: observed('0 0 4294967295\n') for key in ['uid_map', 'gid_map']}, 'namespaces': namespaces}}}
        document = {'format': 'zryna.linux-native-host-facts.v1', 'producer': {'effectiveUid': 0},
                    'phases': {key: copy.deepcopy(phase) for key in ['before', 'after']},
                    'processEpochs': {'pid1': {'comparison': 'same', 'beforeStartTicks': '33', 'afterStartTicks': '33'}}}
        def select_context(record):
            raw = INPUT.canonical(record); value['nativeVm'] = {'raw': raw, 'sha256': INPUT.sha256(raw)}
        select_context(document)
        seen = []
        def read(name, end, maximum=16384):
            seen.append(name)
            if name in ['/proc/1/uid_map', '/proc/1/gid_map']: return '0 0 4294967295\n'
            if name == '/proc/self/mountinfo': return '1 2 0:1 / /sys/fs/cgroup rw - cgroup2 cgroup rw\n'
            key = name.rsplit('/', 1)[1]
            if name == '/sys/fs/cgroup/' + key and key in CGROUP.ROOT_ONLY:
                raise FileNotFoundError(2, 'synthetic documented root absence')
            return controls[key]
        ticks = lambda pid, end: '33'
        with patch.object(CGROUP.os, 'readlink', side_effect=lambda name: name.rsplit('/', 1)[1] + ':[7]'):
            context = CGROUP.native_context(value, read, ticks, 1)
            self.assertEqual(set(context['rootOnlyAbsent']), CGROUP.ROOT_ONLY)
            result = CGROUP.ancestry(value, cid, '0::' + leaf + '\n', read, ticks, 1)
            self.assertEqual([row['group'] for row in result['ancestors']], [leaf, '/system.slice', '/'])
            empty = copy.deepcopy(document)
            for item in empty['phases'].values():
                item['ancestorControls'][0]['files'] = {key: observed(raw) for key, raw in controls.items()}
            select_context(empty)
            with self.assertRaisesRegex(ValueError, 'genuine hierarchy root'):
                CGROUP.native_context(value, read, ticks, 1)
            for readable in CGROUP.ROOT_RESOURCES:
                partial = copy.deepcopy(document)
                for item in partial['phases'].values():
                    item['ancestorControls'][0]['files'][readable] = observed(controls[readable])
                select_context(partial)
                with self.assertRaisesRegex(ValueError, 'genuine hierarchy root'):
                    CGROUP.native_context(value, read, ticks, 1)
            select_context(document)
            for readable in CGROUP.ROOT_ONLY:
                def appeared(name, end, maximum=16384):
                    return controls[readable] if name == '/sys/fs/cgroup/' + readable else read(name, end, maximum)
                with self.assertRaisesRegex(ValueError, 'became readable'):
                    CGROUP.native_context(value, appeared, ticks, 1)
            for fault in ['missing-non-root', 'lower-ancestor', 'wrong-leaf', 'rebased', 'appeared-during-walk']:
                hits = []
                def mutant(name, end, maximum=16384):
                    if fault == 'missing-non-root' and name == '/sys/fs/cgroup/system.slice/memory.max':
                        raise FileNotFoundError(2, 'synthetic forbidden non-root absence')
                    if fault == 'lower-ancestor' and name == '/sys/fs/cgroup/system.slice/cpu.max': return '199999 100000\n'
                    if fault == 'wrong-leaf' and name == '/sys/fs/cgroup' + leaf + '/pids.max': return '33\n'
                    if fault == 'appeared-during-walk' and name == '/sys/fs/cgroup/cpu.max':
                        hits.append(name)
                        if len(hits) > 1: return controls['cpu.max']
                    return read(name, end, maximum)
                with self.assertRaises(ValueError):
                    CGROUP.ancestry(value, cid, '0::/../hidden\n' if fault == 'rebased' else '0::' + leaf + '\n', mutant, ticks, 1)


if __name__ == '__main__':
    unittest.main()
