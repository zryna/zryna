import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from link_trace import select_trace


class LinkTraceTests(unittest.TestCase):
    def setUp(self):
        self.local = ['/proc/17/fd/3/runtime.o', '/proc/17/fd/4/program.o',
                      '/proc/17/fd/4/runtime.o', '/proc/17/fd/5/runtime.o']
        args = ['-Wl,-t,-t', '-fsanitize=address,undefined'] + self.local
        self.record = {'name': 'link', 'success': True, 'diagnostic': None,
                       'arguments_bytes': [list(x.encode()) for x in args]}
        self.providers = ['/usr/lib/gcc/x86_64-linux-gnu/14/crtbegin.o', '/lib/x86_64-linux-gnu/libc.so.6']
        self.lines = self.local + self.providers + [self.providers[0]]

    def parse(self, lines):
        return select_trace(('\n'.join(lines) + '\n').encode(), self.record)

    def test_complete_source_inputs_and_all_external_occurrences(self):
        report = self.parse(self.lines)
        self.assertEqual(report['paths'], self.providers)
        self.assertEqual(report['occurrences'], self.providers + [self.providers[0]])
        self.assertEqual(report['localInputs'], self.local)

    def test_foreign_or_extra_local_product_never_becomes_provider(self):
        for path in ['/proc/18/fd/4/program.o', '/proc/17/fd/4/invocation.elf',
                     '/tmp/injected.o', '/workspace/source/private.o', '/dev/fd/4']:
            with self.subTest(path=path), self.assertRaises(AssertionError):
                self.parse(self.lines + [path])

    def test_missing_and_duplicate_local_inputs_reject(self):
        for lines in [self.lines[1:], self.lines + [self.local[0]]]:
            with self.assertRaises(AssertionError):
                self.parse(lines)

    def test_unknown_syntax_and_case_alias_reject(self):
        for token in ['-lgcc', '(libgcc.a)member.o', '/usr/lib/a b.so',
                      '/usr/lib/file.so\x00hidden', '/lib/x86_64-linux-gnu/LIBC.so.6']:
            with self.subTest(token=token), self.assertRaises(AssertionError):
                self.parse(self.lines + [token])

    def test_65_unique_external_paths_reject_without_truncation(self):
        with self.assertRaises(AssertionError):
            self.parse(self.local + ['/usr/lib/provider-' + str(i) + '.o' for i in range(65)])

    def test_unsuccessful_or_untraced_link_reject(self):
        self.record['success'] = False
        with self.assertRaises(AssertionError):
            self.parse(self.lines)
        self.record['success'] = True
        self.record['arguments_bytes'] = self.record['arguments_bytes'][1:]
        with self.assertRaises(AssertionError):
            self.parse(self.lines)


if __name__ == '__main__':
    unittest.main()
