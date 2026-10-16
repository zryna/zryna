"""Retained no-follow ancestors with named reachability checks around every source use."""
import hashlib
import os
import stat
from cli_image_plan import path
from cli_common import require

FIELDS = ['st_dev', 'st_ino', 'st_size', 'st_mode', 'st_uid', 'st_gid', 'st_nlink',
          'st_mtime_ns', 'st_ctime_ns']

def state(value):
    return tuple(getattr(value, field) for field in FIELDS)

def directory_state(value):
    return value.st_dev, value.st_ino, value.st_mode, value.st_uid, value.st_gid

class Sources:
    def __init__(self):
        fd = os.open('/', os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
        self.directories = {'/': (fd, directory_state(os.fstat(fd)))}
        self.files = {}

    def parent(self, name):
        path(name, absolute=True)
        parent = '/'
        for part in name.split('/')[1:-1]:
            child = parent.rstrip('/') + '/' + part
            if child not in self.directories:
                require(len(self.directories) < 512, 'source ancestor count')
                fd = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                             dir_fd=self.directories[parent][0])
                self.directories[child] = fd, directory_state(os.fstat(fd))
            parent = child
        return self.directories[parent][0]

    def revalidate(self):
        for name, (fd, identity) in self.directories.items():
            require(directory_state(os.fstat(fd)) == identity, 'source ancestor handle changed')
            if name == '/':
                named = os.stat('/', follow_symlinks=False)
            else:
                parent, leaf = name.rsplit('/', 1)
                named = os.stat(leaf, dir_fd=self.directories[parent or '/'][0], follow_symlinks=False)
            require(stat.S_ISDIR(named.st_mode) and directory_state(named) == identity,
                    'source ancestor moved, replaced or unlinked')

    def open(self, name):
        parent = self.parent(name)
        self.revalidate()
        fd = os.open(name.rsplit('/', 1)[1], os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC,
                     dir_fd=parent)
        value = os.fstat(fd)
        if not stat.S_ISREG(value.st_mode):
            os.close(fd)
            raise ValueError('ordinary source required')
        if name in self.files and state(value) != self.files[name]:
            os.close(fd)
            raise ValueError('source replaced or metadata changed')
        return fd, state(value)

    def finish(self, name, fd, before):
        after = os.fstat(fd)
        named = os.stat(name.rsplit('/', 1)[1], dir_fd=self.parent(name), follow_symlinks=False)
        require(state(after) == before and state(named) == before, 'source identity changed')
        self.revalidate()

    def verify(self, item):
        name = item['sourcePath']
        fd, before = self.open(name)
        digest, total = hashlib.sha256(), 0
        try:
            require(before[2] == item['bytes'], 'source byte length')
            while True:
                block = os.read(fd, 65536)
                if not block:
                    break
                total += len(block)
                require(total <= item['bytes'], 'source first extra byte')
                digest.update(block)
            require(total == item['bytes'] and digest.hexdigest() == item['sha256'], 'source digest')
            self.finish(name, fd, before)
            self.files[name] = before
        finally:
            os.close(fd)

    def revalidate_files(self):
        for name in self.files:
            fd, before = self.open(name)
            try:
                self.finish(name, fd, before)
            finally:
                os.close(fd)

    def read_plan(self, name, expected_bytes, expected_sha256):
        fd, before = self.open(name)
        try:
            require(before[2] == expected_bytes, 'externally selected plan size')
            output = bytearray()
            while True:
                block = os.read(fd, min(65536, expected_bytes + 1 - len(output)))
                if not block:
                    break
                output.extend(block)
                require(len(output) <= expected_bytes, 'plan first extra byte')
            require(len(output) == expected_bytes and hashlib.sha256(output).hexdigest() == expected_sha256,
                    'externally selected plan digest')
            self.finish(name, fd, before)
            self.files[name] = before
            return bytes(output)
        finally:
            os.close(fd)

    def close(self):
        errors = []
        for fd, _ in reversed(list(self.directories.values())):
            try:
                os.close(fd)
            except OSError as error:
                errors.append(error)
        self.directories.clear()
        if errors:
            raise ExceptionGroup('source handle cleanup failed', errors)
