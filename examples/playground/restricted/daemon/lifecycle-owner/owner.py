"""Private source-only loader and lifecycle owner entry; never a source-request API."""
import builtins
import hashlib
import os
from pathlib import Path
import types

_ROOT = Path(__file__).resolve().parent
_OLD = _ROOT.parent / 'cli-parity'
_PINS = {'cli_common': 'c8cfd73e1fde0f97fbf0d65422a42db79df835128511d0e4eca3207854822b2d', 'cli_engine': 'b4c7ee834f193ed4d1f61b89ca39f73704aad5216b3e6b1e03da8aabbdc5bd67', 'cli_cgroup': 'd6cb56146d9aa795b711ea5ddf57664141f67ea90e1c84895660a7538ed25ad6', 'cli_kernel_read': 'a46fda5c4a99adadf6a29108b4bb42f34b56b3c6ff60a1ee0d2c7b5d753a03e7', 'cli_kernel_net': '1bb84fdf95c8d39a2b88d1bf76a72eab89982d6160a75751af4b1370ea470d8f', 'cli_mounts': '7744fe6e6332a400c3337aa3f8782292ba10b1ff87613dbca55432f171a82990', 'cli_proc': '7b96e8b54edfd1746ae2db1451dbbf20db194a0e9e03abb43992ccc1468a004d'}
_PINS['cli_exec'] = '7b2873f39b021ae9176794432b7b679c01d497b1b2fb9de359dac27ec042aa3a'
_PINS['cli_plan'] = '33857536b36fb1798cf6ce31b67ef7b584ce5b9c7340d6ce20116b3300f1c623'
_PREPARATION = {'plan': 'cf3c9c080303710e8613c85518864341269ea52c4673b848aa7cf3d7bc23fcfc', 'handles': '014c29ca354c5aacf5aeae27a755f780bd8f14abe82c98576cde0465dfd6760f', 'archive': '95515139e7634237387921a167f4b228260c2019b988b30eb65c265e6e23e172', 'engine': 'f8172f41398656d513366ad6b41f2b3dadd83cc1360d751a9d84b7ccef8e693b', 'install': '9580608303a3a21c437a88765af48accd4f80cec7ee55bf8c921ca2d26c54cac', 'emit': '872a93b32b723c11f55321dc91ea05d5490ebe59130656897d29bf06a9409443', 'material_seed': '1a5a12ca59d78e95447e8b1bcfb676f282a7f11b727abcff34a904860c670a90'}
_NEW = {'lifecycle_inputs', 'lifecycle_engine', 'lifecycle_cgroup', 'lifecycle_kernel',
        'lifecycle_execute', 'lifecycle_backend', 'lifecycle_operation',
        'preparation', 'preparation_plan', 'preparation_primitives', 'preparation_volume'}
_CACHE = {}
_IMPORT = builtins.__import__


def _load(name):
    if name in _CACHE:
        return _CACHE[name]
    if name not in _PINS and name not in _PREPARATION and name not in _NEW:
        raise ValueError('not a fixed lifecycle primitive')
    directory = _OLD if name in _PINS else _ROOT.parent / 'preparation' if name in _PREPARATION else _ROOT
    path = directory / (name + '.py')
    descriptor = os.open(path, os.O_RDONLY | getattr(os, 'O_NOFOLLOW', 0) | getattr(os, 'O_NONBLOCK', 0) | getattr(os, 'O_CLOEXEC', 0))
    try:
        with os.fdopen(descriptor, 'rb') as stream:
            source = stream.read(131073)
    except BaseException:
        raise
    pin = _PINS.get(name, _PREPARATION.get(name))
    if len(source) > 131072 or pin is not None and hashlib.sha256(source).hexdigest() != pin:
        raise ValueError('fixed primitive source bytes differ')
    module = types.ModuleType(name)
    module.__file__ = str(path)
    environment = dict(vars(builtins))
    def imports(selected, globals=None, locals=None, fromlist=(), level=0):
        if level == 0 and selected == 'owner':
            return types.SimpleNamespace(_load=_load)
        if level == 0 and (selected in _PINS or selected in _PREPARATION or selected in _NEW):
            return _load(selected)
        return _IMPORT(selected, globals, locals, fromlist, level)
    environment['__import__'] = imports
    module.__dict__['__builtins__'] = environment
    _CACHE[name] = module
    try:
        exec(compile(source, str(path), 'exec', dont_inherit=True), module.__dict__)
    except BaseException:
        del _CACHE[name]
        raise
    return module


def run_pair(selected):
    """Execute only the two original lifecycle modes under creation-owned parents."""
    return _load('lifecycle_operation').run_pair(selected)
