"""Exact actual CLI-only rlimits; the original full installation needs >512 retained FDs."""
from cli_common import require


def validate(text):
    require(type(text) is str and len(text) <= 16384, 'bounded actual CLI limits')
    expected = {'Max core file size': ['0', '0', 'bytes'],
                'Max file size': ['268435456', '268435456', 'bytes'],
                'Max open files': ['1024', '1024', 'files']}
    actual = {}
    for name, values in expected.items():
        rows = [line[len(name):].split() for line in text.splitlines() if line.startswith(name)]
        require(rows == [values], 'exact finite actual CLI limit: ' + name)
        actual[name] = values
    return actual
