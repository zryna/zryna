"""Only fixed audit/bootstrap/libtest commands, original attached streams before parsing."""
import base64
import json
import re
from cli_common import require, sha256
from cli_exec import streams
from lifecycle_inputs import ENV, NAME, ORDINARY

BOOTSTRAP = """import os,resource,sys
assert os.getuid()==os.geteuid()==os.getgid()==os.getegid()==65534
assert sys.flags.isolated and sys.flags.no_site and sys.flags.dont_write_bytecode
for kind,value in [(resource.RLIMIT_CORE,0),(resource.RLIMIT_FSIZE,268435456),(resource.RLIMIT_NOFILE,512)]:
    assert resource.getrlimit(kind)==(value,value)
print('LIFECYCLE-PYTHON-BOOTSTRAP-1')
"""


def command(selected, mode, step):
    if step in ['audit-before', 'audit-after']:
        return [selected['capture']['compiler']['path'], '--audit-materials']
    if step == 'python-bootstrap':
        return ['/usr/bin/python3', '-I', '-S', '-B', '-c', BOOTSTRAP]
    require(step == 'test', 'only fixed internal steps')
    return ['/lifecycle/test', *mode['args']]


def identity(value, eid, cid, argv, started):
    config = value['ProcessConfig']
    require(set(config) <= {'entrypoint', 'arguments', 'privileged', 'tty', 'user', 'working_dir'}, 'no hidden exec selectors')
    require(value['ID'] == eid and value['ContainerID'] == cid and value['OpenStdin'] is False
            and value['OpenStdout'] is True and value['OpenStderr'] is True
            and config['entrypoint'] == argv[0] and config['arguments'] == argv[1:] and config['privileged'] is False
            and config['tty'] is False and config['user'] == '65534:65534' and config.get('working_dir', '/tmp') == '/tmp'
            and type(value['Running']) is bool and (started or value['Running'] is False), 'same fixed created exec')


def libtest(stdout, mode):
    text = stdout.decode('utf-8', errors='strict'); require(text.endswith('\n\n'), 'complete original libtest footer')
    lines = [line for line in text.splitlines() if line]
    count = 3 if mode['id'] == 'default' else 1
    require(len(lines) == count + 2 and lines[0] == f'running {count} test' + ('s' if count != 1 else ''), 'exact target running rows')
    expected = {name: 'ok' for name in (ORDINARY if mode['id'] == 'default' else [NAME])}
    if mode['id'] == 'default':
        expected[NAME] = 'ignored, requires independently authenticated installed materials and owned native parent envelope'
    seen = {}
    for line in lines[1:-1]:
        match = re.fullmatch(r'test ([A-Za-z0-9_]+) \.\.\. (.+)', line)
        require(match and match[1] not in seen and match[1] in expected and match[2] == expected[match[1]], 'original exact owning test name/status')
        seen[match[1]] = match[2]
    require(seen == expected, 'all original target rows present')
    counts = mode['counts']
    prefix = ('test result: ok. ' + str(counts['passed']) + ' passed; ' + str(counts['failed']) + ' failed; '
              + str(counts['ignored']) + ' ignored; ' + str(counts['measured']) + ' measured; '
              + str(counts['filtered']) + ' filtered out; finished in ')
    require(lines[-1].startswith(prefix) and re.fullmatch(r'[0-9]+(?:\.[0-9]+)?s', lines[-1][len(prefix):]), 'exact original default/positive counts')


def run(engine, cid, selected, mode, step, record):
    argv = command(selected, mode, step)
    request = {'AttachStdin': False, 'AttachStdout': True, 'AttachStderr': True, 'Tty': False,
               'Privileged': False, 'User': '65534:65534', 'WorkingDir': '/tmp', 'Env': ENV, 'Cmd': argv}
    record['request'] = request
    created = engine.json('POST', '/containers/' + cid + '/exec', request)
    eid = created['Id']; require(type(eid) is str and re.fullmatch('[a-f0-9]{64}', eid), 'full owned exec ID')
    record['execId'] = eid
    record['initial'] = engine.json('GET', '/exec/' + eid + '/json'); identity(record['initial'], eid, cid, argv, False)
    raw, _ = engine.request('POST', '/exec/' + eid + '/start', value={'Detach': False, 'Tty': False}, statuses=(200,), maximum_response=139264)
    record.update(wireBase64=base64.b64encode(raw).decode(), wireBytes=len(raw), wireSha256=sha256(raw))
    stdout, stderr = streams(raw)
    record['capture'] = {name: {'base64': base64.b64encode(value).decode(), 'bytes': len(value), 'sha256': sha256(value)} for name, value in [('stdout', stdout), ('stderr', stderr)]}
    record['final'] = engine.json('GET', '/exec/' + eid + '/json'); final = record['final']
    identity(final, eid, cid, argv, True)
    require(final['Running'] is False and type(final['ExitCode']) is int and final['ExitCode'] == 0 and not stderr, 'actual completed successful fixed exec and empty stderr')
    if step in ['audit-before', 'audit-after']:
        material = selected['materials']
        expected = {'version': 1, 'rootSha256': material['manifestSha256'], 'files': material['files'], 'bytes': material['bytes']}
        require(stdout == (json.dumps(expected, separators=(',', ':')) + '\n').encode(), 'complete actual material inventory/root audit')
    elif step == 'python-bootstrap':
        require(stdout == b'LIFECYCLE-PYTHON-BOOTSTRAP-1\n', 'actual isolated os/resource/sys bootstrap with inherited limits')
    else:
        libtest(stdout, mode)
