"""Bounded actual empty/staged project tar observations before any CLI execution."""
import tarfile
from cli_common import require, sha256


def inspect_project(raw, operation, populated):
    require(type(raw) is bytes and 1536 <= len(raw) <= 32768 and len(raw) % 512 == 0,
            'bounded project archive')
    expected = {'project': (tarfile.DIRTYPE, 0o700, 0, None)}
    if populated:
        expected['project/src'] = (tarfile.DIRTYPE, 0o700, 0, None)
        for file in operation['projectFiles']:
            expected['project/' + file['path']] = (tarfile.REGTYPE, file['mode'], file['bytes'], file['sha256'])
    observed, offset = {}, 0
    while offset + 512 <= len(raw) and any(raw[offset:offset + 512]):
        entry = tarfile.TarInfo.frombuf(raw[offset:offset + 512], 'utf8', 'strict')
        require(entry.type in [tarfile.REGTYPE, tarfile.DIRTYPE], 'no hidden PAX/GNU/link/control tar members')
        name = entry.name.rstrip('/')
        require(name in expected and entry.name in [name, name + '/'] and name not in observed and len(observed) < 5
                and not entry.linkname and entry.uid == entry.gid == 65534, 'closed project tar identity/members')
        kind, mode, size, digest = expected[name]
        require(entry.type == kind and entry.mode == mode and entry.size == size, 'actual project type/mode/size')
        content = raw[offset + 512:offset + 512 + size]
        end = offset + 512 + ((size + 511) // 512) * 512
        require(end <= len(raw) and not any(raw[offset + 512 + size:end]) and
                len(content) == size and (digest is None or sha256(content) == digest), 'actual fixed source bytes/padding')
        observed[name] = {'mode': mode, 'bytes': size, 'sha256': sha256(content) if digest else None}
        offset = end
    require(set(observed) == set(expected) and len(raw) - offset >= 1024 and not any(raw[offset:]),
            'complete tar including only zero end blocks')
    return {'bytes': len(raw), 'sha256': sha256(raw), 'members': observed, 'fullInventoryChecked': True}


def capture(engine, cid, operation, populated):
    raw, _ = engine.request('GET', '/containers/' + cid + '/archive?path=%2Fwork%2Fproject',
                            statuses=(200,), maximum_response=32768)
    return inspect_project(raw, operation, populated)
