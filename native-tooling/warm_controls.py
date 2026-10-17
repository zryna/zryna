import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('fixed_warm', Path(__file__).with_name('warm-fixed.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class Controls(unittest.TestCase):
    def test_null_refuses_before_git_or_build(self):
        with patch.dict(module.os.environ, {}, clear=True), patch.object(module.subprocess, 'check_output', side_effect=AssertionError('must not execute')):
            with self.assertRaisesRegex(AssertionError, 'UNBOUND'):
                module.main()

    def test_closed_environment_excludes_auth_and_ambient_flags(self):
        with tempfile.TemporaryDirectory(prefix='m6410-warm-control-') as directory:
            root = Path(directory)
            (root / '.cargo').mkdir(); (root / '.rustup').mkdir(); (root / 'node').write_bytes(b'fixture, never executed')
            ambient = {'HOME': str(root), 'USERPROFILE': str(root), 'PATH': str(root),
                       'GH_TOKEN': 'fixture-secret-must-not-forward', 'RUSTFLAGS': 'fixture-no',
                       'ProgramFiles(x86)': 'fixture-sdk', 'ProgramW6432': 'fixture-pf', 'SystemDrive': 'fixture-drive'}
            with patch.object(module.shutil, 'which', return_value=str(root / 'node')):
                for windows in [False, True]:
                    env, cargo = module.environment(ambient, root, windows)
                    self.assertNotIn('GH_TOKEN', env); self.assertNotIn('RUSTFLAGS', env)
                    self.assertEqual(env['CARGO_TARGET_DIR'], str(root / 'native-tooling-warm-target'))
                    self.assertEqual(env['CARGO_BUILD_JOBS'], '2')
                    self.assertEqual(env['RUSTUP_TOOLCHAIN'], '1.97.1')
                    for name in ['ProgramFiles(x86)', 'ProgramW6432', 'SystemDrive']:
                        self.assertEqual(name in env, windows)


if __name__ == '__main__':
    unittest.main()
