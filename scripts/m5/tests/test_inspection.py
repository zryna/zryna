from pathlib import Path
import struct
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from inspection import observe


def symbol(name, index=0, address=0, size=0, info=0):
    return struct.pack('<IBBHQQ', name, info, 0, index, address, size)


def fixture():
    base = 0x401000
    text = b'\xe8\x05\x00\x00\x00\xc3' + b'\x90' * 4 + b'\xc3' + b'\x90' * 5
    strings = b'\0__wrap_sum_bytes\0sum_bytes\0'
    names = b'\0__asan_report_load1\0'
    table = symbol(0) + symbol(1, 1, base, 6, 2) + symbol(18, 1, base + 10, 1, 2)
    dynamic = symbol(0) + symbol(1)
    pieces = [text, strings, table, names, dynamic]
    offset, sections = 64, [(0,) * 10]
    for index, data in enumerate(pieces):
        kind = [1, 3, 2, 3, 11][index]
        flags, address = (6, base) if index == 0 else (0, 0)
        link = 2 if kind == 2 else 4 if kind == 11 else 0
        sections.append((0, kind, flags, address, offset, len(data), link, 0, 1,
                         24 if kind in (2, 11) else 0))
        offset += len(data)
    header = struct.pack('<16sHHIQQQIHHHHHH', b'\x7fELF\x02\x01\x01' + b'\0' * 9,
                         2, 62, 1, base, 0, offset, 0, 64, 0, 0, 64, len(sections), 0)
    elf = header + b''.join(pieces) + b''.join(struct.pack('<IIQQQQIIQQ', *s) for s in sections)
    assembly = b'0000000000401000 <__wrap_sum_bytes>:\n 401000: call 40100a <sum_bytes>\n 401005: ret\n'
    return elf, assembly


class InspectionTests(unittest.TestCase):
    def test_full_observed_arrays_and_direct_forward_bytes(self):
        elf, asm = fixture()
        value = observe(elf, asm, ['__asan_report_load1'])
        self.assertEqual(value['undefinedDynamicSymbols'], ['__asan_report_load1'])
        self.assertEqual(value['orderedMnemonics'], ['call', 'ret'])
        self.assertEqual(value['calls'][0]['directCallBytes'], 'e805000000')
        self.assertFalse(value['reviewedBindings'])
        self.assertEqual(value['targetExecution'], 'UNRUN')

    def test_missing_extra_or_replaced_dynamic_symbol_rejects(self):
        elf, asm = fixture()
        for expected in [[], ['__asan_report_load1', 'unobserved'], ['substituted']]:
            with self.subTest(expected=expected), self.assertRaises(AssertionError):
                observe(elf, asm, expected)

    def test_detached_call_redirection_is_never_run_and_rejects_inspection(self):
        elf, asm = fixture()
        changed = elf[:65] + b'\x04\x00\x00\x00' + elf[69:]
        with self.assertRaises(AssertionError):
            observe(changed, asm, ['__asan_report_load1'])

    def test_foreign_or_duplicate_instruction_sites_reject(self):
        elf, asm = fixture()
        for extra in [b' 401100: ret\n', b' 401005: ret\n', b' 401005:\n']:
            with self.assertRaises(AssertionError):
                observe(elf, asm + extra, ['__asan_report_load1'])

    def test_truncated_or_non_x86_64_elf_rejects(self):
        elf, asm = fixture()
        bad = bytearray(elf)
        bad[18:20] = b'\x03\x00'
        for value in [elf[:64], bytes(bad)]:
            with self.assertRaises(AssertionError):
                observe(value, asm, ['__asan_report_load1'])


if __name__ == '__main__':
    unittest.main()
