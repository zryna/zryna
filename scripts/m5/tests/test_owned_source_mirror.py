"""Exact source-mirror includes, byte bindings and no-extra-file controls."""
from pathlib import Path
import tempfile
import unittest
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import owned_inputs
from capture_io import sha


class OwnedSourceMirror(unittest.TestCase):
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
            for relative in owned_inputs.SOURCE_FILES:
                included = repo / relative
                included.parent.mkdir(parents=True, exist_ok=True)
                data = ('// exact cross-crate include: ' + relative).encode()
                included.write_bytes(data)
                source['files'].append({'path': relative, 'bytes': len(data), 'sha256': sha(data)})
            unrelated = 'scripts/m5/unselected.json'
            (repo / unrelated).write_bytes(b'{}')
            source['files'].append({'path': unrelated, 'bytes': 2, 'sha256': sha(b'{}')})
            mirror, manifest = owned_inputs.mirror(repo, output, source)
            self.assertEqual({row['path'] for row in manifest},
                             {row['path'] for row in source['files']} - {unrelated})
            self.assertIn('scripts/m5/sqlite_fixture.json', {row['path'] for row in manifest})
            self.assertFalse((mirror / unrelated).exists())
            for relative in owned_inputs.SOURCE_FILES:
                self.assertEqual((mirror / relative).read_bytes(), (repo / relative).read_bytes())
            self.assertEqual((mirror / source['files'][0]['path']).read_bytes(), original + b'\n' + entry)
            owned_inputs.check_mirror(repo, mirror, manifest)
            (mirror / 'extra.rs').write_bytes(b'undeclared')
            with self.assertRaises(AssertionError):
                owned_inputs.check_mirror(repo, mirror, manifest)

    def mirror_include_fixture(self, directory, omitted=None):
        repo, output = Path(directory) / 'repo', Path(directory) / 'output'
        output.mkdir()
        source = {'files': []}
        for relative in owned_inputs.SOURCE_FILES:
            if relative != omitted:
                included = repo / relative
                included.parent.mkdir(parents=True, exist_ok=True)
                data = b'// retained source include'
                included.write_bytes(data)
                source['files'].append({'path': relative, 'bytes': len(data), 'sha256': sha(data)})
        return repo, output, source

    def test_source_mirror_requires_every_fixed_cross_crate_include(self):
        for omitted in owned_inputs.SOURCE_FILES:
            with tempfile.TemporaryDirectory() as directory:
                repo, output, source = self.mirror_include_fixture(directory, omitted)
                with self.assertRaisesRegex(AssertionError, 'complete fixed cross-crate includes'):
                    owned_inputs.mirror(repo, output, source)

    def test_source_mirror_cross_crate_include_hash_drift_rejects(self):
        for changed in owned_inputs.SOURCE_FILES:
            with tempfile.TemporaryDirectory() as directory:
                repo, output, source = self.mirror_include_fixture(directory)
                (repo / changed).write_bytes(b'// replaced source include')
                with self.assertRaises(AssertionError):
                    owned_inputs.mirror(repo, output, source)
