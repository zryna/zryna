"""Acquire one pinned source-only SQLite fixture; never execute downloaded bytes."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import stat
import time
import urllib.request
import zipfile

SPEC_PATH = Path(__file__).with_name('sqlite_fixture.json')
SPEC = json.loads(SPEC_PATH.read_bytes())


def require(condition, message):
    if not condition:
        raise ValueError(message)


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':')) + '\n').encode()


def identity(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_uid,
            value.st_nlink, value.st_size, value.st_mtime_ns, value.st_ctime_ns)


def read_fd(fd, maximum, private=False):
    before = os.fstat(fd)
    require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1,
            'input must be a regular single-link file')
    require(before.st_size <= maximum, 'file exceeds input limit')
    if private:
        require(before.st_uid == os.getuid() and stat.S_IMODE(before.st_mode) == 0o400,
                'cache file ownership or permissions changed')
    pieces, size = [], 0
    while True:
        chunk = os.read(fd, min(65536, maximum + 1 - size))
        if not chunk:
            break
        size += len(chunk)
        require(size <= maximum, 'file grew beyond input limit')
        pieces.append(chunk)
    require(identity(before) == identity(os.fstat(fd)), 'file changed during read')
    return b''.join(pieces)


def read_archive(path):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        return read_fd(fd, SPEC['limits']['archiveBytes'])
    finally:
        os.close(fd)


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise ValueError('SQLite source redirects are not admitted')


def download():
    # Default HTTPS handler retains system TLS trust, configured proxies and CA.
    opener = urllib.request.build_opener(NoRedirect())
    request = urllib.request.Request(SPEC['url'], headers={'Accept-Encoding': 'identity'})
    started = time.monotonic()
    with opener.open(request, timeout=SPEC['limits']['requestSeconds']) as response:
        require(response.status == 200 and response.geturl() == SPEC['url'],
                'unexpected SQLite HTTP response')
        require(response.headers.get('Content-Encoding', 'identity') == 'identity',
                'encoded HTTP body is not the pinned archive')
        length = response.headers.get('Content-Length')
        require(length is None or length == str(SPEC['archiveBytes']),
                'unexpected SQLite HTTP content length')
        chunks, size = [], 0
        while True:
            require(time.monotonic() - started <= SPEC['limits']['totalSeconds'],
                    'SQLite request exceeded total deadline')
            data = response.read(min(65536, SPEC['limits']['archiveBytes'] + 1 - size))
            if not data:
                break
            size += len(data)
            require(size <= SPEC['limits']['archiveBytes'], 'SQLite download exceeds ceiling')
            chunks.append(data)
    require(time.monotonic() - started <= SPEC['limits']['totalSeconds'],
            'SQLite request exceeded total deadline')
    return b''.join(chunks)


def authenticate_archive(data):
    require(len(data) == SPEC['archiveBytes'], 'SQLite archive byte count mismatch')
    require(len(data) <= SPEC['limits']['archiveBytes'], 'SQLite archive exceeds ceiling')
    require(hashlib.sha3_256(data).hexdigest() == SPEC['archiveSha3_256'],
            'SQLite archive SHA3-256 mismatch')
    return unpack(data)


def unpack(data):
    # Caller authenticates archive bytes before invoking the ZIP parser.
    root = SPEC['archiveRoot'] + '/'
    expected = {root, *(root + name for name in SPEC['members'])}
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        entries = archive.infolist()
        require(len(entries) <= SPEC['limits']['entries'], 'too many ZIP entries')
        names = [item.filename for item in entries]
        require(len(names) == len(set(names)), 'duplicate ZIP name')
        require(set(names) == expected, 'unexpected or missing ZIP member')
        require(sum(item.file_size for item in entries) <= SPEC['limits']['expandedBytes'],
                'ZIP expansion exceeds ceiling')
        selected = {}
        for item in entries:
            require(item.filename == item.orig_filename and '\\' not in item.filename
                    and not any(part in ('.', '..') for part in item.filename.split('/')),
                    'unsafe ZIP path')
            kind = stat.S_IFMT(item.external_attr >> 16)
            require(kind in ((0, stat.S_IFDIR) if item.is_dir() else (0, stat.S_IFREG)),
                    'linked or special ZIP member')
            require(item.flag_bits == 0 and item.compress_type in (0, 8),
                    'unsupported ZIP encoding or encryption')
            name = item.filename.removeprefix(root)
            if name in SPEC['files']:
                fixed = SPEC['files'][name]
                require(item.file_size == fixed['bytes'] <= fixed['maxBytes'],
                        'SQLite source member exceeds fixed size')
                with archive.open(item) as stream:
                    value = stream.read(fixed['maxBytes'] + 1)
                require(len(value) == fixed['bytes'] and
                        hashlib.sha256(value).hexdigest() == fixed['sha256'],
                        'SQLite source SHA-256 mismatch')
                selected[name] = value
    require(set(selected) == set(SPEC['files']), 'missing SQLite source')
    for value in selected.values():
        for key, expected_value in [('VERSION', '"' + SPEC['version'] + '"'),
                                    ('VERSION_NUMBER', str(SPEC['versionNumber'])),
                                    ('SOURCE_ID', '"' + SPEC['sourceId'] + '"')]:
            matches = re.findall(rb'^#define SQLITE_' + key.encode() + rb' +([^\r\n]+)',
                                 value, re.MULTILINE)
            require(matches == [expected_value.encode()], 'SQLite version/source-id mismatch')
    return selected


def receipt():
    return canonical({'format': SPEC['format'], 'manifestCanonicalSha256':
                      hashlib.sha256(canonical(SPEC)).hexdigest(), 'files': SPEC['files'],
                      'archiveSha3_256': SPEC['archiveSha3_256']})


def directory(path):
    """Walk without symlinks; create only the final component, with mode0700."""
    path = Path(path).absolute()
    require(path.name and '..' not in path.parts, 'unsafe output path')
    require(not path.is_relative_to(Path(__file__).resolve().parents[2]),
            'SQLite source output must be outside checkout')
    fd = os.open('/', os.O_RDONLY | os.O_DIRECTORY)
    try:
        for part in path.parts[1:-1]:
            child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=fd)
            os.close(fd)
            fd = child
        created = False
        try:
            os.mkdir(path.name, 0o700, dir_fd=fd)
            created = True
        except FileExistsError:
            pass
        child = os.open(path.name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=fd)
        try:
            value = os.fstat(child)
            require(value.st_uid == os.getuid() and stat.S_IMODE(value.st_mode) == 0o700,
                    'cache directory must be owned mode0700')
        except BaseException:
            os.close(child)
            raise
        return path, child, created
    finally:
        os.close(fd)


def cached(fd):
    expected = {'archive.zip', 'receipt.json', *SPEC['files']}
    require(set(os.listdir(fd)) == expected, 'incomplete or unexpected cache inventory')
    contents = {}
    for name in sorted(expected):
        maximum = (SPEC['limits']['archiveBytes'] if name == 'archive.zip' else
                   SPEC['files'][name]['maxBytes'] if name in SPEC['files'] else 4096)
        file_fd = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=fd)
        try:
            contents[name] = read_fd(file_fd, maximum, private=True)
        finally:
            os.close(file_fd)
    selected = authenticate_archive(contents['archive.zip'])
    require(contents['receipt.json'] == receipt(), 'cache receipt mismatch')
    require(all(contents[name] == value for name, value in selected.items()),
            'cached SQLite source bytes changed')
    return selected


def prepare(output, archive=None):
    path, fd, created = directory(output)
    try:
        if created:
            data = read_archive(archive) if archive is not None else download()
            selected = authenticate_archive(data)
            for name, value in {'archive.zip': data, **selected, 'receipt.json': receipt()}.items():
                file_fd = os.open(name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
                                  0o400, dir_fd=fd)
                with os.fdopen(file_fd, 'wb') as stream:
                    stream.write(value)
                    stream.flush()
                    os.fsync(stream.fileno())
        cached(fd)
        current = os.stat(path, follow_symlinks=False)
        require((current.st_dev, current.st_ino) == (os.fstat(fd).st_dev, os.fstat(fd).st_ino),
                'output directory was replaced')
        return path
    finally:
        os.close(fd)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True)
    parser.add_argument('--archive', help='offline local copy of the exact pinned ZIP')
    args = parser.parse_args()
    print(prepare(args.output, args.archive))


if __name__ == '__main__':
    main()
