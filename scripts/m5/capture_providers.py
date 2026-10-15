"""Filesystem-only complete selected materials. No compiler/loader/script execution."""
from collections import deque
from pathlib import Path
import hashlib
import os
import re
import stat
from capture_io import Directory, REPORT_LIMIT, parse_json, sha, stamp, write_json

ROOTS = ('/usr/lib', '/usr/lib64', '/lib', '/lib64')
TOOL_ROOTS = ('/usr/bin', '/usr/lib', '/usr/lib64')
PROVIDER_LIMIT = 32 * 1024 * 1024
TOOL_LIMIT = 64 * 1024 * 1024


def walk(path, roots=ROOTS):
    assert isinstance(path, str) and path.startswith('/') and len(path) <= 4096
    assert '\0' not in path and not path.startswith('/proc/')
    pending = deque(path.split('/'))
    fds = [os.open('/', os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)]
    names, epoch, links, steps = [], [], 0, 0
    try:
        while pending:
            token = pending.popleft()
            steps += 1
            assert steps <= 512
            if token in ('', '.'):
                continue
            if token == '..':
                assert names
                os.close(fds.pop())
                names.pop()
                continue
            assert len(token) <= 255
            before = os.stat(token, dir_fd=fds[-1], follow_symlinks=False)
            full = '/' + '/'.join(names + [token])
            assert len(full) <= 4096
            if stat.S_ISLNK(before.st_mode):
                links += 1
                assert links <= 40
                target = os.readlink(token, dir_fd=fds[-1])
                assert len(target) <= 4096 and '\0' not in target
                assert stamp(before) == stamp(os.stat(token, dir_fd=fds[-1],
                                                      follow_symlinks=False))
                epoch.append({'kind': 'symlink', 'path': full,
                              'descriptor': stamp(before), 'target': target})
                if target.startswith('/'):
                    while len(fds) > 1:
                        os.close(fds.pop())
                    names = []
                pending.extendleft(reversed(target.split('/')))
                continue
            if not any(x not in ('', '.') for x in pending):
                assert any(full.startswith(root + '/') for root in roots)
                assert stat.S_ISREG(before.st_mode)
                fd = os.open(token, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK,
                             dir_fd=fds[-1])
                try:
                    assert stamp(os.fstat(fd)) == stamp(before)
                except BaseException:
                    os.close(fd)
                    raise
                epoch.append({'kind': 'regular', 'path': full,
                              'descriptor': stamp(before)})
                return fd, full, epoch
            assert stat.S_ISDIR(before.st_mode)
            fd = os.open(token, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                         dir_fd=fds[-1])
            try:
                assert stamp(os.fstat(fd)) == stamp(before)
            except BaseException:
                os.close(fd)
                raise
            epoch.append({'kind': 'directory', 'path': full,
                          'identity': stamp(before)[:5]})
            fds.append(fd)
            names.append(token)
        raise AssertionError('no terminal selected regular material')
    finally:
        for fd in reversed(fds):
            os.close(fd)


def copy_hash(fd, maximum, dest=None):
    os.lseek(fd, 0, os.SEEK_SET)
    before = os.fstat(fd)
    assert 0 < before.st_size <= maximum
    length, prefix, digest = 0, b'', hashlib.sha256()
    while data := os.read(fd, 1048576):
        length += len(data)
        assert length <= before.st_size
        digest.update(data)
        if len(prefix) < 65536:
            prefix = (prefix + data)[:65536]
        if dest is not None:
            view = memoryview(data)
            while view:
                view = view[os.write(dest, view):]
    assert length == before.st_size and stamp(os.fstat(fd)) == stamp(before)
    return length, digest.hexdigest(), prefix


def tool_identity(value):
    return ':'.join(str(x) for x in [value.st_dev, value.st_ino, value.st_size,
        value.st_mode, value.st_mtime_ns // 1000000000, value.st_mtime_ns % 1000000000,
        value.st_ctime_ns // 1000000000, value.st_ctime_ns % 1000000000])


def capture(paths, output, expected=None, roots=ROOTS):
    assert 0 < len(paths) <= (3 if expected is not None else 64)
    assert len(paths) == len(set(paths)) and len({x.casefold() for x in paths}) == len(paths)
    if expected is not None:
        assert len(expected) == len(paths) == 3
    aliases, materials, held_files, total = [], {}, [], 0
    maximum = TOOL_LIMIT if expected is not None else PROVIDER_LIMIT
    with Directory(output) as directory:
        assert not list(Path(output).iterdir())
        try:
            for index, path in enumerate(paths):
                fd, canonical, epoch = walk(path, roots)
                held_files.append(fd)
                before = os.fstat(fd)
                if expected is not None:
                    assert expected[index]['path'] == path == canonical
                    assert expected[index]['identity'] == tool_identity(before)
                key = (before.st_dev, before.st_ino)
                if key not in materials:
                    total += before.st_size
                    assert total <= maximum
                    name = 'material-' + str(index).zfill(2) + '.bytes'
                    dest = os.open(name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
                                   0o600, dir_fd=directory.fd)
                    try:
                        length, digest, prefix = copy_hash(fd, maximum, dest)
                        os.fsync(dest)
                    finally:
                        os.close(dest)
                    if prefix.startswith(b'\x7fELF'):
                        kind, script = 'ELF opaque bytes', None
                    elif expected is None and prefix.startswith(b'!<arch>\n'):
                        kind, script = 'ar opaque bytes', None
                    else:
                        assert expected is None and length <= 65536
                        script = prefix.decode('utf-8')
                        assert 'GNU ld script' in script and re.search(r'\b(?:GROUP|INPUT)\s*\(', script)
                        kind = 'GNU ld script opaque data; directives never resolved or executed'
                    materials[key] = {'canonicalPath': canonical, 'descriptor': stamp(before),
                        'bytes': length, 'sha256': digest, 'retainedFile': name,
                        'kind': kind, 'linkerScriptText': script}
                else:
                    length, digest, _ = copy_hash(fd, maximum)
                    assert (length, digest) == (materials[key]['bytes'], materials[key]['sha256'])
                if expected is not None:
                    assert (length, digest) == (expected[index]['bytes'], expected[index]['sha256'])
                aliases.append({'tracePath': path, 'canonicalPath': canonical,
                                'componentEpoch': epoch, 'material': materials[key]})
            for alias, fd in zip(aliases, held_files):
                again, canonical, epoch = walk(alias['tracePath'], roots)
                try:
                    assert canonical == alias['canonicalPath'] and epoch == alias['componentEpoch']
                    for selected in (fd, again):
                        length, digest, _ = copy_hash(selected, maximum)
                        assert (length, digest) == (alias['material']['bytes'], alias['material']['sha256'])
                finally:
                    os.close(again)
            report = {'format': 'zryna.m5.hosted-prefix-material-observation.v1',
                'aliases': aliases, 'retainedMaterials': list(materials.values()),
                'uniqueMaterialCount': len(materials), 'uniqueMaterialBytes': total,
                'providerPolicy': 'UNBOUND_MANUAL_REVIEW', 'targets': 'UNRUN',
                'completeInputUse': False, 'loaderSelectionProven': False,
                'upstreamPublisherAuthentication': False, 'privateRecipeQualified': False}
            pin = write_json(directory, 'MATERIALS.json', report)
            assert pin['bytes'] <= REPORT_LIMIT
            verify_inventory(report, output)
            return report
        finally:
            for fd in held_files:
                os.close(fd)


def verify_inventory(report, output):
    expected = {x['retainedFile'] for x in report['retainedMaterials']} | {'MATERIALS.json'}
    assert len(expected) == len(report['retainedMaterials']) + 1
    assert {x.name for x in Path(output).iterdir()} == expected
    with Directory(output) as held:
        assert parse_json(held.read('MATERIALS.json', REPORT_LIMIT)) == report
        for material in report['retainedMaterials']:
            data = held.read(material['retainedFile'], TOOL_LIMIT)
            assert len(data) == material['bytes'] and sha(data) == material['sha256']
        held.check()
