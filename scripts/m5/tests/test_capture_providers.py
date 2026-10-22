import sys
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import capture_providers as providers
from capture_io import Directory, namespace, sha


class ProviderCaptureTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.allowed = self.root / 'allowed'
        self.allowed.mkdir()
        self.output = self.root / 'capture'
        self.output.mkdir()
        self.file = self.allowed / 'provider.o'
        self.file.write_bytes(b'\x7fELF' + b'opaque test data')
        self.roots = (str(self.allowed),)

    def test_complete_bytes_and_alias_epochs_retained(self):
        alias = self.allowed / 'alias.o'
        alias.symlink_to(self.file.name)
        result = providers.capture([str(self.file), str(alias)], self.output, roots=self.roots)
        self.assertEqual(result['uniqueMaterialCount'], 1)
        material = result['retainedMaterials'][0]
        self.assertEqual((self.output / material['retainedFile']).read_bytes(), self.file.read_bytes())
        self.assertEqual(material['sha256'], sha(self.file.read_bytes()))
        self.assertEqual(result['providerPolicy'], 'UNBOUND_MANUAL_REVIEW')

    def test_escape_terminal_is_rejected_before_file_open(self):
        outside = self.root / 'outside.o'
        outside.write_bytes(b'\x7fELFsecret')
        alias = self.allowed / 'escape.o'
        alias.symlink_to(outside)
        with self.assertRaises(AssertionError):
            providers.capture([str(alias)], self.output, roots=self.roots)
        self.assertFalse((self.output / 'MATERIALS.json').exists())

    def test_regular_replacement_after_retention_rejects(self):
        original = providers.copy_hash
        calls = []
        def replace_after_copy(fd, maximum, dest=None):
            result = original(fd, maximum, dest)
            calls.append(fd)
            if len(calls) == 1:
                replacement = self.allowed / 'replacement.o'
                replacement.write_bytes(self.file.read_bytes())
                replacement.replace(self.file)
            return result
        with patch.object(providers, 'copy_hash', replace_after_copy), self.assertRaises(AssertionError):
            providers.capture([str(self.file)], self.output, roots=self.roots)
        self.assertFalse((self.output / 'MATERIALS.json').exists())

    def test_symlink_replacement_after_retention_rejects(self):
        alias = self.allowed / 'alias.o'
        alias.symlink_to(self.file.name)
        original = providers.copy_hash
        calls = []
        def replace_after_copy(fd, maximum, dest=None):
            result = original(fd, maximum, dest)
            calls.append(fd)
            if len(calls) == 1:
                replacement = self.allowed / 'other-provider.o'
                replacement.write_bytes(self.file.read_bytes())
                alias.unlink()
                alias.symlink_to(replacement.name)
            return result
        with patch.object(providers, 'copy_hash', replace_after_copy), self.assertRaises(AssertionError):
            providers.capture([str(alias)], self.output, roots=self.roots)

    def test_missing_duplicate_and_65_selected_materials_reject(self):
        for paths in [[str(self.allowed / 'missing.o')], [str(self.file)] * 2,
                      [str(self.allowed / ('file-' + str(i))) for i in range(65)]]:
            with self.subTest(paths=paths), self.assertRaises((AssertionError, FileNotFoundError)):
                providers.capture(paths, self.output, roots=self.roots)

    def test_unknown_kind_and_nonregular_material_reject(self):
        self.file.write_bytes(b'arbitrary data, not ELF/archive/linker script')
        with self.assertRaises(AssertionError):
            providers.capture([str(self.file)], self.output, roots=self.roots)
        with self.assertRaises(AssertionError):
            providers.walk(str(self.allowed), self.roots)

    def test_create_only_and_linked_output_reject(self):
        with Directory(self.output) as held:
            held.write('foreign', b'preserve')
        with self.assertRaises(AssertionError):
            providers.capture([str(self.file)], self.output, roots=self.roots)
        self.assertEqual((self.output / 'foreign').read_bytes(), b'preserve')
        link = self.root / 'linked-output'
        link.symlink_to(self.output, target_is_directory=True)
        with self.assertRaises(OSError):
            Directory(link)

    def test_namespace_rejects_unselected_extra_and_missing_copy(self):
        result = providers.capture([str(self.file)], self.output, roots=self.roots)
        expected = {x['retainedFile'] for x in result['retainedMaterials']} | {'MATERIALS.json'}
        self.assertEqual({x['path'] for x in namespace(self.output)['files']}, expected)
        extra = self.output / 'extra.bytes'
        extra.write_bytes(b'extra')
        with self.assertRaises(AssertionError):
            providers.verify_inventory(result, self.output)
        extra.unlink()
        (self.output / result['retainedMaterials'][0]['retainedFile']).unlink()
        with self.assertRaises(AssertionError):
            providers.verify_inventory(result, self.output)


if __name__ == '__main__':
    unittest.main()
