"""Pure policy, source tar and Engine stream negatives; no daemon or native user code."""
import copy
import io
from pathlib import Path
import struct
import tarfile
import unittest
from unittest import mock
from cli_archive import inspect_project
from cli_config import approval as validate_approval, configuration
from cli_exec import exec_identity, streams
from cli_plan import operation, project_archive
import cli_identity
from test_cli_helpers import CID, EID, Engine, approval, project_tar


class ContractTests(unittest.TestCase):
    def test_fixed19_plan_selector_rejects_unknown_and_changed_command_or_source(self):
        for value in ['shell', '../../run-addition-0', None, 'x' * 129]:
            with self.assertRaises(ValueError):
                operation(value)
        selected = operation('build-addition')
        self.assertLessEqual(len(project_archive(selected)), 32768)
        for mutate in [lambda x: x['argv'].append('--node=/bin/sh'),
                       lambda x: x['projectFiles'][0].update(bodyBase64='ZXZpbA==')]:
            changed = copy.deepcopy(selected); mutate(changed)
            with self.assertRaises(ValueError):
                project_archive(changed)

    def test_exact_policy_and_closed_approval_preserve_original_caps_and_finite_scratch(self):
        selected = validate_approval(approval())
        for mutate in [lambda x: x.update(seccomp=x['seccomp'] + ' '),
                       lambda x: x.update(command_plan_sha256='0' * 64), lambda x: x.update(extra=True)]:
            changed = copy.deepcopy(selected); mutate(changed)
            with self.assertRaises(ValueError):
                validate_approval(changed)
        config = configuration(selected, '1' * 32)
        host = config['HostConfig']
        self.assertEqual(host['PidsLimit'], 32); self.assertEqual(host['Memory'], 536870912)
        self.assertEqual(host['Mounts'], []); self.assertEqual(host['NetworkMode'], 'none')
        self.assertTrue(host['ReadonlyRootfs']); self.assertEqual(host['CapDrop'], ['ALL'])
        self.assertEqual(config['Cmd'], ['idle']); self.assertEqual(config['User'], '65534:65534')

    def test_actual_project_tar_has_exact_uid_modes_inventory_hashes_and_no_links_or_hidden_headers(self):
        selected = operation('build-addition')
        for populated in [False, True]:
            raw = project_tar(selected, populated)
            result = inspect_project(raw, selected, populated)
            self.assertTrue(result['fullInventoryChecked'])
            self.assertEqual(len(result['members']), 5 if populated else 1)
        raw = project_tar(selected, True)
        for index in [0, 512, 1024]:
            changed = bytearray(raw); changed[index] ^= 1
            with self.assertRaises((ValueError, tarfile.TarError)):
                inspect_project(bytes(changed), selected, True)
        for data in [raw + b'x' * 512, raw[:1024], raw + bytes(32768), project_tar(selected, False)]:
            with self.assertRaises((ValueError, tarfile.TarError)):
                inspect_project(data, selected, True)
        output = io.BytesIO()
        with tarfile.open(fileobj=output, mode='w', format=tarfile.GNU_FORMAT) as archive:
            info = tarfile.TarInfo('project'); info.type = tarfile.SYMTYPE; info.linkname = '/host'
            archive.addfile(info)
        with self.assertRaises(ValueError):
            inspect_project(output.getvalue(), selected, False)

    def test_cli_stream_complete_multiplex_framing_and_per_stream_caps_are_enforced(self):
        frame = lambda kind, body: struct.pack('>BBBBI', kind, 0, 0, 0, len(body)) + body
        self.assertEqual(streams(frame(1, b'{}') + frame(2, b'fixture')), (b'{}', b'fixture'))
        for raw in [b'', b'\x01', frame(0, b'x'), frame(1, b'x')[:-1], frame(1, b'x') + b'x',
                    frame(1, b'a' * 65536) + frame(1, b'x'), frame(2, b'a' * 65537),
                    frame(1, b'x') * 1025]:
            with self.assertRaises(ValueError):
                streams(raw)

    def test_container_readback_rejects_hidden_authority_and_original_image_mutation(self):
        selected = approval(); expected = configuration(selected, '1' * 32)
        engine = Engine(operation('build-addition'))
        engine.json('POST', '/containers/create?name=zryna410-cli-' + '1' * 32, expected)
        state = engine.state
        cli_identity.container(state, CID, 'zryna410-cli-' + '1' * 32, expected)
        for mutate in [lambda x: x['HostConfig'].update(CapAdd=['SYS_ADMIN']),
                       lambda x: x['HostConfig'].update(NetworkMode='host'),
                       lambda x: x['HostConfig'].update(Binds=['/:/host']),
                       lambda x: x['HostConfig'].update(ExtraUnknown=True),
                       lambda x: x['Config'].update(User='0:0'),
                       lambda x: x['Mounts'].append({'Type': 'volume', 'Destination': '/input'})]:
            changed = copy.deepcopy(state); mutate(changed)
            with self.assertRaises(ValueError):
                cli_identity.container(changed, CID, 'zryna410-cli-' + '1' * 32, expected, cleanup=True)
        image = engine.json('GET', '/images/' + selected['image'] + '/json')
        cli_identity.image(image, selected)
        image['RootFS']['Layers'].append('sha256:' + 'f' * 64)
        with self.assertRaises(ValueError):
            cli_identity.image(image, selected)


if __name__ == '__main__':
    unittest.main()
