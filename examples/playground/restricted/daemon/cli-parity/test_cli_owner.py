"""Actual owner sequencing over inert Engine/kernel doubles only."""
import copy
import time
import unittest
from unittest import mock
import cli_owner
from cli_plan import operation
from test_cli_helpers import CID, Engine, approval


class OwnerTests(unittest.TestCase):
    def setUp(self):
        self.operation = operation('build-addition')
        self.engine = Engine(self.operation)
        self.owner = cli_owner.Owner(approval(), self.operation['id'], self.engine)
        self.init = {'pid': 321, 'ticks': '100', 'cid': CID, 'membership': '0::/docker/' + CID + '\n',
                     'startedAt': '2026-10-16T00:00:00Z'}
        self.stack = [mock.patch.object(cli_owner, 'Engine', return_value=self.engine),
            mock.patch.object(cli_owner.kernel, 'ticks', return_value='100'),
            mock.patch.object(cli_owner.kernel, 'group_path', return_value='/sys/fs/cgroup/docker/' + CID),
            mock.patch.object(cli_owner.kernel, 'read', return_value=self.init['membership']),
            mock.patch.object(cli_owner.kernel, 'capture', return_value=copy.deepcopy(self.init)),
            mock.patch.object(cli_owner.kernel, 'gone', return_value={'originalInitAbsent': True, 'cgroup': 'removed'})]
        for patch in self.stack:
            patch.start()
        self.addCleanup(lambda: [patch.stop() for patch in reversed(self.stack)])

    def test_fixed_source_checked_before_exec_actual_capture_and_shared_cleanup(self):
        result = self.owner.run()
        self.assertEqual(result['operationStatus'], 'captured-source-outcome')
        self.assertTrue(result['cleanupConfirmed']); self.assertTrue(self.engine.deleted)
        self.assertEqual(len(result['kernel']), 3); self.assertEqual(result['ownedVolumes'], [])
        self.assertTrue(result['projectBeforeUpload']['empty'])
        self.assertEqual(len(result['projectAfterUpload']['files']), 3)
        self.assertEqual(result['exec']['capture']['exitCode'], 0)
        calls = len(self.engine.calls)
        self.assertIs(self.owner.stop(), result); self.assertEqual(len(self.engine.calls), calls)
        with self.assertRaises(ValueError):
            self.owner.run()

    def test_before_kernel_failure_never_uploads_or_executes_and_retains_original_reap_identity(self):
        with mock.patch.object(cli_owner.kernel, 'capture', side_effect=PermissionError('inert proc denied')):
            result = self.owner.run()
        self.assertFalse(self.engine.exec_started); self.assertFalse(self.engine.populated)
        self.assertTrue(result['cleanupConfirmed']); self.assertTrue(result['removed'])
        self.assertEqual(result['failure'], {'phase': 'before-kernel', 'type': 'PermissionError'})
        self.assertIn('reap', result)

    def test_changed_source_readback_never_executes(self):
        def stage(engine, cid, identifier, record):
            engine.populated = True
            raise ValueError('inert changed staged source')
        with mock.patch.object(cli_owner, 'project_stage', side_effect=stage):
            result = self.owner.run()
        self.assertTrue(self.engine.populated); self.assertFalse(self.engine.exec_started)
        self.assertTrue(result['cleanupConfirmed'])
        self.assertEqual(result['failure']['phase'], 'fixed-source-upload')

    def test_lost_create_response_recovers_only_exact_never_started_owned_name(self):
        self.engine.lose_create = True
        result = self.owner.run()
        self.assertEqual(result['containerId'], CID); self.assertTrue(result['removed'])
        self.assertTrue(result['cleanupConfirmed']); self.assertFalse(self.engine.exec_started)
        self.assertNotIn('reap', result); self.assertIsNone(result['wait'])

    def test_unknown_teardown_is_sticky_and_cannot_become_capture_success(self):
        self.engine.cleanup_fail = True
        result = self.owner.run()
        self.assertTrue(self.engine.exec_started); self.assertFalse(result['cleanupConfirmed'])
        self.assertTrue(result['ownershipUnconfirmed']); self.assertEqual(result['operationStatus'], 'incomplete')
        self.engine.cleanup_fail = False
        calls = len(self.engine.calls)
        self.assertIs(self.owner.stop(), result); self.assertEqual(calls, len(self.engine.calls))

    def test_created_pid_zero_with_prior_start_timestamps_never_gets_never_started_cleanup_credit(self):
        original = self.engine.request
        def lost_start(method, path, **kwargs):
            if path.endswith('/start'):
                self.engine.state['State'].update(StartedAt='2026-10-16T00:00:00Z',
                                                 FinishedAt='2026-10-16T00:00:01Z')
                raise TimeoutError('inert ambiguous start')
            return original(method, path, **kwargs)
        with mock.patch.object(self.engine, 'request', side_effect=lost_start):
            result = self.owner.run()
        self.assertFalse(result['cleanupConfirmed']); self.assertTrue(result['ownershipUnconfirmed'])
        self.assertFalse(self.engine.exec_started); self.assertFalse(self.engine.populated)
        self.assertFalse(self.engine.deleted); self.assertEqual(result['failure']['phase'], 'start')

    def test_changed_process_configuration_is_never_started_or_deleted_as_original(self):
        original = self.engine.json
        def changed(method, path, value=None):
            result = original(method, path, value)
            if path.startswith('/containers/') and path.endswith('/json'):
                result['Config']['User'] = '0:0'
            return result
        with mock.patch.object(self.engine, 'json', side_effect=changed):
            result = self.owner.run()
        self.assertFalse(self.engine.exec_started); self.assertFalse(self.engine.deleted)
        self.assertFalse(result['cleanupConfirmed']); self.assertTrue(result['ownershipUnconfirmed'])

    def test_whole_active_deadline_and_wrong_peer_rejected_before_effects(self):
        self.engine.engine_uid = 0
        with self.assertRaises(ValueError):
            cli_owner.Owner(approval(), 'build-addition', self.engine)
        self.engine.engine_uid = 65534; self.engine.deadline = time.monotonic() + 11
        with self.assertRaises(ValueError):
            cli_owner.Owner(approval(), 'build-addition', self.engine)
        self.assertEqual(self.engine.calls, [])


if __name__ == '__main__':
    unittest.main()
