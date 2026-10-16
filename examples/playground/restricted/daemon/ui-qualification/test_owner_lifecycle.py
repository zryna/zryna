"""Inert Engine fixtures only: no socket, image import, process or browser launch."""
import copy
import hashlib
import json
import time
import unittest
from unittest import mock
import owner_lifecycle as lifecycle
import owner_identity as identity
import owner_config

CID = 'b' * 64


def approval():
    seccomp = '{"defaultAction":"SCMP_ACT_ERRNO","syscalls":[]}'
    return {'format': 'zryna.playground-ui-owner-approval.v1', 'version': 1,
            'image': 'sha256:' + 'a' * 64, 'layer_sha256': 'c' * 64,
            'seccomp': seccomp, 'seccomp_sha256': hashlib.sha256(seccomp.encode()).hexdigest(),
            'ui_plan_sha256': 'd' * 64, 'material': {'name': 'zryna410-materials-evaluator-' + 'e' * 32,
            'nonce': 'e' * 32, 'created_at': '2026-10-16T00:00:00Z', 'root_sha256': 'f' * 64,
            'files': 419, 'bytes': 548525078}}


class Engine:
    def __init__(self):
        self.engine_uid, self.deadline = 65534, time.monotonic() + 299
        self.state, self.deleted, self.calls = None, False, []
        self.lose_create = self.cleanup_fail = False

    def json(self, method, path, value=None):
        self.calls.append((method, path))
        if path.startswith('/images/'):
            selected = approval()
            return {'Id': selected['image'], 'Os': 'linux', 'Architecture': 'amd64',
                    'RootFS': {'Type': 'layers', 'Layers': ['sha256:' + selected['layer_sha256']]},
                    'Config': {'User': '65534:65534', 'Entrypoint': ['/ui-supervisor'],
                               'Cmd': None, 'Env': lifecycle.configuration(selected, 'e' * 32)['Env'], 'WorkingDir': '/tmp'}}
        if path.startswith('/volumes/'):
            pin = approval()['material']
            return {'Name': pin['name'], 'Driver': 'local', 'Scope': 'local', 'Options': None,
                    'CreatedAt': pin['created_at'], 'Labels': {'zryna410.materials-nonce': pin['nonce'],
                    'zryna410.materials-root': pin['root_sha256']}}
        if path.startswith('/containers/json?'):
            return [] if self.state is None or self.deleted else [{'Id': CID}]
        if path.startswith('/containers/create?'):
            name = path.split('name=')[1]
            self.state = {'Id': CID, 'Name': '/' + name, 'Image': value['Image'],
                'Config': {key: copy.deepcopy(item) for key, item in value.items() if key != 'HostConfig'},
                'HostConfig': copy.deepcopy(value['HostConfig']), 'RestartCount': 0,
                'Mounts': [{'Type': 'volume', 'Name': approval()['material']['name'],
                            'Destination': '/materials', 'RW': False, 'Driver': 'local'}],
                'State': {'Status': 'created', 'Pid': 0, 'Running': False, 'Paused': False,
                          'Restarting': False, 'OOMKilled': False, 'Error': '', 'ExitCode': 0,
                          'StartedAt': '0001-01-01T00:00:00Z', 'FinishedAt': '0001-01-01T00:00:00Z'}}
            if self.lose_create:
                raise TimeoutError('fixture lost create response')
            return {'Id': CID}
        return copy.deepcopy(self.state)

    def request(self, method, path, **kwargs):
        self.calls.append((method, path))
        if self.cleanup_fail and ('/kill' in path or method == 'DELETE'):
            raise TimeoutError('fixture cleanup failure')
        if path.endswith('/start'):
            self.state['State'].update(Status='running', Pid=321, Running=True)
        elif '/kill?' in path:
            self.state['State'].update(Status='exited', Pid=0, Running=False, ExitCode=137)
        elif '/wait?' in path:
            return json.dumps({'StatusCode': self.state['State']['ExitCode']}).encode(), None
        elif method == 'DELETE':
            self.deleted = True
        elif method == 'GET' and path.endswith('/json'):
            return json.dumps({'message': 'No such container'} if self.deleted else self.state).encode(), None
        return b'', None


class LifecycleTests(unittest.TestCase):
    def setUp(self):
        self.engine = Engine()
        self.owner = lifecycle.Owner(approval(), self.engine)
        self.closed = False
        stream = mock.Mock()
        stream.close.side_effect = lambda: setattr(self, 'closed', True)
        self.stack = []
        for patch in [mock.patch.object(lifecycle, 'attach', return_value=stream),
            mock.patch.object(lifecycle, 'run_proc', return_value=object()),
            mock.patch.object(lifecycle, 'proc_description', return_value={'execId': 'c' * 64,
                'receipt': {'format': 'zryna.playground-ui-proc-audit.v1', 'version': 1, 'sysrqTrigger': 'absent-enoent'}}),
            mock.patch.object(lifecycle.kernel, 'initial', return_value={'pid': 321, 'ticks': '100',
                              'cid': CID, 'membership': '0::/../docker/' + CID + '\n', 'startedAt': 'fixture-start'}),
            mock.patch.object(lifecycle.kernel, 'capture', return_value={'pid': 321, 'ticks': '100',
                              'cid': CID, 'membership': '0::/../docker/' + CID + '\n', 'startedAt': 'fixture-start'}),
            mock.patch.object(lifecycle.kernel, 'gone', return_value={'originalInitAbsent': True, 'cgroup': 'removed'}),
            mock.patch.object(lifecycle, 'audit', return_value={'receipt': {'version': 1, 'rootSha256': 'f' * 64,
                                                                   'files': 419, 'bytes': 548525078}}),
            mock.patch.object(lifecycle, 'Engine', return_value=self.engine)]:
            self.stack.append(patch);patch.start()
        self.addCleanup(lambda: [patch.stop() for patch in reversed(self.stack)])

    def test_audit_before_ready_and_single_admission_cleanup(self):
        self.owner.start()
        self.assertTrue(self.owner.ready)
        with self.assertRaises(ValueError):
            self.owner.start()
        receipt = self.owner.stop()
        calls = len(self.engine.calls)
        self.assertIs(self.owner.stop(), receipt)
        self.assertEqual(calls, len(self.engine.calls))
        self.assertTrue(receipt['removed']);self.assertTrue(self.closed)
        self.assertEqual(receipt['reap']['cgroup'], 'removed')
        self.assertEqual(receipt['wait']['StatusCode'], 137)

    def test_audit_failure_never_ready_and_still_cleans(self):
        with mock.patch.object(lifecycle, 'audit', side_effect=ValueError('fixture invalid material')):
            with self.assertRaises(ValueError):
                self.owner.start()
        self.assertFalse(self.owner.ready)
        self.assertTrue(self.owner.stop()['removed'])

    def test_detailed_kernel_failure_keeps_provisional_init_for_proven_reap(self):
        with mock.patch.object(lifecycle.kernel, 'capture', side_effect=ValueError('fixture missing observation')):
            with self.assertRaises(ValueError):
                self.owner.start()
        record = self.owner.stop()
        self.assertFalse(self.owner.ready);self.assertTrue(record['removed'])
        self.assertTrue(record['dockerRemoved']);self.assertTrue(record['kernelReapProven'])
        self.assertEqual(record['initialIdentity']['ticks'], '100')

    def test_native_proc_audit_is_bracketed_by_original_init_identity(self):
        for changed in [{'ticks': 'changed'}, {'pid': 322}, {'startedAt': 'later-start'}]:
            with self.subTest(changed=changed):
                initial = {'pid': 321, 'ticks': '100', 'cid': CID,
                           'membership': '0::/../docker/' + CID + '\n', 'startedAt': 'fixture-start'}
                owner = lifecycle.Owner(approval(), Engine())
                with mock.patch.object(lifecycle.kernel, 'capture', return_value={**initial, **changed}):
                    with self.assertRaises(ValueError):
                        owner.start()
                self.assertEqual(owner.init, initial)
                self.assertFalse(owner.ready)

    def test_unknown_kernel_identity_cannot_be_hidden_by_successful_docker_removal(self):
        with mock.patch.object(lifecycle.kernel, 'initial', side_effect=PermissionError('fixture unknown identity')):
            with self.assertRaises(PermissionError):
                self.owner.start()
            record = self.owner.stop()
        self.assertTrue(record['dockerRemoved']);self.assertFalse(record['kernelReapProven'])
        self.assertFalse(record['removed']);self.assertTrue(record['ownershipUnconfirmed'])
        self.assertEqual(record['kernelIdentityFailure'], 'PermissionError')

    def test_attach_failure_created_container_removed_without_wait_or_kill(self):
        with mock.patch.object(lifecycle, 'attach', side_effect=TimeoutError('fixture attach refused')):
            with self.assertRaises(TimeoutError):
                self.owner.start()
        self.assertTrue(self.owner.stop()['removed'])
        self.assertFalse(any('/wait?' in path or '/kill?' in path for _, path in self.engine.calls))

    def test_nonzero_finished_time_cannot_claim_never_started_cleanup(self):
        with mock.patch.object(lifecycle, 'attach', side_effect=TimeoutError('fixture attach refused')):
            with self.assertRaises(TimeoutError):
                self.owner.start()
        self.engine.state['State']['FinishedAt'] = '2026-10-16T00:00:00Z'
        record = self.owner.stop()
        self.assertTrue(record['dockerRemoved']);self.assertFalse(record['removed'])
        self.assertTrue(record['ownershipUnconfirmed']);self.assertFalse(record['kernelReapProven'])

    def test_lost_create_response_recovers_exact_created_name(self):
        self.engine.lose_create = True
        with self.assertRaises(TimeoutError):
            self.owner.start()
        self.assertIsNone(self.owner.cid)
        self.assertTrue(self.owner.stop()['removed'])
        self.assertEqual(self.owner.cid, CID)

    def test_cleanup_failure_is_sticky_and_retains_original_owner(self):
        self.owner.start();self.engine.cleanup_fail = True
        record = self.owner.stop();deadline = self.owner.cleanup_deadline
        self.assertFalse(record['removed']);self.assertTrue(record['ownershipUnconfirmed'])
        self.assertEqual(record['containerId'], CID);self.assertEqual(record['cleanupFailure'], 'TimeoutError')
        self.assertIs(self.owner.stop(), record);self.assertEqual(self.owner.cleanup_deadline, deadline)

    def test_actual_resource_or_start_failure_does_not_prevent_owned_cleanup(self):
        self.owner.start()
        self.engine.state['State'].update(OOMKilled=True, Error='fixture recorded resource failure')
        self.assertTrue(self.owner.stop()['removed'])

    def test_substitution_during_cleanup_preserves_owner_without_killing_foreign_object(self):
        self.owner.start();self.engine.state['Name'] = '/foreign'
        before = len(self.engine.calls)
        record = self.owner.stop()
        self.assertTrue(record['ownershipUnconfirmed']);self.assertFalse(record['removed'])
        self.assertFalse(any('/kill?' in path or method == 'DELETE' for method, path in self.engine.calls[before:]))

    def test_cleanup_uses_shared_earlier_monotonic_limit_and_never_renews_it(self):
        self.owner.start();limit = time.monotonic() + 0.5
        self.assertTrue(self.owner.stop(limit)['removed'])
        self.assertEqual(self.owner.cleanup_deadline, limit)
        self.owner.stop(time.monotonic() + 2)
        self.assertEqual(self.owner.cleanup_deadline, limit)

    def test_substituted_owner_or_extra_host_authority_is_never_killed(self):
        for extra in [{'CapAdd': ['SYS_ADMIN']}, {'DeviceCgroupRules': ['c *:* rwm']},
                      {'CgroupParent': '/foreign'}, {'UnknownAuthority': True}, {'ReadonlyPaths': ['/proc']},
                      {'ReadonlyPaths': ['/proc/sys']}]:
            with self.subTest(extra=extra):
                selected = approval();expected = lifecycle.configuration(selected, 'e' * 32)
                self.engine.json('POST', '/containers/create?name=zryna410-ui-' + 'e' * 32, expected)
                state = copy.deepcopy(self.engine.state);state['HostConfig'].update(extra)
                with self.assertRaises(ValueError):
                    identity.container(state, CID, 'zryna410-ui-' + 'e' * 32, expected)

    def test_only_false_and_docker_null_oom_readback_preserve_enabled_killer(self):
        expected = lifecycle.configuration(approval(), 'e' * 32)
        self.engine.json('POST', '/containers/create?name=zryna410-ui-' + 'e' * 32, expected)
        for value in [False, None]:
            state = copy.deepcopy(self.engine.state);state['HostConfig']['OomKillDisable'] = value
            identity.container(state, CID, 'zryna410-ui-' + 'e' * 32, expected)
        for value in [True, 0, 1, 'false', {}, []]:
            state = copy.deepcopy(self.engine.state);state['HostConfig']['OomKillDisable'] = value
            with self.assertRaises(ValueError):
                identity.container(state, CID, 'zryna410-ui-' + 'e' * 32, expected)
        state = copy.deepcopy(self.engine.state);del state['HostConfig']['OomKillDisable']
        with self.assertRaises(ValueError):
            identity.container(state, CID, 'zryna410-ui-' + 'e' * 32, expected)

    def test_docker_null_oom_readback_does_not_strand_created_running_owner(self):
        self.owner.start();self.engine.state['HostConfig']['OomKillDisable'] = None
        self.assertTrue(self.owner.stop()['removed'])

    def test_approval_cannot_select_socket_user_network_or_command(self):
        for changed in [{'socket': '/tmp/engine.sock'}, {'version': True}, {'material': {}}]:
            with self.assertRaises((ValueError, KeyError)):
                owner_config.approval({**approval(), **changed})


if __name__ == '__main__':
    unittest.main()
