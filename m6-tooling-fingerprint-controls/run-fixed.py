"""Prospective ordinary trusted-run wrapper, not hostile containment or native cleanup authority."""
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys
import threading
import time

CAP = 1048576
FINAL = 262144
ALLOWED = frozenset(('PATH', 'Path', 'LANG', 'LC_ALL', 'TZ', 'HOME', 'USERPROFILE', 'TMPDIR',
    'TEMP', 'TMP', 'SystemRoot', 'WINDIR', 'ProgramFiles', 'COMSPEC', 'PATHEXT', 'CARGO_HOME',
    'RUSTUP_HOME', 'GITHUB_WORKSPACE', 'RUNNER_TEMP', 'SELECTED_FINAL_MAIN', 'SELECTED_FINAL_TREE',
    'SELECTED_OPERATOR_COMMIT', 'GITHUB_REPOSITORY', 'GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT',
    'GITHUB_WORKFLOW_REF', 'GITHUB_JOB', 'GITHUB_SHA', 'RUNNER_OS', 'ImageOS', 'ImageVersion'))


WINDOWS = os.name == 'nt'
NATIVE_IDENTITY = None
if WINDOWS:
    import importlib.util
    identity_spec = importlib.util.spec_from_file_location(
        'fixed_collector_windows_identity', Path(__file__).with_name('windows_file_identity.py'))
    assert identity_spec is not None and identity_spec.loader is not None
    NATIVE_IDENTITY = importlib.util.module_from_spec(identity_spec)
    identity_spec.loader.exec_module(NATIVE_IDENTITY)


def fingerprint_windows(path, deadline):
    assert type(deadline) is float and math.isfinite(deadline) and time.monotonic() < deadline
    before = path.lstat()
    assert stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_ino != 0
    with path.open('rb') as source:
        native_before = NATIVE_IDENTITY.capture(source.fileno(), path, before.st_size, deadline)
        digest = hashlib.sha256()
        count = 0
        for chunk in iter(lambda: source.read(65536), b''):
            assert time.monotonic() < deadline, 'Original fingerprint observation deadline.'
            count += len(chunk)
            assert count <= before.st_size, 'First extra fingerprint byte.'
            digest.update(chunk)
        assert count == before.st_size, 'Complete original fingerprint bytes.'
        native_after = NATIVE_IDENTITY.capture(source.fileno(), path, before.st_size, deadline)
        NATIVE_IDENTITY.require_same(native_before, native_after)
    assert time.monotonic() < deadline, 'Original fingerprint observation deadline.'
    return {'path': str(path), 'bytes': count, 'sha256': digest.hexdigest(), 'identity': native_before}


def fingerprint(path, deadline=None):
    if WINDOWS:
        return fingerprint_windows(path, deadline)
    before = path.lstat()
    assert stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_ino != 0
    stable = lambda value: (value.st_dev, value.st_ino, value.st_mode, value.st_nlink,
                            value.st_size, value.st_mtime_ns, value.st_ctime_ns)
    with path.open('rb') as source:
        held = os.fstat(source.fileno())
        assert stable(held) == stable(before)
        digest = hashlib.sha256()
        for chunk in iter(lambda: source.read(65536), b''):
            digest.update(chunk)
        assert stable(os.fstat(source.fileno())) == stable(before) and stable(path.lstat()) == stable(before)
    return {'path': str(path), 'bytes': before.st_size, 'sha256': digest.hexdigest(),
            'identity': {'dev': str(before.st_dev), 'ino': str(before.st_ino), 'mode': str(before.st_mode),
                         'modifiedNs': str(before.st_mtime_ns), 'changedNs': str(before.st_ctime_ns),
                         'bytes': str(before.st_size), 'links': str(before.st_nlink)}}


def metadata(path, value):
    with path.open('xb') as target:
        target.write((json.dumps(value, indent=2) + '\n').encode('utf-8'))
        target.flush()
        os.fsync(target.fileno())


def capture(node, entry, cwd, env, root, name, seconds):
    start = time.monotonic()
    observation_deadline = start + seconds
    before = fingerprint(node, observation_deadline)
    out = [os.open(root / (name + suffix), os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
           for suffix in ('.stdout', '.stderr')]
    state = [{'observed': 0, 'retained': 0, 'eof': False, 'failure': None} for _ in range(2)]
    overflow = threading.Event()
    stop_storage = threading.Event()
    lock = threading.Lock()
    child = None
    unknown = False
    termination = []
    readers = []
    capture_error = None

    def stop_node():
        if child is None or child.poll() is not None:
            return False
        for label, signal in [('terminate-original-Node-child-only', child.terminate),
                              ('kill-original-Node-child-only', child.kill)]:
            termination.append(label)
            try:
                signal()
                child.wait(timeout=2)
                return False
            except subprocess.TimeoutExpired:
                pass
            except OSError:
                return True
        return child.poll() is None

    try:
        assert time.monotonic() - start < seconds, 'Outer admission expired before Node launch.'
        child = subprocess.Popen([str(node), str(entry)], cwd=str(cwd), env=env,
                                 stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                 shell=False)

        def consume(index, stream):
            try:
                while True:
                    data = os.read(stream.fileno(), 65536)
                    if not data:
                        state[index]['eof'] = True
                        break
                    with lock:
                        state[index]['observed'] += len(data)
                        if not stop_storage.is_set():
                            prefix = data[:max(0, CAP - state[index]['retained'])]
                            offset = 0
                            while offset < len(prefix):
                                count = os.write(out[index], prefix[offset:])
                                assert count > 0
                                offset += count
                            state[index]['retained'] += len(prefix)
                        if state[index]['observed'] > CAP:
                            overflow.set()
            except Exception as error:
                state[index]['failure'] = type(error).__name__
                overflow.set()

        for index, stream in enumerate((child.stdout, child.stderr)):
            thread = threading.Thread(target=consume, args=(index, stream), daemon=True)
            thread.start()
            readers.append(thread)
        while child.poll() is None and time.monotonic() - start < seconds and not overflow.is_set():
            time.sleep(0.02)
        if child.poll() is None:
            unknown = stop_node()
    except Exception as error:
        capture_error = type(error).__name__ + ': ' + str(error)[:2048]
        unknown = stop_node()
    finally:
        drain_end = time.monotonic() + 2
        for thread in readers:
            thread.join(max(0, drain_end - time.monotonic()))
        if any(thread.is_alive() for thread in readers):
            unknown = True
        # Freeze retained prefixes before flushing. Unknown pipes never imply descendant cleanup.
        with lock:
            stop_storage.set()
            snapshot = [dict(value) for value in state]
            for fd in out:
                os.fsync(fd)
                os.close(fd)
        if child is not None and child.poll() is not None and not unknown:
            child.stdout.close()
            child.stderr.close()
    after = fingerprint(node, observation_deadline)
    assert after == before, 'Selected Node identity changed.'
    streams = [fingerprint(root / (name + suffix), observation_deadline) for suffix in ('.stdout', '.stderr')]
    complete = not unknown and not overflow.is_set() and all(s['eof'] and s['failure'] is None for s in snapshot)
    accepted = complete and capture_error is None and child is not None and child.returncode == 0 and not termination
    final_fit = all(p['bytes'] <= FINAL for p in streams)
    record = {'program': before, 'argv': [str(entry)], 'cwd': str(cwd), 'streams': streams,
              'streamObservations': snapshot, 'captureFailure': capture_error, 'completeOriginalStreams': complete,
              'returncode': child.returncode if child else None, 'elapsedSeconds': time.monotonic() - start,
              'terminationRequests': termination, 'outerUnknown': unknown,
              'accepted': accepted and final_fit, 'finalCarrierFits': final_fit,
              'meaning': 'Actual Node entry stdout/stderr, not Cargo stdout/stderr.',
              'nativeDescendantCleanupAuthority': 'Original inner owner records only; no outer claim.'}
    metadata(root / (name + '.json'), record)
    return record


def main():
    intent_deadline = time.monotonic() + 615
    assert len(sys.argv) == 1
    assert all(re.fullmatch(r'[a-f0-9]{40}', os.environ.get(key, '')) for key in
               ('SELECTED_FINAL_MAIN', 'SELECTED_FINAL_TREE', 'SELECTED_OPERATOR_COMMIT')), 'UNBOUND'
    env = {key: os.environ[key] for key in ALLOWED if key in os.environ}
    if 'PATH' in env:
        env.pop('Path', None)
    node_found = shutil.which('node', path=env.get('PATH', env.get('Path', '')))
    assert node_found is not None
    node = Path(node_found).resolve(strict=True)
    workspace = Path(env['GITHUB_WORKSPACE']).resolve(strict=True)
    root = Path(env['RUNNER_TEMP']).resolve(strict=True) / 'native-tooling-outer'
    root.mkdir(mode=0o700)
    selected = workspace / 'source'
    prepared = root.parent / 'native-tooling-preparation'
    operator = workspace / 'operator' / 'native-tooling'
    producer = prepared / 'BINDING_PRODUCER_CAPSULE'
    entries = [('COLLECT', operator / 'collect.mjs', 615),
               ('PREPARE', producer / 'prepare.mjs', 615),
               ('PREBUILD', prepared / 'PREPARATION_CAPSULE' / 'prebuild.mjs', 3015),
               ('BIND', producer / 'bind.mjs', 615),
               ('PROJECT', prepared / 'PROJECT_CAPSULE' / 'record-project.mjs', 615),
               ('LSP', prepared / 'LSP_CAPSULE' / 'record-lsp.mjs', 615),
               ('FORMATTER', prepared / 'FORMATTER_CAPSULE' / 'record-formatter.mjs', 615)]
    records = []
    failure = None
    # Create-only result exists before the first child. Final local fsync is not a hard deadline.
    result = (root / 'RESULT.json').open('xb')
    try:
        result.flush()
        os.fsync(result.fileno())
        metadata(root / 'INTENT.json', {'wrapperPython': {'path': sys.executable, 'version': sys.version,
                 'trustedBoundary': 'Ordinary runner image under root-preapproved Actions provenance'}, 'node': fingerprint(node, intent_deadline), 'entries': [(n, str(p), t) for n, p, t in entries],
                 'environment': env, 'scope': 'Trusted ordinary Node capture; no new hostile owner or pass assembly.'})
        for name, entry, seconds in entries:
            record = capture(node, entry, selected, env, root, name, seconds)
            records.append(record)
            if not record['accepted']:
                failure = name + ': stopped; preserve actual original/prefix streams'
                break
    except Exception as error:
        failure = type(error).__name__ + ': ' + str(error)[:2048]
    finally:
        result.write((json.dumps({'records': records, 'failure': failure, 'casePassCounts': None,
                                 'qualification': False}, indent=2) + '\n').encode('utf-8'))
        result.flush()
        os.fsync(result.fileno())
        result.close()
    if failure:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
