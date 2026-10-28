"""Linux-only read-only observer. No selectors, credentials, children or approval inputs."""
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import stat
import sys
import time

class _SourceOnlyLoader(importlib.machinery.SourceFileLoader):
    def get_code(self, fullname):
        # -B prevents cache writes, but ordinary SourceFileLoader may still read pyc.
        # Always compile these fixed source bytes; never consult adjacent bytecode.
        return self.source_to_code(self.get_data(self.path), self.path)


_SOURCE = str(Path(__file__).with_name('native_host_facts.py'))
_SPEC = importlib.util.spec_from_file_location('native_host_facts', _SOURCE,
                                            loader=_SourceOnlyLoader('native_host_facts', _SOURCE))
_CORE = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_CORE)


class _KernelIO:
    def __init__(self):
        self.pid = os.getpid()
        self.deadline = time.monotonic() + _CORE.LIMITS['seconds']
        self._files = {f'/proc/{pid}/{name}' for pid in (self.pid, 1) for name in _CORE.FILES}
        self._links = {f'/proc/{pid}/ns/{name}' for pid in (self.pid, 1) for name in _CORE.NAMESPACES}

    def monotonic(self):
        return time.monotonic()

    def environment(self, key):
        if key not in _CORE.PROJECTIONS:
            raise ValueError('not an environment projection selector')
        return os.environ.get(key)

    def _guard(self):
        if time.monotonic() >= self.deadline:
            raise TimeoutError('read-only collector deadline')

    def readlink(self, path):
        if path not in self._links:
            raise ValueError('not a fixed namespace selector')
        self._guard()
        return os.readlink(path)

    def read(self, path, maximum):
        if path not in self._files:
            prefix = '/sys/fs/cgroup'
            group, separator, leaf = path.rpartition('/')
            if not separator or leaf not in _CORE.CONTROLS or not (group == prefix or group.startswith(prefix + '/')):
                raise ValueError('not a fixed proc/control selector')
            _CORE._canonical(group[len(prefix):] or '/')
            if maximum != 4096:
                raise ValueError('not the fixed control read bound')
        elif maximum != _CORE.FILES[path.rsplit('/', 1)[1]]:
            raise ValueError('not the fixed proc read bound')
        self._guard()
        descriptors = []
        try:
            descriptor = os.open('/', os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC)
            descriptors.append(descriptor)
            parts = path.split('/')[1:]
            for part in parts[:-1]:
                self._guard()
                descriptor = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                                     dir_fd=descriptor)
                descriptors.append(descriptor)
            descriptor = os.open(parts[-1], os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC,
                                 dir_fd=descriptor)
            descriptors.append(descriptor)
            if not stat.S_ISREG(os.fstat(descriptor).st_mode):
                raise ValueError('fixed kernel selector is not a regular pseudo-file')
            data = bytearray()
            while len(data) <= maximum:
                self._guard()
                block = os.read(descriptor, min(65536, maximum + 1 - len(data)))
                if not block:
                    break
                data.extend(block)
            return bytes(data)
        finally:
            for descriptor in reversed(descriptors):
                os.close(descriptor)


def main():
    if sys.platform != 'linux' or len(sys.argv) != 1:
        raise SystemExit('Linux observer accepts no arguments or file selectors')
    result = _CORE.collect_native_host(_KernelIO())
    result['producer'] = {'platform': sys.platform, 'python': sys.version.split()[0],
                          'effectiveUid': os.geteuid(), 'effectiveGid': os.getegid(),
                          'scope': 'local observations; executable/source origin not authenticated here'}
    data = (json.dumps(result, sort_keys=True, separators=(',', ':')) + '\n').encode('utf-8')
    if len(data) > 6 * 1024 * 1024:
        raise SystemExit('bounded report encoding exceeded')
    sys.stdout.buffer.write(data)


if __name__ == '__main__':
    main()
