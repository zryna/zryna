"""Bounded observation files; recorded data never supplies execution authority."""
from pathlib import Path
import hashlib
import json
import os
import stat

SELECTED_LIMIT = 96 * 1024 * 1024
REPORT_LIMIT = 1024 * 1024
ENTRY_LIMIT = 10000


def stamp(value):
    return [value.st_dev, value.st_ino, value.st_mode, value.st_uid,
            value.st_gid, value.st_size, value.st_mtime_ns, value.st_ctime_ns]


def directory_identity(value):
    return stamp(value)[:5]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def no_duplicates(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError("duplicate JSON key")
        value[key] = item
    return value


def parse_json(data):
    return json.loads(data.decode('utf-8'), object_pairs_hook=no_duplicates)


class Directory:
    def __init__(self, path):
        self.path = Path(path)
        assert str(self.path) == str(path)
        assert self.path.is_absolute() and '..' not in self.path.parts
        self.chain = []
        fd = os.open('/', os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        self.chain.append((fd, directory_identity(os.fstat(fd)), None))
        try:
            for part in self.path.parts[1:]:
                assert part not in ('', '.', '..')
                fd = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                             dir_fd=self.chain[-1][0])
                self.chain.append((fd, directory_identity(os.fstat(fd)), part))
            self.check()
        except BaseException:
            self.close()
            raise

    @property
    def fd(self):
        return self.chain[-1][0]

    def check(self):
        for index, (fd, expected, name) in enumerate(self.chain):
            assert directory_identity(os.fstat(fd)) == expected
            if index:
                named = os.stat(name, dir_fd=self.chain[index - 1][0],
                                follow_symlinks=False)
                assert directory_identity(named) == expected

    def close(self):
        for fd, _, _ in reversed(self.chain):
            os.close(fd)
        self.chain = []

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()

    def read(self, name, maximum):
        assert name not in ('', '.', '..') and '/' not in name
        self.check()
        before = os.stat(name, dir_fd=self.fd, follow_symlinks=False)
        assert stat.S_ISREG(before.st_mode) and before.st_size <= maximum
        fd = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK,
                     dir_fd=self.fd)
        try:
            assert stamp(os.fstat(fd)) == stamp(before)
            chunks, length = [], 0
            while data := os.read(fd, min(1048576, maximum - length + 1)):
                length += len(data)
                assert length <= maximum
                chunks.append(data)
            assert length == before.st_size
            assert stamp(os.fstat(fd)) == stamp(before)
            assert stamp(os.stat(name, dir_fd=self.fd, follow_symlinks=False)) == stamp(before)
            self.check()
            return b''.join(chunks)
        finally:
            os.close(fd)

    def write(self, name, data):
        assert name not in ('', '.', '..') and '/' not in name
        self.check()
        fd = os.open(name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
                     0o600, dir_fd=self.fd)
        try:
            view = memoryview(data)
            while view:
                view = view[os.write(fd, view):]
            os.fsync(fd)
            named = os.stat(name, dir_fd=self.fd, follow_symlinks=False)
            assert stamp(os.fstat(fd)) == stamp(named) and named.st_size == len(data)
            self.check()
        finally:
            os.close(fd)

    def finalize_reserved(self, name, data):
        """Set completion only in an already allocated bounded status file."""
        assert name == 'CAPTURE_RESULT.json'
        self.check()
        before = os.stat(name, dir_fd=self.fd, follow_symlinks=False)
        assert stat.S_ISREG(before.st_mode) and len(data) <= before.st_size
        fd = os.open(name, os.O_WRONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=self.fd)
        try:
            assert stamp(os.fstat(fd)) == stamp(before)
            view = memoryview(data)
            while view:
                view = view[os.write(fd, view):]
            os.ftruncate(fd, len(data))
            os.fsync(fd)
            named = os.stat(name, dir_fd=self.fd, follow_symlinks=False)
            assert stamp(os.fstat(fd)) == stamp(named) and named.st_size == len(data)
            self.check()
        finally:
            os.close(fd)


def read(path, maximum):
    path = Path(path)
    with Directory(path.parent) as parent:
        return parent.read(path.name, maximum)


def write_json(parent, name, value, *, compact=False):
    # Complete source censuses need no formatting whitespace; all report bounds still apply.
    if compact:
        data = (json.dumps(value, separators=(',', ':'), ensure_ascii=False) + '\n').encode()
    else:
        data = (json.dumps(value, indent=2, ensure_ascii=False) + '\n').encode()
    assert len(data) <= REPORT_LIMIT
    parent.write(name, data)
    return {'path': name, 'bytes': len(data), 'sha256': sha(data)}


def usage(root):
    """Visible active namespace observations, not a hard/open-unlinked quota."""
    seen, entries, allocated, length = set(), 0, 0, 0
    with Directory(root) as held:
        for parent, dirs, names in os.walk(root, followlinks=False):
            for name in dirs + names:
                try:
                    value = (Path(parent) / name).lstat()
                except FileNotFoundError:
                    continue
                assert not stat.S_ISLNK(value.st_mode)
                assert value.st_dev == os.fstat(held.fd).st_dev
                assert stat.S_ISREG(value.st_mode) or stat.S_ISDIR(value.st_mode)
                entries += 1
                key = (value.st_dev, value.st_ino)
                if key not in seen:
                    seen.add(key)
                    allocated += value.st_blocks * 512
                    if stat.S_ISREG(value.st_mode):
                        length += value.st_size
                assert entries <= ENTRY_LIMIT and allocated <= SELECTED_LIMIT
                assert length <= SELECTED_LIMIT
        held.check()
    return {'entries': entries, 'allocatedBytes': allocated, 'retainedBytes': length}


def namespace(root):
    files, seen, allocated, length, entries = [], set(), 0, 0, 0
    with Directory(root) as held:
        for parent, dirs, names in os.walk(root, followlinks=False):
            for name in dirs + names:
                path = Path(parent) / name
                value = path.lstat()
                assert not stat.S_ISLNK(value.st_mode)
                assert value.st_dev == os.fstat(held.fd).st_dev
                assert stat.S_ISREG(value.st_mode) or stat.S_ISDIR(value.st_mode)
                entries += 1
                key = (value.st_dev, value.st_ino)
                if key not in seen:
                    seen.add(key)
                    allocated += value.st_blocks * 512
                    if stat.S_ISREG(value.st_mode):
                        length += value.st_size
                assert entries <= ENTRY_LIMIT and allocated <= SELECTED_LIMIT
                assert length <= SELECTED_LIMIT
                if stat.S_ISREG(value.st_mode):
                    data = read(path, SELECTED_LIMIT)
                    files.append({'path': str(path.relative_to(root)),
                                  'bytes': len(data), 'sha256': sha(data)})
        held.check()
    return {'entries': entries, 'allocatedBytes': allocated,
            'retainedBytes': length, 'files': sorted(files, key=lambda x: x['path'])}
