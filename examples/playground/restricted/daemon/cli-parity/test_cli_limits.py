"""CLI-only finite FD budget readback, not runtime qualification."""
import copy
import unittest
import cli_limits
import cli_identity
from cli_config import configuration
from cli_plan import operation
from test_cli_helpers import CID, Engine, approval

GOOD = 'Max core file size 0 0 bytes\nMax file size 268435456 268435456 bytes\nMax open files 1024 1024 files\n'


class LimitTests(unittest.TestCase):
    def test_only_exact_finite_actual_limits_are_accepted(self):
        self.assertEqual(cli_limits.validate(GOOD)['Max open files'], ['1024', '1024', 'files'])
        for changed in [GOOD.replace('1024', x) for x in ['512', '2048', 'unlimited', '0', 'unknown']] + \
                       [GOOD.replace('1024 1024', '512 1024'), GOOD.replace('1024 1024', '1024 2048'),
                        GOOD + 'Max open files 1024 1024 files\n', GOOD.replace('Max open files', 'Unknown limit'),
                        GOOD.replace('268435456', '536870912'), GOOD.replace('0 0 bytes', '1 1 bytes')]:
            with self.assertRaises(ValueError):
                cli_limits.validate(changed)

    def test_daemon_readback_must_equal_exact_cli1024_request_even_during_cleanup(self):
        expected = configuration(approval(), '1' * 32); engine = Engine(operation('build-addition'))
        name = 'zryna410-cli-' + '1' * 32
        engine.json('POST', '/containers/create?name=' + name, expected)
        for cleanup in [False, True]:
            cli_identity.container(engine.state, CID, name, expected, cleanup=cleanup)
            for field in ['Soft', 'Hard']:
                for value in [512, 2048, 'unknown', None]:
                    changed = copy.deepcopy(engine.state)
                    changed['HostConfig']['Ulimits'][-1][field] = value
                    with self.assertRaises(ValueError):
                        cli_identity.container(changed, CID, name, expected, cleanup=cleanup)


if __name__ == '__main__':
    unittest.main()
