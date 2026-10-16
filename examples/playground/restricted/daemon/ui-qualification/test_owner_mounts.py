"""Actual compiler mount replay proves topology only; UI numeric fixture is synthetic."""
import hashlib
import json
from pathlib import Path
import unittest
import owner_mounts
import owner_kernel
import owner_audit
from test_owner_proc_audit import Engine as AuditEngine

FIXTURE = Path(__file__).parent / 'fixtures' / 'actual-compiler-mountinfo.txt'
PROVENANCE = json.loads(FIXTURE.with_name('actual-compiler-mountinfo.provenance.json').read_text())
CID = PROVENANCE['containerId']


def change(text, mount_id, transform):
    return '\n'.join(transform(line) if line.startswith(str(mount_id) + ' ') else line
                     for line in text.splitlines()) + '\n'


def synthetic_ui_proc(actual):
    text = '\n'.join(line for line in actual.splitlines() if not line.startswith('214 ')) + '\n'
    for mount_id in [215, 216, 217, 218]:
        text = change(text, mount_id, lambda line: line.replace(' 214 ', ' 229 ', 1))
    return text + ''.join(f'{index} 229 0:81 {path[5:]} {path} ro,nosuid,nodev,noexec - proc proc rw\n'
                          for index, path in enumerate(owner_mounts.PROC_READONLY, 250))


class MountTests(unittest.TestCase):
    def setUp(self):
        data = FIXTURE.read_bytes()
        self.assertEqual(hashlib.sha256(data).hexdigest(), PROVENANCE['fixture']['sha256'])
        self.assertEqual(len(data), PROVENANCE['fixture']['bytes'])
        self.actual = data.decode('ascii')
        self.ui = synthetic_ui_proc(self.actual)

    def test_actual_parent_graph_resolves_lower_numbered_effective_mounts_in_any_row_order(self):
        for text in [self.actual, '\n'.join(reversed(self.actual.splitlines())) + '\n']:
            result = owner_mounts.topology(text, CID)
            self.assertEqual(result['/proc']['id'], 214)
            self.assertEqual(result['/proc']['stack']['base']['id'], 229)
            for target, top, base in [('/etc/hostname', 220, 239), ('/etc/hosts', 221, 240),
                                      ('/etc/resolv.conf', 222, 238)]:
                self.assertEqual(result[target]['id'], top)
                self.assertEqual(result[target]['stack']['base']['id'], base)
                self.assertEqual(result[target]['root'], '/null')
            self.assertIn('ro', result['/materials']['options'])

    def test_actual_compiler_sizes_cannot_pass_ui_budget_synthetic_ui_sizes_can(self):
        with self.assertRaises(ValueError):
            owner_kernel.mounts(self.actual, CID)
        with self.assertRaises(ValueError):
            owner_kernel.mounts(self.ui, CID)
        synthetic = change(self.ui, 233, lambda line: line.replace('size=65536k', 'size=131072k'))
        synthetic = change(synthetic, 237, lambda line: line.replace('size=1024k', 'size=65536k'))
        self.assertEqual(owner_kernel.mounts(synthetic, CID)['/dev/shm']['superOptions'][-1], 'size=65536k')

    def test_ui_requires_five_protected_subpaths_and_present_etc_bind_requires_mask(self):
        single_rw = '\n'.join(line for line in self.actual.splitlines() if not line.startswith('214 ')) + '\n'
        for mount_id in [215, 216, 217, 218]:
            single_rw = change(single_rw, mount_id, lambda line: line.replace(' 214 ', ' 229 ', 1))
        with self.assertRaises(ValueError):
            owner_mounts.resolve(single_rw, CID)
        single_ro = '\n'.join(line for line in self.actual.splitlines() if not line.startswith('229 ')) + '\n'
        single_ro = change(single_ro, 214, lambda line: line.replace('214 229 ', '214 227 '))
        with self.assertRaises(ValueError):
            owner_mounts.resolve(single_ro, CID)
        for text in [self.ui, '\n'.join(reversed(self.ui.splitlines())) + '\n']:
            self.assertEqual(owner_mounts.resolve(text, CID)['/proc']['id'], 229)
        unmasked = '\n'.join(line for line in self.ui.splitlines() if not line.startswith('220 ')) + '\n'
        with self.assertRaises(ValueError):
            owner_mounts.resolve(unmasked, CID)

    def test_ui_protection_missing_writable_wrong_source_or_subtree_override_fails(self):
        changed = [change(self.ui, 229, lambda line: line.replace(' rw,nosuid', ' ro,nosuid')),
                   change(self.ui, 229, lambda line: line.replace('rw,nosuid,nodev,noexec', 'rw,nosuid,nodev'))]
        for mount_id in range(250, 255):
            changed += ['\n'.join(line for line in self.ui.splitlines() if not line.startswith(str(mount_id) + ' ')) + '\n',
                        change(self.ui, mount_id, lambda line: line.replace(' ro,nosuid', ' rw,nosuid')),
                        change(self.ui, mount_id, lambda line: line.replace('0:81', '0:999')),
                        change(self.ui, mount_id, lambda line: line.replace(' - proc proc ', ' - tmpfs tmpfs ')),
                        change(self.ui, mount_id, lambda line: line.replace(' 229 ', ' 227 ', 1))]
        changed += [self.ui + '999 253 0:99 / /proc/sys/over ro - tmpfs tmpfs ro\n']
        for text in changed:
            with self.subTest(text=text[-100:]), self.assertRaises(ValueError):
                owner_mounts.resolve(text, CID)

    def test_absent_fifth_requires_original_target_bound_fixed_exec_proof_and_presence_agreement(self):
        missing = '\n'.join(line for line in self.ui.splitlines() if not line.startswith('254 ')) + '\n'
        absent = owner_audit.run_proc(AuditEngine('absent-enoent', CID), CID)
        present = owner_audit.run_proc(AuditEngine('present-readonly', CID), CID)
        self.assertNotIn('/proc/sysrq-trigger', owner_mounts.resolve(missing, CID, absent))
        self.assertIn('/proc/sysrq-trigger', owner_mounts.resolve(self.ui, CID, present))
        for text, proof in [(missing, None), (missing, present), (self.ui, absent),
                            (missing, owner_audit.proc_description(absent, CID))]:
            with self.assertRaises(ValueError):
                owner_mounts.resolve(text, CID, proof)
        with self.assertRaises(ValueError):
            owner_mounts.resolve(missing, 'f' * 64, absent)
        contradictory = missing + '999 229 0:99 / /proc/sysrq-trigger/hidden ro - tmpfs tmpfs ro\n'
        with self.assertRaises(ValueError):
            owner_mounts.resolve(contradictory, CID, absent)

    def test_actual_failed_ui_four_mounts_replay_needs_synthetic_typed_proof_not_generic_skip(self):
        path = FIXTURE.with_name('actual-ui-proc4-mountinfo.txt')
        provenance = json.loads(path.with_name('actual-ui-proc4-mountinfo.provenance.json').read_text())
        data = path.read_bytes();cid = provenance['containerId']
        self.assertEqual(hashlib.sha256(data).hexdigest(), provenance['fixture']['sha256'])
        with self.assertRaises(ValueError):
            owner_kernel.mounts(data.decode(), cid)
        proof = owner_audit.run_proc(AuditEngine('absent-enoent', cid), cid)
        self.assertNotIn('/proc/sysrq-trigger', owner_kernel.mounts(data.decode(), cid, proof))

    def test_disconnected_cyclic_extra_layers_duplicate_id_and_hidden_children_fail(self):
        changed = [change(self.actual, 214, lambda line: line.replace('214 229 ', '214 227 ')),
                   change(self.actual, 229, lambda line: line.replace('229 227 ', '229 214 ')),
                   self.actual + '999 214 0:81 / /proc ro,nosuid,nodev,noexec - proc proc rw\n',
                   self.actual + '214 227 0:99 / /unexpected ro - tmpfs tmpfs ro\n',
                   change(self.actual, 216, lambda line: line.replace('216 214 ', '216 229 ')),
                   change(self.actual, 233, lambda line: line.replace('233 227 ', '233 999 '))]
        for text in changed:
            with self.subTest(text=text[-100:]), self.assertRaises(ValueError):
                owner_mounts.topology(text, CID)

    def test_proc_remount_and_null_masks_require_exact_filesystem_identity_root_and_flags(self):
        changed = [change(self.actual, 214, lambda line: line.replace('0:81', '0:999')),
                   change(self.actual, 214, lambda line: line.replace(' ro,nosuid', ' rw,nosuid')),
                   change(self.actual, 220, lambda line: line.replace(' /null ', ' /arbitrary ')),
                   change(self.actual, 220, lambda line: line.replace('0:83', '0:999')),
                   change(self.actual, 239, lambda line: line.replace(CID, 'b' * 64)),
                   change(self.actual, 220, lambda line: line.replace('rw,nosuid,noexec', 'rw,nosuid')),
                   change(self.actual, 237, lambda line: line.replace('237 232 ', '237 235 '))]
        for text in changed:
            with self.subTest(text=text[-100:]), self.assertRaises(ValueError):
                owner_mounts.topology(text, CID)

    def test_protected_root_material_image_and_scratch_stacks_or_overmounts_are_rejected(self):
        for extra in ['999 227 0:99 / / ro - tmpfs tmpfs ro',
                      '999 235 0:99 / /materials ro - tmpfs tmpfs ro',
                      '999 235 0:99 / /materials/hidden ro - tmpfs tmpfs ro',
                      '999 227 0:99 / /app/compiler ro - tmpfs tmpfs ro',
                      '999 227 0:99 / /resources/hidden ro - tmpfs tmpfs ro',
                      '999 233 0:99 / /tmp rw - tmpfs tmpfs rw',
                      '999 237 0:99 / /dev/shm rw - tmpfs tmpfs rw',
                      '999 233 0:99 / /tmp/hidden rw - tmpfs tmpfs rw',
                      '999 237 0:99 / /dev/shm/hidden rw - tmpfs tmpfs rw']:
            with self.subTest(extra=extra), self.assertRaises(ValueError):
                owner_mounts.topology(self.actual + extra + '\n', CID)


if __name__ == '__main__':
    unittest.main()
