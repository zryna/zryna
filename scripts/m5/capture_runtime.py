"""Finite, shell-free compiler/version observations and captured-group cleanup."""
from pathlib import Path
import os
import selectors
import signal
import subprocess
import time
from capture_io import Directory, usage, write_json


def counters(path):
    text = Path(path).read_text()
    assert len(text.encode()) <= 65536
    return {key: int(value) for key, value in (line.split() for line in text.splitlines())}


def memory():
    values = {}
    for line in Path('/proc/meminfo').read_text().splitlines():
        key, value = line.split(':', 1)
        fields = value.split()
        if fields and fields[0].isdecimal():
            values[key] = int(fields[0]) * 1024
    assert values['MemAvailable'] >= 512 * 1024 * 1024
    group = Path('/sys/fs/cgroup')
    events = counters(group / 'memory.events') if (group / 'memory.events').is_file() else None
    result = {'hostMemAvailable': values['MemAvailable'], 'cgroupOOMEvents': events,
              'kernelOOMKill': counters('/proc/vmstat')['oom_kill'],
              'scope': 'Shared host/cgroup observations only; no capacity reservation/private-host grant'}
    if (group / 'memory.max').is_file():
        maximum = (group / 'memory.max').read_text().strip()
        if maximum != 'max':
            maximum = int(maximum)
            current = int((group / 'memory.current').read_text())
            stats = counters(group / 'memory.stat')
            credit = min(current, stats['inactive_file'] + stats['slab_reclaimable'])
            result.update({'maximum': maximum, 'current': current, 'inactiveSlabCredit': credit,
                           'estimatedRemaining': maximum - current + credit})
            assert result['estimatedRemaining'] >= 512 * 1024 * 1024
    return result


class Resources:
    def __init__(self, output):
        self.output = Path(output)
        # Ten seconds are reserved for controlled captured-group cleanup.
        self.deadline = time.monotonic() + 290
        self.baseline = memory()

    def check(self):
        assert time.monotonic() < self.deadline, '300-second capture deadline'
        value = memory()
        assert value['kernelOOMKill'] == self.baseline['kernelOOMKill']
        before, now = self.baseline['cgroupOOMEvents'], value['cgroupOOMEvents']
        assert (before is None) == (now is None)
        if before is not None:
            assert all(now[k] == before[k] for k in ('oom', 'oom_kill', 'oom_group_kill'))
        volume = os.statvfs(self.output)
        assert volume.f_bavail * volume.f_frsize >= 512 * 1024 * 1024
        value['captureNamespace'] = usage(self.output)
        return value


def process_identity(pid):
    raw = Path('/proc/' + str(pid) + '/stat').read_text()
    fields = raw[raw.rfind(')') + 2:].split()
    return {'pid': pid, 'pgrp': int(fields[2]), 'session': int(fields[3]),
            'startTicks': int(fields[19])}


def group_absent(group):
    try:
        os.killpg(group, 0)
        return False
    except ProcessLookupError:
        return True


def run(argv, output, label, resources, env=None, timeout=300, maximum=8 * 1024 * 1024):
    assert argv and all(isinstance(x, str) for x in argv)
    resources.check()
    started = time.monotonic()
    deadline = min(resources.deadline, started + timeout)
    child = subprocess.Popen(argv, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                             env=env, start_new_session=True, shell=False)
    owned = process_identity(child.pid)
    assert owned['pgrp'] == owned['session'] == child.pid
    selector = selectors.DefaultSelector()
    chunks, lengths, signals = [[], []], [0, 0], []
    complete = False
    for index, stream in enumerate((child.stdout, child.stderr)):
        os.set_blocking(stream.fileno(), False)
        selector.register(stream, selectors.EVENT_READ, index)
    try:
        while selector.get_map():
            assert time.monotonic() < deadline, 'bounded subprocess deadline'
            resources.check()
            for key, _ in selector.select(0.25):
                data = os.read(key.fileobj.fileno(), 65536)
                if not data:
                    selector.unregister(key.fileobj)
                    continue
                lengths[key.data] += len(data)
                assert lengths[key.data] <= maximum
                chunks[key.data].append(data)
        code = child.wait(timeout=max(0.01, deadline - time.monotonic()))
        complete = True
    finally:
        selector.close()
        if child.poll() is None:
            assert process_identity(child.pid) == owned, 'unclear captured process ownership'
            os.killpg(child.pid, signal.SIGTERM)
            signals.append('SIGTERM')
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                assert process_identity(child.pid) == owned
                os.killpg(child.pid, signal.SIGKILL)
                signals.append('SIGKILL')
                child.wait(timeout=5)
        child.stdout.close()
        child.stderr.close()
        with Directory(output) as held:
            for index, suffix in enumerate(('stdout', 'stderr')):
                held.write(label + '.' + suffix + '.log', b''.join(chunks[index]))
            write_json(held, label + '.process.json', {'argv': argv,
                'elapsedSeconds': time.monotonic() - started, 'exitCode': child.returncode,
                'owned': owned, 'leaderReaped': child.poll() is not None,
                'ownedGroupAbsent': group_absent(child.pid), 'signals': signals,
                'completeStreams': complete, 'scope': 'Captured cargo/tool group only; no universal teardown'})
    assert complete and code == 0 and signals == [] and group_absent(child.pid)
    resources.check()
    return b''.join(chunks[0]), b''.join(chunks[1])
