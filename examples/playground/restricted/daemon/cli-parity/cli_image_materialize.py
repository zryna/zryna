"""Pinned compressed full349 archive -> private inert0400 cache, never filesystem extraction paths."""
import gzip
import hashlib
import os
import tarfile
import time
from cli_common import require
from cli_image_plan import EPOCH, ROOT


class Reader:
    def __init__(self, stream, deadline):
        self.stream, self.deadline, self.bytes = stream, deadline, 0
    def read(self, count):
        require(type(count) is int and 0 <= count <= 65536 and time.monotonic() < self.deadline,
                'finite original archive read')
        value = self.stream.read(count)
        self.bytes += len(value)
        require(self.bytes <= 156000000 and time.monotonic() < self.deadline, 'full349 archive expansion/deadline')
        return value


def materialize(document, sources, cache, cache_fd, deadline):
    source = document['archive']
    sources.verify(source)
    wanted = {ROOT + '/' + file['archivePath']: file for file in document['files']}
    directories = {ROOT}
    for name in wanted:
        parts = name.split('/')
        directories.update('/'.join(parts[:count]) for count in range(1, len(parts)))
    observed, files, count = set(), [], 0
    fd, before = sources.open(source['sourcePath'])
    try:
        with os.fdopen(os.dup(fd), 'rb') as compressed, gzip.GzipFile(fileobj=compressed, mode='rb') as stream:
            reader = Reader(stream, deadline)
            while True:
                header = reader.read(512)
                require(len(header) == 512, 'complete original USTAR header')
                if not any(header):
                    require(reader.read(512) == bytes(512) and not reader.read(1), 'exact original tar terminator/CRC')
                    break
                count += 1
                require(count <= 862, 'finite original tar inventory')
                entry = tarfile.TarInfo.frombuf(header, 'ascii', 'strict')
                name = entry.name
                require(name not in observed and entry.uid == entry.gid == 0 and entry.mtime == EPOCH
                        and not entry.linkname and entry.type in [tarfile.DIRTYPE, tarfile.REGTYPE],
                        'ordinary original archive metadata; no hidden PAX/GNU/links')
                observed.add(name)
                if entry.type == tarfile.DIRTYPE:
                    require(name in directories and entry.mode == 0o755 and entry.size == 0, 'exact original directory')
                    continue
                item = wanted.get(name)
                require(item is not None and entry.mode == item['mode'] and entry.size == item['bytes'],
                        'complete original file declaration/mode')
                leaf = str(len(files)).zfill(4) + '-' + item['sha256']
                destination = cache + '/' + leaf
                output = os.open(leaf, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC,
                                 0o400, dir_fd=cache_fd)
                digest, copied = hashlib.sha256(), 0
                try:
                    while copied < entry.size:
                        block = reader.read(min(65536, entry.size - copied))
                        require(block, 'complete original file bytes')
                        view = memoryview(block)
                        while view:
                            n = os.write(output, view)
                            require(n > 0, 'inert retained cache write')
                            view = view[n:]
                        digest.update(block); copied += len(block)
                    require(copied == item['bytes'] and digest.hexdigest() == item['sha256'], 'full original file digest')
                finally:
                    os.close(output)
                require(not any(reader.read((-entry.size) % 512)), 'zero original file padding')
                files.append({key: item[key] for key in ['path', 'bytes', 'sha256', 'mode']} | {'sourcePath': destination})
        require(observed == set(wanted) | directories and len(files) == 349, 'complete original349 topology')
        sources.finish(source['sourcePath'], fd, before)
    finally:
        os.close(fd)
    sources.verify(source)
    files += document['runtimeFiles']
    return {'files': sorted(files, key=lambda file: file['path']), 'directories': document['directories'],
            'bytes': document['bytes']}
