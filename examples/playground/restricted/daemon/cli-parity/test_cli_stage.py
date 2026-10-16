"""Inert fixed-command and live-staging receipt negatives; no installed runtime execution."""
import base64
import copy
import json
import struct
import unittest
from unittest import mock
import cli_stage
from cli_plan import operation
from test_cli_helpers import CID, Engine, stage_receipt


class StageTests(unittest.TestCase):
    def test_fixed_bootstrap_source_and_closed_source_data_are_the_only_selected_arguments(self):
        command = cli_stage.command('build-addition')
        self.assertEqual(command['executable'], '/opt/zryna-release/runtime/node/bin/node')
        self.assertEqual(command['argv'][0], '--eval'); self.assertEqual(len(command['argv']), 3)
        self.assertTrue(command['argv'][1].endswith(cli_stage.ENTRY))
        request = json.loads(base64.b64decode(command['argv'][2], validate=True))
        self.assertEqual(request, {'version': 1, 'operationId': 'build-addition',
                                  'files': operation('build-addition')['projectFiles']})
        with self.assertRaises(ValueError):
            cli_stage.command('shell')
        with mock.patch.object(cli_stage, 'SCRIPT_SHA256', '0' * 64), self.assertRaises(ValueError):
            cli_stage.command('build-addition')

    def test_staging_receipt_requires_actual_fresh_tmpfs_root_complete_inventory_and_source_identity(self):
        selected = operation('build-addition'); good = stage_receipt(selected)
        self.assertEqual(cli_stage.receipt(good, selected['id']), good)
        for mutate in [lambda v: v.update(filesystem='overlay'), lambda v: v['before'].update(empty=False),
                       lambda v: v['before']['root'].update(uid=0, mode=0o555),
                       lambda v: v['after']['root'].update(ino=789),
                       lambda v: v['after']['files'].pop(), lambda v: v['after']['files'][0].update(sha256='0' * 64),
                       lambda v: v['after']['files'][0].update(nlink=True),
                       lambda v: v['after']['src'].update(sameDevice=1),
                       lambda v: v['after']['files'][0].update(path='../../outside')]:
            changed = copy.deepcopy(good); mutate(changed)
            with self.assertRaises(ValueError):
                cli_stage.receipt(changed, selected['id'])

    def test_fixed_staging_exec_retains_real_status_stream_and_readback_before_cli(self):
        selected = operation('build-addition'); engine = Engine(selected); record = {}
        result = cli_stage.run(engine, CID, selected['id'], record)
        self.assertTrue(engine.stage_started); self.assertFalse(engine.exec_started)
        self.assertTrue(result['before']['empty']); self.assertEqual(record['final']['ExitCode'], 0)
        self.assertFalse(record['request']['AttachStdin']); self.assertEqual(record['request']['User'], '65534:65534')
        self.assertEqual(record['verified'], result)

    def test_changed_actual_exec_selectors_stderr_failed_exit_or_false_receipt_never_admit(self):
        for kind in ['selector', 'stderr', 'exit', 'receipt']:
            selected = operation('build-addition'); engine = Engine(selected)
            original_json, original_request = engine.json, engine.request
            def changed_json(method, path, value=None):
                result = original_json(method, path, value)
                if path.startswith('/exec/'):
                    if kind == 'selector': result['ProcessConfig']['user'] = '0:0'
                    if kind == 'exit': result['ExitCode'] = 70
                return result
            def changed_request(method, path, **kwargs):
                result = original_request(method, path, **kwargs)
                if path.startswith('/exec/') and path.endswith('/start'):
                    if kind == 'stderr':
                        return result[0] + struct.pack('>BBBBI', 2, 0, 0, 0, 1) + b'x', None
                    if kind == 'receipt':
                        raw = json.dumps(stage_receipt(selected) | {'filesystem': 'overlay'}).encode()
                        return struct.pack('>BBBBI', 1, 0, 0, 0, len(raw)) + raw, None
                return result
            with mock.patch.object(engine, 'json', side_effect=changed_json), \
                 mock.patch.object(engine, 'request', side_effect=changed_request), self.assertRaises(ValueError):
                cli_stage.run(engine, CID, selected['id'], {})
            self.assertFalse(engine.exec_started)


if __name__ == '__main__':
    unittest.main()
