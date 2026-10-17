"""Ordinary trusted Actions build preparation, never a native test/containment receipt."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time

SAFE = ('LANG', 'LC_ALL', 'TZ', 'HOME', 'USERPROFILE', 'TMPDIR', 'TEMP', 'TMP',
        'SystemRoot', 'WINDIR', 'ProgramFiles', 'COMSPEC', 'PATHEXT')
RECIPES = (('driver', '-p', 'zryna-driver', '--lib'),
           ('cli', '-p', 'zryna', '--test', 'cli'),
           ('main', '-p', 'zryna', '--bin', 'zryna'),
           ('protocol', '-p', 'zryna-language-server', '--test', 'protocol'),
           ('process', '-p', 'zryna-language-server', '--test', 'process'))


def environment(ambient, runner_temp, windows):
    env = {key: ambient[key] for key in SAFE if key in ambient}
    if windows:
        env.update({key: ambient[key] for key in ('ProgramFiles(x86)', 'ProgramW6432', 'SystemDrive') if key in ambient})
    user = env['USERPROFILE' if windows else 'HOME']
    env['CARGO_HOME'] = str(Path(ambient.get('CARGO_HOME', str(Path(user) / '.cargo'))).resolve(strict=True))
    env['RUSTUP_HOME'] = str(Path(ambient.get('RUSTUP_HOME', str(Path(user) / '.rustup'))).resolve(strict=True))
    host = 'x86_64-pc-windows-msvc' if windows else 'x86_64-unknown-linux-gnu'
    toolchain = Path(env['RUSTUP_HOME']) / 'toolchains' / ('1.97.1-' + host) / 'bin'
    node = Path(shutil.which('node', path=ambient.get('PATH', ambient.get('Path', '')))).resolve(strict=True)
    env['PATH'] = os.pathsep.join((str(node.parent), str(toolchain), ambient.get('PATH', ambient.get('Path', ''))))
    env.update(ZRYNA_NODE=str(node), RUSTUP_TOOLCHAIN='1.97.1',
               CARGO_TARGET_DIR=str(runner_temp / 'native-tooling-warm-target'),
               CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0',
               CARGO_BUILD_JOBS='2', RUST_TEST_THREADS='2')
    return env, toolchain / ('cargo.exe' if windows else 'cargo')


def main():
    assert len(sys.argv) == 1
    for key in ('SELECTED_FINAL_MAIN', 'SELECTED_FINAL_TREE', 'SELECTED_OPERATOR_COMMIT'):
        assert re.fullmatch('[a-f0-9]{40}', os.environ.get(key, '')), 'UNBOUND ordinary build preparation'
    assert os.environ.get('GITHUB_REPOSITORY') == 'zryna/zryna'
    workspace = Path(os.environ['GITHUB_WORKSPACE']).resolve(strict=True)
    temp = Path(os.environ['RUNNER_TEMP']).resolve(strict=True)
    source = workspace / 'source'
    for args, expected in ((['rev-parse', 'HEAD'], os.environ['SELECTED_FINAL_MAIN']),
                           (['rev-parse', 'HEAD^{tree}'], os.environ['SELECTED_FINAL_TREE']),
                           (['status', '--porcelain=v1', '--untracked-files=all'], '')):
        assert subprocess.check_output(['git', *args], cwd=source, timeout=15).decode().strip() == expected
    env, cargo = environment(os.environ, temp, os.name == 'nt')
    target = Path(env['CARGO_TARGET_DIR']); target.mkdir(mode=0o700)
    root = temp / 'native-tooling-warm-records'; root.mkdir(mode=0o700)
    records = []; deadline = time.monotonic() + 900; failure = None
    try:
        for name, *target_args in RECIPES:
            args = [str(cargo), 'test', '--locked', '--offline', *target_args, '--no-run', '--message-format=json']
            start = time.monotonic(); assert start < deadline
            completed = subprocess.run(args, cwd=source, env=env, stdin=subprocess.DEVNULL,
                                       stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                       timeout=deadline - start, check=False)
            streams = []
            for suffix, data in (('stdout', completed.stdout), ('stderr', completed.stderr)):
                with (root / (name + '.' + suffix)).open('xb') as output:
                    output.write(data[:1048576]); output.flush(); os.fsync(output.fileno())
                streams.append({'name': name + '.' + suffix, 'observedBytes': len(data),
                                'retainedBytes': min(len(data), 1048576),
                                'complete': len(data) <= 1048576,
                                'retainedSha256': hashlib.sha256(data[:1048576]).hexdigest()})
            records.append({'argv': args, 'cwd': str(source), 'environment': env,
                            'elapsedSeconds': time.monotonic() - start, 'returncode': completed.returncode,
                            'streams': streams, 'nativeDescendantCleanupClaim': False})
            assert completed.returncode == 0 and all(s['complete'] for s in streams)
    except BaseException as error:
        failure = type(error).__name__ + ': ' + str(error)[:2048]
        raise
    finally:
        with (root / 'RESULT.json').open('x') as output:
            json.dump({'scope': 'Ordinary trusted Actions no-run preparation only; no native test, conformance or descendant containment proof.',
                       'source': os.environ['SELECTED_FINAL_MAIN'], 'tree': os.environ['SELECTED_FINAL_TREE'],
                       'target': str(target), 'records': records, 'failure': failure,
                       'casePassCounts': None, 'testsExecuted': False}, output)
            output.write('\n'); output.flush(); os.fsync(output.fileno())


if __name__ == '__main__':
    main()
