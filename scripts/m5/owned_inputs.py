"""Source-selected compiler inputs: complete bytes and epochs, never report-selected exec."""
from pathlib import Path
import hashlib
import os
import stat
from capture_io import Directory, REPORT_LIMIT, read, sha, stamp, write_json

SOURCE_ROOTS = ('crates/zryna-driver/src', 'tests', 'runtime', 'spec', 'adapters', 'examples')
# Existing --cfg=test driver modules consume these exact source files and fixture manifest.
SOURCE_FILES = (
    'crates/zryna-native-mir/tests/native_c_v0/capture.rs',
    'crates/zryna-native-c-ir/tests/capture.rs',
    'crates/zryna-semantics/src/data_ownership_v1/tests/aggregate_contracts.rs',
    'crates/zryna-ownership-runtime-abi/include/zryna_ownership_runtime_v1.h',
    'scripts/m5/sqlite_fixture.json',
)
INPUT_BYTES = 6 * 1024 * 1024 * 1024  # Existing normal Cargo target bound; no private inventory claim.
INPUT_ENTRIES = 10000


def file_pin(path, maximum=512 * 1024 * 1024, *, allow_empty=False):
    assert isinstance(allow_empty, bool)
    path = Path(path)
    with Directory(path.parent) as parent:
        before = os.stat(path.name, dir_fd=parent.fd, follow_symlinks=False)
        assert stat.S_ISREG(before.st_mode) and (0 if allow_empty else 1) <= before.st_size <= maximum, (
            'regular compiler input size', str(path), before.st_mode, before.st_size, maximum)
        fd = os.open(path.name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=parent.fd)
        try:
            assert stamp(os.fstat(fd)) == stamp(before)
            total, digest = 0, hashlib.sha256()
            while data := os.read(fd, 1048576):
                total += len(data)
                assert total <= before.st_size
                digest.update(data)
            assert total == before.st_size and stamp(os.fstat(fd)) == stamp(before)
            assert stamp(os.stat(path.name, dir_fd=parent.fd, follow_symlinks=False)) == stamp(before)
            parent.check()
            return {'path': str(path), 'bytes': total, 'sha256': digest.hexdigest(),
                    'epoch': stamp(before)}
        finally:
            os.close(fd)


def snapshot_inputs(dependencies, sysroot):
    paths = []
    for directory in (Path(dependencies), Path(sysroot) / 'lib/rustlib/x86_64-unknown-linux-gnu/lib'):
        with Directory(directory) as held:
            for item in sorted(directory.iterdir()):
                if item.suffix in ('.rlib', '.rmeta', '.so'):
                    assert len(paths) < INPUT_ENTRIES
                    paths.append(item)
            held.check()
    assert paths and len(paths) == len(set(paths))
    rows, total = [], 0
    for path in paths:
        row = file_pin(path, allow_empty=path.suffix == '.rmeta')
        total += row['bytes']
        assert total <= INPUT_BYTES
        rows.append(row)
    return rows


def check_inputs(rows, dependencies, sysroot):
    assert snapshot_inputs(dependencies, sysroot) == rows


def parent(root, relative):
    parts = Path(relative).parts
    assert parts and all(p not in ('', '.', '..') for p in parts)
    current = Path(root)
    for name in parts[:-1]:
        with Directory(current) as held:
            try:
                os.mkdir(name, mode=0o700, dir_fd=held.fd)
            except FileExistsError:
                pass
            held.check()
        current /= name
        with Directory(current) as held:
            held.check()
    return current, parts[-1]


def mirror(repo, output, source):
    root = Path(output) / 'mirror'
    with Directory(output) as held:
        os.mkdir('mirror', mode=0o700, dir_fd=held.fd)
    rows = [row for row in source['files'] if row['path'] in SOURCE_FILES or any(
        row['path'].startswith(prefix + '/') for prefix in SOURCE_ROOTS)]
    assert set(SOURCE_FILES) <= {row['path'] for row in rows}, 'complete fixed cross-crate includes'
    assert rows and len(rows) <= INPUT_ENTRIES
    total, manifest = 0, []
    for row in rows:
        data = read(Path(repo) / row['path'], 2 * 1024 * 1024)
        assert len(data) == row['bytes'] and sha(data) == row['sha256']
        total += len(data)
        assert total <= 16 * 1024 * 1024
        if row['path'] == 'crates/zryna-driver/src/lib.rs':
            entry = read(Path(repo) / 'scripts/m5/owned_sanitizer_runtime_main.rs', REPORT_LIMIT)
            data += b'\n' + entry
        directory, name = parent(root, row['path'])
        with Directory(directory) as held:
            held.write(name, data)
        manifest.append({'path': row['path'], 'originalBytes': row['bytes'],
                         'originalSha256': row['sha256'], 'mirrorBytes': len(data),
                         'mirrorSha256': sha(data)})
    with Directory(output) as held:
        write_json(held, 'SOURCE_MIRROR.json', {'roots': SOURCE_ROOTS,
            'includeFiles': SOURCE_FILES, 'files': manifest,
            'appendOnly': 'crates/zryna-driver/src/lib.rs + source-owned fixed main',
            'originalSourceBytes': total})
    return root, manifest


def check_mirror(repo, root, manifest):
    for row in manifest:
        original = read(Path(repo) / row['path'], 2 * 1024 * 1024)
        assert len(original) == row['originalBytes'] and sha(original) == row['originalSha256']
        expected = original
        if row['path'] == 'crates/zryna-driver/src/lib.rs':
            expected += b'\n' + read(Path(repo) / 'scripts/m5/owned_sanitizer_runtime_main.rs', REPORT_LIMIT)
        actual = read(Path(root) / row['path'], 3 * 1024 * 1024)
        assert actual == expected and len(actual) == row['mirrorBytes'] and sha(actual) == row['mirrorSha256']
    named = {str(p.relative_to(root)) for p in Path(root).rglob('*') if p.is_file()}
    assert named == {row['path'] for row in manifest}
    assert not any(p.is_symlink() for p in Path(root).rglob('*'))
