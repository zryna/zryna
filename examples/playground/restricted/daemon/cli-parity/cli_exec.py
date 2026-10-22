"""One fixed declared CLI operation; closed EngineExec selectors and bounded captured streams."""
import base64
import re
import struct
from cli_common import ENV, require, sha256
from cli_plan import operation


def exec_identity(value, eid, cid, selected, started):
    config = value['ProcessConfig']
    require(set(config) <= {'entrypoint', 'arguments', 'privileged', 'tty', 'user', 'working_dir'} and
            config.get('working_dir', '/work/project') == '/work/project', 'no hidden exec selectors')
    require(value['ID'] == eid and value['ContainerID'] == cid and value['OpenStdin'] is False and
            value['OpenStdout'] is True and value['OpenStderr'] is True and
            config['entrypoint'] == selected['executable'] and config['arguments'] == selected['argv'] and
            config['privileged'] is False and config['tty'] is False and config['user'] == '65534:65534' and
            type(value['Running']) is bool and (started or value['Running'] is False), 'fixed created CLI exec')


def streams(raw):
    require(type(raw) is bytes and 0 < len(raw) <= 139264, 'finite attached CLI stream')
    offset, count, output = 0, 0, [bytearray(), bytearray()]
    while offset < len(raw):
        require(offset + 8 <= len(raw), 'complete CLI frame header')
        channel, a, b, c, size = struct.unpack('>BBBBI', raw[offset:offset + 8])
        count += 1
        require(channel in [1, 2] and a == b == c == 0 and 0 < size <= 65536 and count <= 1024
                and offset + 8 + size <= len(raw), 'closed bounded Docker CLI multiplex frame')
        destination = output[channel - 1]
        require(len(destination) + size <= 65536, 'CLI stdout/stderr64KiB each before retention')
        destination.extend(raw[offset + 8:offset + 8 + size])
        offset += 8 + size
    return bytes(output[0]), bytes(output[1])


def run(engine, cid, identifier, record):
    selected = operation(identifier)
    request = {'AttachStdin': False, 'AttachStdout': True, 'AttachStderr': True, 'Tty': False,
               'Privileged': False, 'User': '65534:65534', 'WorkingDir': '/work/project', 'Env': ENV,
               'Cmd': [selected['executable'], *selected['argv']]}
    created = engine.json('POST', '/containers/' + cid + '/exec', request)
    eid = created['Id']
    require(type(eid) is str and re.fullmatch('[a-f0-9]{64}', eid), 'actual created CLI exec ID')
    record['execId'], record['request'] = eid, request
    initial = engine.json('GET', '/exec/' + eid + '/json')
    exec_identity(initial, eid, cid, selected, False)
    record['initial'] = initial
    raw, _ = engine.request('POST', '/exec/' + eid + '/start', value={'Detach': False, 'Tty': False},
                            statuses=(200,), maximum_response=139264)
    record['wireBytes'], record['wireSha256'] = len(raw), sha256(raw)
    stdout, stderr = streams(raw)
    final = engine.json('GET', '/exec/' + eid + '/json')
    record['final'] = final
    exec_identity(final, eid, cid, selected, True)
    require(final['Running'] is False and type(final['ExitCode']) is int, 'actual completed CLI exec exit')
    record['capture'] = {'exitCode': final['ExitCode'], 'stdoutBase64': base64.b64encode(stdout).decode(),
                         'stderrBase64': base64.b64encode(stderr).decode(), 'stdoutBytes': len(stdout),
                         'stderrBytes': len(stderr), 'stdoutSha256': sha256(stdout), 'stderrSha256': sha256(stderr)}
    require(not stderr and final['ExitCode'] in [0, 3], 'CLI source outcome only, no setup/runtime failure')
    return {'exitCode': final['ExitCode'], 'stdout': stdout, 'stderr': stderr}
