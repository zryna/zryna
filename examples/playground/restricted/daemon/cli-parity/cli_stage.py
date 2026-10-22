"""Fixed trusted Node bootstrap writes/readbacks in the actual private project tmpfs."""
import base64
import json
import os
import re
import stat
from cli_common import ENV, exact, require, sha256
from cli_engine import decode
from cli_exec import exec_identity, streams
from cli_plan import operation, project_archive

SCRIPT_SHA256 = '3f2d3b9dfee3c775af5c8bdeefe87d5f4888d2495948c3815c48800269f9f9f3'
NODE = '/opt/zryna-release/runtime/node/bin/node'
ENTRY = "\nprocess.stdout.write(JSON.stringify(stage(JSON.parse(Buffer.from(process.argv[1], 'base64').toString('utf8'))))+'\\n');\n"


def command(identifier):
    selected = operation(identifier)
    project_archive(selected)  # Re-admit all closed source bytes/modes before executable preparation.
    path = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'project-stage.cjs')
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    try:
        before = os.fstat(fd)
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size == 4889,
                'ordinary fixed staging bootstrap')
        raw = os.read(fd, 4890)
        after = os.fstat(fd)
        require(all(getattr(before, key) == getattr(after, key) for key in
                    ['st_dev', 'st_ino', 'st_size', 'st_mode', 'st_uid', 'st_gid', 'st_mtime_ns', 'st_ctime_ns'])
                and sha256(raw) == SCRIPT_SHA256, 'fixed staging bootstrap identity/bytes')
    finally:
        os.close(fd)
    request = {'version': 1, 'operationId': identifier, 'files': selected['projectFiles']}
    body = json.dumps(request, separators=(',', ':'), ensure_ascii=True).encode()
    require(0 < len(body) <= 32768, 'bounded closed staging source data')
    return {'executable': NODE, 'argv': ['--eval', raw.decode('ascii') + ENTRY,
                                        base64.b64encode(body).decode('ascii')]}


def receipt(value, identifier):
    selected = operation(identifier)
    exact(value, ['version', 'operationId', 'sourceStaged', 'filesystem', 'before', 'after'])
    require(type(value['version']) is int and value['version'] == 1 and value['operationId'] == identifier and
            value['sourceStaged'] is True and value['filesystem'] == 'tmpfs', 'actual staging result domain')
    exact(value['before'], ['empty', 'root']); exact(value['after'], ['root', 'src', 'files'])
    root = value['before']['root']; exact(root, ['dev', 'ino', 'uid', 'gid', 'mode'])
    require(value['before']['empty'] is True and value['after']['root'] == root and
            all(type(root[key]) is int and 0 < root[key] <= 9007199254740991 for key in ['dev', 'ino']) and
            root['uid'] == root['gid'] == 65534 and root['mode'] == 0o700, 'actual fresh writable project identity')
    require(json.dumps(value['after']['src'], sort_keys=True) ==
            json.dumps({'mode': 0o700, 'uid': 65534, 'gid': 65534, 'sameDevice': True}, sort_keys=True),
            'actual same-device private source directory')
    expected = sorted([{'path': file['path'], 'bytes': file['bytes'], 'mode': 0o600, 'uid': 65534,
                        'gid': 65534, 'nlink': 1, 'sha256': file['sha256']}
                       for file in selected['projectFiles']], key=lambda file: file['path'])
    require(json.dumps(value['after']['files'], sort_keys=True) == json.dumps(expected, sort_keys=True), 'complete live source readback bytes/hashes/modes/inventory')
    return value


def run(engine, cid, identifier, record):
    selected = command(identifier)
    request = {'AttachStdin': False, 'AttachStdout': True, 'AttachStderr': True, 'Tty': False,
        'Privileged': False, 'User': '65534:65534', 'WorkingDir': '/work/project', 'Env': ENV,
        'Cmd': [selected['executable'], *selected['argv']]}
    created = engine.json('POST', '/containers/' + cid + '/exec', request)
    eid = created['Id']; require(type(eid) is str and re.fullmatch('[a-f0-9]{64}', eid), 'actual staging exec ID')
    record['execId'], record['request'] = eid, request
    initial = engine.json('GET', '/exec/' + eid + '/json'); exec_identity(initial, eid, cid, selected, False)
    record['initial'] = initial
    raw, _ = engine.request('POST', '/exec/' + eid + '/start', value={'Detach': False, 'Tty': False},
                            statuses=(200,), maximum_response=32768)
    record['wireBytes'], record['wireSha256'] = len(raw), sha256(raw)
    stdout, stderr = streams(raw)
    record['capture'] = {'stdoutBase64': base64.b64encode(stdout).decode(),
        'stderrBase64': base64.b64encode(stderr).decode(), 'stdoutBytes': len(stdout), 'stderrBytes': len(stderr),
        'stdoutSha256': sha256(stdout), 'stderrSha256': sha256(stderr)}
    final = engine.json('GET', '/exec/' + eid + '/json'); exec_identity(final, eid, cid, selected, True)
    record['final'] = final
    require(final['Running'] is False and type(final['ExitCode']) is int and final['ExitCode'] == 0 and
            not stderr and 0 < len(stdout) <= 16384, 'actual successful staging exec/status/streams')
    result = receipt(decode(stdout), identifier)
    record['verified'] = result
    return result
