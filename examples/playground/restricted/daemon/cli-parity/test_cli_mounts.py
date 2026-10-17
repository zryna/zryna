"""Actual compiler topology replay; synthetic CLI scratch profile conveys no runtime acceptance."""
import hashlib
import json
from pathlib import Path
import unittest
import cli_mounts
import cli_kernel
import cli_proc

FIXTURE = Path(__file__).parent / 'fixtures' / 'actual-compiler-mountinfo.txt'
PROVENANCE = json.loads(FIXTURE.with_name('actual-compiler-mountinfo.provenance.json').read_text())
CID = PROVENANCE['containerId']


def change(text, mount_id, transform):
    return '\n'.join(transform(line) if line.startswith(str(mount_id) + ' ') else line
                     for line in text.splitlines()) + '\n'


class MountTests(unittest.TestCase):
    def setUp(self):
        data = FIXTURE.read_bytes()
        self.assertEqual(hashlib.sha256(data).hexdigest(), PROVENANCE['fixture']['sha256'])
        self.assertEqual(len(data), PROVENANCE['fixture']['bytes'])
        self.actual = data.decode('ascii')

    def test_actual_proc_and_masks_resolve_by_parent_graph_in_both_row_orders(self):
        for text in [self.actual, '\n'.join(reversed(self.actual.splitlines())) + '\n']:
            result = cli_mounts.resolve(text, CID)
            self.assertEqual(result['/proc']['id'], 214)
            self.assertEqual(result['/proc']['stack']['base']['id'], 229)
            for target, top, base in [('/etc/hostname', 220, 239), ('/etc/hosts', 221, 240),
                                      ('/etc/resolv.conf', 222, 238)]:
                self.assertEqual(result[target]['id'], top)
                self.assertEqual(result[target]['stack']['base']['id'], base)
            cli_proc.visible_proc(text)

    def test_actual_compiler_mounts_are_not_cli_role_and_explicit_synthetic_cli_profile_is(self):
        with self.assertRaises(ValueError):
            cli_kernel.mounts(self.actual, CID)
        synthetic = '\n'.join(line for line in self.actual.splitlines() if not line.startswith('235 ')) + '\n'
        synthetic = change(synthetic, 236, lambda line: line.replace('/input ro,', '/input rw,'))
        synthetic += '999 227 0:99 / /work/project rw,nosuid,nodev,noexec - tmpfs tmpfs rw,size=2048k,mode=700,uid=65534,gid=65534\n'
        self.assertEqual(cli_kernel.mounts(synthetic, CID)['/work/project']['superOptions'][1], 'size=2048k')

    def test_unrelated_duplicates_disconnected_or_cyclic_layers_and_hidden_stack_children_fail(self):
        values = [change(self.actual, 214, lambda line: line.replace('214 229 ', '214 227 ')),
                  change(self.actual, 229, lambda line: line.replace('229 227 ', '229 214 ')),
                  self.actual + '999 214 0:81 / /proc ro,nosuid,nodev,noexec - proc proc rw\n',
                  self.actual + '214 227 0:99 / /unexpected ro - tmpfs tmpfs ro\n',
                  change(self.actual, 216, lambda line: line.replace('216 214 ', '216 229 ')),
                  change(self.actual, 233, lambda line: line.replace('233 227 ', '233 999 '))]
        for text in values:
            with self.assertRaises(ValueError):
                cli_mounts.resolve(text, CID)

    def test_proc_device_flags_and_null_masks_must_match_exact_original_filesystem(self):
        values = [change(self.actual, 214, lambda line: line.replace('0:81', '0:999')),
                  change(self.actual, 214, lambda line: line.replace(' ro,nosuid', ' rw,nosuid')),
                  change(self.actual, 220, lambda line: line.replace(' /null ', ' /other ')),
                  change(self.actual, 220, lambda line: line.replace('0:83', '0:999')),
                  change(self.actual, 239, lambda line: line.replace(CID, 'b' * 64))]
        for text in values:
            with self.assertRaises(ValueError):
                cli_mounts.resolve(text, CID)
        for text in values[:2]:
            with self.assertRaises(ValueError):
                cli_proc.visible_proc(text)

    def test_protected_installation_scratch_and_root_cannot_acquire_stacked_or_subtree_mounts(self):
        for extra in ['999 227 0:99 / / ro - tmpfs tmpfs ro',
                      '999 227 0:99 / /opt/zryna-release ro - tmpfs tmpfs ro',
                      '999 227 0:99 / /opt/zryna-release/lib/hidden ro - tmpfs tmpfs ro',
                      '999 233 0:99 / /tmp rw - tmpfs tmpfs rw',
                      '999 237 0:99 / /dev/shm rw - tmpfs tmpfs rw']:
            with self.assertRaises(ValueError):
                cli_mounts.resolve(self.actual + extra + '\n', CID)

    def test_single_proc_still_requires_readonly_and_present_etc_binds_need_proven_null_masks(self):
        single = '\n'.join(line for line in self.actual.splitlines() if not line.startswith('229 ')) + '\n'
        single = change(single, 214, lambda line: line.replace('214 229 ', '214 227 '))
        self.assertIn('ro', cli_mounts.resolve(single, CID)['/proc']['options'])
        with self.assertRaises(ValueError):
            cli_mounts.resolve(change(single, 214, lambda line: line.replace(' ro,nosuid', ' rw,nosuid')), CID)
        missing_mask = '\n'.join(line for line in self.actual.splitlines() if not line.startswith('220 ')) + '\n'
        with self.assertRaises(ValueError):
            cli_mounts.resolve(missing_mask, CID)


if __name__ == '__main__':
    unittest.main()
