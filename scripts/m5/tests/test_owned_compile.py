"""Data-only exact compiler-artifact selection and source mirror controls."""
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import owned_compile
import owned_inputs
from capture_io import sha


class OwnedCompilerInputs(unittest.TestCase):
    def fixture(self):
        repo = Path('/pure-fixture/repo')
        deps = Path('/pure-fixture/target/debug/deps')
        package = {'id': 'driver-id', 'name': 'zryna-driver', 'version': '0.2.3',
                   'manifest_path': str(repo / 'crates/zryna-driver/Cargo.toml')}
        metadata = {'packages': [package], 'resolve': {'nodes': [{
            'id': 'driver-id', 'deps': [
                {'name': 'serde', 'pkg': 'serde-id', 'dep_kinds': [{'kind': None, 'target': None}]},
                {'name': 'wasmparser', 'pkg': 'wasmparser-id', 'dep_kinds': [{'kind': 'dev', 'target': None}]}]}]}}
        messages = [{'reason': 'compiler-artifact', 'package_id': name + '-id',
                     'target': {'kind': ['lib']}, 'profile': {'opt_level': '0', 'test': False, 'debuginfo': 0},
                     'features': [], 'filenames': [str(deps / ('lib' + name + '-abc.rlib'))]}
                    for name in ('serde', 'wasmparser')]
        manifest = b'[dependencies]\nserde="1"\n[dev-dependencies]\nwasmparser="1"\n'
        return repo, deps, metadata, messages, manifest

    def select(self, fixture):
        repo, deps, metadata, messages, manifest = fixture
        with patch.object(owned_compile, 'read', return_value=manifest), patch.object(
                owned_compile, 'file_pin', side_effect=lambda path: {'path': str(path)}):
            return owned_compile.direct_externs(repo, metadata, messages, deps)

    def test_exact_normal_and_dev_package_ids_are_selected(self):
        rows, version = self.select(self.fixture())
        self.assertEqual([r['packageID'] for r in rows], ['serde-id', 'wasmparser-id'])
        self.assertEqual(version, '0.2.3')

    def test_missing_dev_library_rejects_instead_of_silent_partial_compile(self):
        fixture = self.fixture()
        fixture[3].pop()
        with self.assertRaises(AssertionError):
            self.select(fixture)

    def test_two_same_package_libraries_reject_ambiguity(self):
        fixture = self.fixture()
        fixture[3].append(dict(fixture[3][0]))
        with self.assertRaises(AssertionError):
            self.select(fixture)

    def test_artifact_outside_selected_dependency_directory_rejects(self):
        fixture = self.fixture()
        fixture[3][0]['filenames'] = ['/pure-fixture/other/libserde-abc.rlib']
        with self.assertRaises(AssertionError):
            self.select(fixture)

    def test_exact_full_file_epoch_and_hash_detect_replacement(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'input.rlib'
            path.write_bytes(b'original library bytes')
            first = owned_inputs.file_pin(path)
            path.write_bytes(b'changed! library bytes')
            second = owned_inputs.file_pin(path)
            self.assertNotEqual(first['sha256'], second['sha256'])
            self.assertNotEqual(first['epoch'], second['epoch'])

    def test_normal_mirror_appends_only_fixed_main_and_rejects_extra_file(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo, output = root / 'repo', root / 'output'
            lib = repo / 'crates/zryna-driver/src/lib.rs'
            main = repo / 'scripts/m5/owned_sanitizer_runtime_main.rs'
            lib.parent.mkdir(parents=True)
            main.parent.mkdir(parents=True)
            output.mkdir()
            original = b'// exact source-only fixture\n'
            entry = b'fn main() {}\n'
            lib.write_bytes(original)
            main.write_bytes(entry)
            source = {'files': [{'path': 'crates/zryna-driver/src/lib.rs',
                                'bytes': len(original), 'sha256': sha(original)}]}
            mirror, manifest = owned_inputs.mirror(repo, output, source)
            self.assertEqual((mirror / source['files'][0]['path']).read_bytes(), original + b'\n' + entry)
            owned_inputs.check_mirror(repo, mirror, manifest)
            (mirror / 'extra.rs').write_bytes(b'undeclared')
            with self.assertRaises(AssertionError):
                owned_inputs.check_mirror(repo, mirror, manifest)

    def test_cargo_uses_only_pinned_rustc_despite_hostile_path_or_toolchain(self):
        tools = {'pins': [{'path': '/official/pinned/bin/rustc'}]}
        env = owned_compile.cargo_environment(tools, {'PATH': '/hostile/bin',
                                                     'RUSTUP_TOOLCHAIN': 'hostile'})
        self.assertEqual(env['RUSTC'], '/official/pinned/bin/rustc')
        for name in ('RUSTC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER'):
            with self.assertRaises(AssertionError):
                owned_compile.cargo_environment(tools, {name: '/hostile/compiler'})

    def test_product_rejects_nonelf_wrong_architecture_and_unexpected_kind(self):
        header = bytearray(64)
        header[:7] = b'\x7fELF\x02\x01\x01'
        header[16:18] = (3).to_bytes(2, 'little')
        header[18:20] = (62).to_bytes(2, 'little')
        header[20:24] = (1).to_bytes(4, 'little')
        self.assertEqual(owned_compile.product_format(header)['machine'], 62)
        for offset, value in [(0, 0), (16, 1), (18, 40)]:
            changed = bytearray(header)
            changed[offset] = value
            with self.assertRaises(AssertionError):
                owned_compile.product_format(changed)
