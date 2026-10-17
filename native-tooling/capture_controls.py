"""SOURCE ONLY, UNRUN. Mocked child, synthetic local files; never starts Node/native tools."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('fixed_capture', Path(__file__).with_name('run-fixed.py'))
capture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(capture)


class Controls(unittest.TestCase):
    def test_real_unbound_main_refuses_before_filesystem_or_child(self):
        with patch.dict(capture.os.environ, {}, clear=True), patch.object(capture.sys, 'argv', ['recorder']), \
                patch.object(capture.subprocess, 'Popen') as child, patch.object(capture.Path, 'mkdir') as mkdir:
            with self.assertRaisesRegex(AssertionError, 'UNBOUND'):
                capture.main()
            child.assert_not_called()
            mkdir.assert_not_called()

    def test_exact_extra_and_original_cap_prefix_are_not_truncated_successes(self):
        for length, accepted, complete in [(capture.FINAL, True, True),
                                            (capture.FINAL + 1, False, True),
                                            (capture.CAP + 1, False, False)]:
            with self.subTest(length=length), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                node = root / 'synthetic-node-bytes'
                node.write_bytes(b'inert-fixture-never-executed')

                class FakeProcess:
                    def __init__(self, *args, **kwargs):
                        self.stdout = tempfile.TemporaryFile()
                        self.stdout.write(b'X' * length)
                        self.stdout.seek(0)
                        self.stderr = tempfile.TemporaryFile()
                        self.returncode = 0
                    def poll(self):
                        return self.returncode
                    def terminate(self):
                        raise AssertionError('Already-exited fake child must not be terminated')
                    kill = terminate

                with patch.object(capture.subprocess, 'Popen', FakeProcess):
                    actual = capture.capture(node, root / 'entry.mjs', root, {}, root, 'CASE', 10)
                self.assertEqual(actual['accepted'], accepted)
                self.assertEqual(actual['completeOriginalStreams'], complete)
                self.assertEqual(actual['streams'][0]['bytes'], min(length, capture.CAP))
                self.assertEqual(actual['streamObservations'][0]['observed'], length)
                self.assertEqual((root / 'CASE.stdout').read_bytes(), b'X' * min(length, capture.CAP))


if __name__ == '__main__':
    unittest.main()
