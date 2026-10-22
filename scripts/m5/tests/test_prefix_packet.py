import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from capture_io import parse_json, sha
from prefix_packet import receipt, records


class OriginatingReceiptTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.case = self.root / 'case'
        self.case.mkdir()
        self.product = self.case / 'program.o'
        self.product.write_bytes(b'opaque originating product')
        data = self.product.read_bytes()
        value = {'kind': 'ordinary-trusted-distinct-rust-fixture-observation',
            'rust_targets_checked': False, 'production_admission': False,
            'private_recipe_qualified': False, 'complete_input_use': False,
            'complete_provider_inventory': False,
            'artifacts': [{'path': 'program.o', 'bytes': len(data), 'sha256': sha(data)}]}
        self.raw = (json.dumps(value, indent=2) + '\n').encode()
        (self.case / 'receipt.json').write_bytes(self.raw)

    def read(self):
        return receipt(str(self.case), self.root, len(self.raw), sha(self.raw))

    def test_complete_receipt_hash_and_product_bytes(self):
        _, products = self.read()
        self.assertEqual(products['program.o'], self.product.read_bytes())

    def test_missing_extra_replaced_and_linked_products_reject(self):
        extra = self.case / 'unregistered.bytes'
        extra.write_bytes(b'foreign')
        with self.assertRaises(AssertionError):
            self.read()
        extra.unlink()
        self.product.write_bytes(b'changed product')
        with self.assertRaises(AssertionError):
            self.read()
        self.product.unlink()
        with self.assertRaises(FileNotFoundError):
            self.read()
        self.product.symlink_to(self.root / 'outside')
        with self.assertRaises((AssertionError, OSError)):
            self.read()

    def test_forged_receipt_hash_and_external_directory_reject(self):
        with self.assertRaises(AssertionError):
            receipt(str(self.case), self.root, len(self.raw), '0' * 64)
        with self.assertRaises(AssertionError):
            receipt(str(self.case), self.root / 'other-owner', len(self.raw), sha(self.raw))

    def test_duplicate_json_fields_never_supply_authority(self):
        with self.assertRaises(ValueError):
            parse_json(b'{"policy":"None","policy":"admitted"}')

    def test_target_run_record_or_unsuccessful_compiler_rejects(self):
        value = {'name': 'compile-client', 'success': True, 'diagnostic': None,
                 'error_raw_fields_available': True}
        records([value], ['compile-client'])
        for field, bad in [('name', 'run'), ('success', False),
                           ('diagnostic', 'ZRYNA-C4102'), ('error_raw_fields_available', False)]:
            with self.subTest(field=field), self.assertRaises(AssertionError):
                records([{**value, field: bad}], ['compile-client'])


if __name__ == '__main__':
    unittest.main()
