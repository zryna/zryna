"""Native ext4 backing admission before reusing the unchanged null-mask topology guard."""
import os
import re
import stat
from contextlib import ExitStack
from cli_common import require
from cli_mounts import resolve as original_resolve
from preparation_primitives import Sources

FILES = ['hostname', 'hosts', 'resolv.conf']
FIELDS = ['st_dev', 'st_ino', 'st_mode', 'st_uid', 'st_gid', 'st_nlink', 'st_mtime_ns', 'st_ctime_ns']


def rows(text):
    require(type(text) is str and 0 < len(text) <= 262144, 'bounded native mountinfo')
    result = []
    for line in text.splitlines():
        left, separator, right = line.partition(' - ')
        before, after = left.split(), right.split()
        require(separator and len(before) >= 6 and len(after) == 3 and len(result) < 512,
                'complete bounded native mountinfo rows')
        target = before[4]
        require(target.startswith('/') and '\\' not in target and '//' not in target
                and all(part not in ['.', '..'] for part in target.split('/')),
                'canonical native mountpoint')
        result.append((left, before, after))
    return result


def backing_metadata(cid, device):
    result, opened = [], []
    with ExitStack() as stack:
        sources = Sources()
        stack.callback(sources.close)
        for name in FILES:
            path = '/var/lib/docker/containers/' + cid + '/' + name
            fd, before = sources.open(path)
            stack.callback(os.close, fd)
            opened.append((path, fd, before))
        entries = [(path, fd, True) for path, (fd, _) in sources.directories.items()]
        entries.extend((path, fd, False) for path, fd, _ in opened)
        for path, fd, directory in entries:
            value = os.fstat(fd)
            require(value.st_uid == 0 and not value.st_mode & 0o022
                    and f'{os.major(value.st_dev)}:{os.minor(value.st_dev)}' == device,
                    'root-owned native backing on the observed root device')
            require(stat.S_ISDIR(value.st_mode) if directory else
                    stat.S_ISREG(value.st_mode) and value.st_nlink == 1,
                    'ordinary no-link native backing ancestry/file')
            result.append({'path': path, **{field: str(getattr(value, field)) for field in FIELDS}})
        for path, fd, before in opened:
            sources.finish(path, fd, before)
        sources.revalidate()
    return result


def resolve(text, cid, host_text, docker_root):
    require(type(cid) is str and re.fullmatch('[a-f0-9]{64}', cid)
            and docker_root == '/var/lib/docker', 'fixed native Docker backing coordinates')
    host = rows(host_text)
    roots = [row for row in host if row[1][4] == '/']
    require(len(roots) == 1, 'sole native host root')
    _, root, filesystem = roots[0]
    require(root[3] == '/' and filesystem[:2] == ['ext4', '/dev/root']
            and re.fullmatch('[0-9]+:[0-9]+', root[2])
            and 'rw' in root[5].split(',') and 'ro' not in root[5].split(','),
            'observed unrebased native ext4 root backing')
    prefix = docker_root + '/containers/' + cid + '/'
    for name in FILES:
        path = prefix + name
        require(not any(before[4] != '/' and
                        (path == before[4] or path.startswith(before[4].rstrip('/') + '/'))
                        for _, before, _ in host), 'no covering host mount over native Docker files')
    metadata = backing_metadata(cid, root[2])
    container = rows(text)
    original, projected = {}, []
    for left, before, after in container:
        target = before[4]
        if target in ['/etc/' + name for name in FILES] and before[3] == prefix + target.rsplit('/', 1)[1]:
            require(target not in original and before[2] == root[2] and after[:2] == filesystem[:2],
                    'exact native root-backed Docker file mount')
            original[target] = {'filesystem': after[0], 'source': after[1]}
            after = ['overlay', 'overlay', after[2]]
        projected.append(left + ' - ' + ' '.join(after))
    require(set(original) == {'/etc/' + name for name in FILES}, 'all three native Docker file bases')
    # Only the already-admitted base filesystem labels are projected. The pinned checker
    # still verifies every stack, parent/device/null-mask, readonly proc and mount ancestry.
    result = original_resolve('\n'.join(projected) + '\n', cid)
    for target, actual in original.items():
        result[target]['stack']['base'].update(actual)
        result[target]['nativeBacking'] = {'hostRootDevice': root[2], 'filesystem': filesystem[0],
                                          'source': filesystem[1], 'metadata': metadata}
    return result
