"""Only the adjacent hash-pinned fixed19 source plan can select an operation."""
import base64
import io
import os
import stat
import tarfile
from cli_common import exact, require, sha256
from cli_engine import decode

PLAN_SHA256 = 'd3fd1e15a7fd200aa402b05b8164726f2f4000f1489401c2414f71f408261efb'


def operation(identifier):
    require(type(identifier) is str and len(identifier) <= 128, 'fixed operation ID')
    path = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'fixed-plan.json')
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    try:
        before = os.fstat(fd)
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size == 38777,
                'ordinary pinned plan file')
        raw = os.read(fd, 38778)
        after = os.fstat(fd)
        require(all(getattr(before, k) == getattr(after, k) for k in
                    ['st_dev', 'st_ino', 'st_size', 'st_mode', 'st_uid', 'st_gid', 'st_mtime_ns', 'st_ctime_ns'])
                and sha256(raw) == PLAN_SHA256, 'fixed plan identity/bytes')
    finally:
        os.close(fd)
    plan = decode(raw)
    rows = [entry for entry in plan['operations'] if entry['id'] == identifier]
    require(len(rows) == 1 and len(plan['operations']) == 19, 'fixed operation membership')
    return rows[0]


def project_archive(selected):
    # Re-admit by ID, refusing a caller-mutated operation object.
    require(selected == operation(selected['id']), 'exact fixed operation')
    output = io.BytesIO()
    with tarfile.open(fileobj=output, mode='w', format=tarfile.USTAR_FORMAT) as archive:
        directory = tarfile.TarInfo('src')
        directory.type, directory.mode, directory.uid, directory.gid = tarfile.DIRTYPE, 0o700, 65534, 65534
        archive.addfile(directory)
        for file in selected['projectFiles']:
            exact(file, ['path', 'bytes', 'sha256', 'mode', 'bodyBase64'])
            require(file['path'] in ['src/main.zry', 'zryna.package.json', 'zryna.lock.json'], 'fixed project files')
            data = base64.b64decode(file['bodyBase64'], validate=True)
            require(sha256(data) == file['sha256'] and len(data) == file['bytes'] <= 16384
                    and file['mode'] == 0o600, 'fixed project source bytes/mode')
            info = tarfile.TarInfo(file['path'])
            info.uid, info.gid, info.mode, info.size = 65534, 65534, 0o600, len(data)
            archive.addfile(info, io.BytesIO(data))
    raw = output.getvalue()
    require(0 < len(raw) <= 32768, 'bounded fixed project tar')
    return raw
