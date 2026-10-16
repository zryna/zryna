"""Finite ordinary-file archive transport. This module never installs or approves anything."""
import hashlib
import os
import tarfile
from cli_common import require


def metadata(name, mode, size=0, directory=False):
    info = tarfile.TarInfo(name)
    info.type = tarfile.DIRTYPE if directory else tarfile.REGTYPE
    info.uid = info.gid = info.mtime = 0
    info.mode, info.size = mode, size
    # Validate USTAR representability before any output, including prefix splitting.
    info.tobuf(format=tarfile.USTAR_FORMAT, encoding='ascii', errors='strict')
    return info


class Output:
    def __init__(self, stream):
        self.stream, self.bytes, self.digest = stream, 0, hashlib.sha256()

    def write(self, block):
        require(len(block) <= 65536, 'bounded archive write')
        view = memoryview(block)
        while view:
            written = self.stream.write(view)
            require(type(written) is int and 0 < written <= len(view), 'archive short write')
            self.digest.update(view[:written])
            self.bytes += written
            view = view[written:]
        return len(block)


class Input:
    def __init__(self, fd):
        self.fd, self.bytes, self.digest = fd, 0, hashlib.sha256()

    def read(self, count):
        require(type(count) is int and 0 <= count <= 65536, 'bounded archive read')
        output = bytearray()
        while len(output) < count:
            block = os.read(self.fd, count - len(output))
            if not block:
                break
            output.extend(block)
        self.bytes += len(output)
        self.digest.update(output)
        return bytes(output)


def emit(document, sources, output):
    headers = [metadata('.', 365, directory=True)]
    headers.extend(metadata(name, 365, directory=True) for name in document['directories'])
    headers.extend(metadata(item['path'], item['mode'], item['bytes']) for item in document['files'])
    # No archive header is released until *all* metadata and source bytes are checked.
    for item in document['files']:
        sources.verify(item)
    sources.revalidate_files()
    destination = Output(output)
    with tarfile.open(fileobj=destination, mode='w|', format=tarfile.USTAR_FORMAT,
                      encoding='ascii', errors='strict') as archive:
        for header in headers[:1 + len(document['directories'])]:
            archive.addfile(header)
        for item, header in zip(document['files'], headers[1 + len(document['directories']):]):
            fd, before = sources.open(item['sourcePath'])
            try:
                captured = Input(fd)
                archive.addfile(header, captured)
                require(not os.read(fd, 1), 'source first extra byte during archive')
                require(captured.bytes == item['bytes'] and
                        captured.digest.hexdigest() == item['sha256'], 'archive source digest')
                sources.finish(item['sourcePath'], fd, before)
            finally:
                os.close(fd)
    # A consumer MUST require emitter exit zero even if a failed stream looks complete.
    for item in document['files']:
        sources.verify(item)
    sources.revalidate_files()
    output.flush()
    return {'files': len(document['files']), 'sourceBytes': document['bytes'],
            'archiveBytes': destination.bytes, 'archiveSha256': destination.digest.hexdigest()}
