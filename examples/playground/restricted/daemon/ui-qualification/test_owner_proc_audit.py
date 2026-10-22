"""Inert fixed-Engine fixtures; no actual guest execution or kernel receipt credit."""
import copy
import json
import struct
import unittest
import owner_audit

CID, EID = 'a' * 64, 'b' * 64


class Engine:
    def __init__(self, outcome='absent-enoent', cid=CID):
        self.cid = cid
        self.receipt = {'format': 'zryna.playground-ui-proc-audit.v1', 'version': 1, 'sysrqTrigger': outcome}
        self.final = False
        self.changed = {}

    def json(self, method, path, value=None):
        if method == 'POST':
            assert path == '/containers/' + self.cid + '/exec'
            assert value['Cmd'] == ['/ui-supervisor', 'proc-audit'] and value['User'] == '65534:65534'
            assert value['AttachStdin'] is False and value['Privileged'] is False and value['WorkingDir'] == '/tmp'
            return {'Id': EID}
        assert path == '/exec/' + EID + '/json'
        record = {'ID': EID, 'ContainerID': self.cid, 'OpenStdin': False, 'OpenStdout': True, 'OpenStderr': True,
                  'ProcessConfig': {'entrypoint': '/ui-supervisor', 'arguments': ['proc-audit'],
                                    'privileged': False, 'tty': False, 'user': '65534:65534'},
                  'Running': False, 'ExitCode': 0}
        record.update(self.changed)
        return record

    def request(self, method, path, **kwargs):
        assert method == 'POST' and path == '/exec/' + EID + '/start' and kwargs['maximum_response'] == 4096
        assert kwargs['value'] == {'Detach': False, 'Tty': False}
        self.final = True
        raw = (json.dumps(self.receipt, separators=(',', ':')) + '\n').encode()
        return struct.pack('>BBBBI', 1, 0, 0, 0, len(raw)) + raw, None


def proof(outcome='absent-enoent'):
    return owner_audit.run_proc(Engine(outcome), CID)


class ProcAuditTests(unittest.TestCase):
    def test_native_outcome_exact_exec_identity_and_container_binding(self):
        for outcome in ['absent-enoent', 'present-readonly']:
            selected = proof(outcome)
            record = owner_audit.proc_description(selected, CID)
            self.assertEqual(record['receipt']['sysrqTrigger'], outcome)
            self.assertEqual(record['execId'], EID)
            with self.assertRaises(ValueError):
                owner_audit.proc_description(selected, 'c' * 64)

    def test_plain_json_copy_and_fresh_unminted_instance_cannot_authorize_absence(self):
        selected = proof()
        for value in [owner_audit.proc_description(selected, CID), copy.copy(selected), owner_audit._ProcProof()]:
            with self.assertRaises(ValueError):
                owner_audit.proc_description(value, CID)
        description = owner_audit.proc_description(selected, CID)
        description['receipt']['sysrqTrigger'] = 'present-readonly'
        self.assertEqual(owner_audit.proc_description(selected, CID)['receipt']['sysrqTrigger'], 'absent-enoent')

    def test_denied_or_unknown_native_outcomes_malformed_receipts_and_failed_exec_are_not_proof(self):
        for outcome in ['absent-eacces', 'absent-enotdir', 'absent-eloop', 'unknown']:
            with self.assertRaises(ValueError):
                proof(outcome)
        for changed in [{'version': True}, {'extra': True}, {'sysrqTrigger': None}]:
            engine = Engine();engine.receipt.update(changed)
            with self.assertRaises(ValueError):
                owner_audit.run_proc(engine, CID)
        for changed in [{'ContainerID': 'c' * 64}, {'ExitCode': 1}, {'ExitCode': False}, {'Running': True},
                        {'ProcessConfig': {'entrypoint': '/bin/sh'}}]:
            engine = Engine();engine.changed = changed
            with self.assertRaises((ValueError, KeyError)):
                owner_audit.run_proc(engine, CID)


if __name__ == '__main__':
    unittest.main()
