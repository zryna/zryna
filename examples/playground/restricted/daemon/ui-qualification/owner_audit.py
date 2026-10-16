"""One fixed supervisor exec; zero caller-selected executable, arguments or environment."""
import re
import struct
import copy
import weakref
from engine import decode
from plan import ENV, exact, require


_PROC_PROOFS = weakref.WeakKeyDictionary()


class _ProcProof:
    pass


def proc_description(proof, cid):
    require(type(proof) is _ProcProof and proof in _PROC_PROOFS and _PROC_PROOFS[proof][0] == cid,
            'UI actual target-bound proc audit required')
    # Only the fixed Engine producer creates the object; callers receive fresh descriptions.
    return copy.deepcopy(_PROC_PROOFS[proof][1])


def exec_identity(value, eid, cid, started, mode='material'):
    require(mode in ['material', 'proc'], 'UI fixed audit mode enum')
    entrypoint, arguments = ('/supervisor', ['audit']) if mode == 'material' else ('/ui-supervisor', ['proc-audit'])
    config = value['ProcessConfig']
    require(set(config) <= {'entrypoint', 'arguments', 'privileged', 'tty', 'user', 'working_dir'} and
            config.get('working_dir', '/tmp') == '/tmp', 'UI no hidden audit process selectors')
    require(value['ID'] == eid and value['ContainerID'] == cid and value['OpenStdin'] is False and
            value['OpenStdout'] is True and value['OpenStderr'] is True and
            config['entrypoint'] == entrypoint and config['arguments'] == arguments and
            config['privileged'] is False and config['tty'] is False and config['user'] == '65534:65534' and
            type(value['Running']) is bool and (started or value['Running'] is False), 'UI fixed audit exec identity')


def streams(raw):
    require(type(raw) is bytes and 0 < len(raw) <= 4096, 'UI bounded audit response')
    offset, stdout, stderr = 0, bytearray(), bytearray()
    while offset < len(raw):
        require(offset + 8 <= len(raw), 'UI complete audit stream header')
        channel, a, b, c, size = struct.unpack('>BBBBI', raw[offset:offset + 8])
        require(channel in [1, 2] and a == b == c == 0 and 0 < size <= 1024 and
                offset + 8 + size <= len(raw), 'UI closed audit frame')
        (stdout if channel == 1 else stderr).extend(raw[offset + 8:offset + 8 + size])
        offset += 8 + size
    require(not stderr and 0 < len(stdout) <= 1024, 'UI real audit stderr/output')
    return bytes(stdout)


def execute(engine, cid, mode):
    require(mode in ['material', 'proc'], 'UI fixed audit mode enum')
    require(type(cid) is str and re.fullmatch('[a-f0-9]{64}', cid), 'UI actual audit container ID')
    command = ['/supervisor', 'audit'] if mode == 'material' else ['/ui-supervisor', 'proc-audit']
    request = {'AttachStdin': False, 'AttachStdout': True, 'AttachStderr': True, 'Tty': False,
               'Privileged': False, 'User': '65534:65534', 'WorkingDir': '/tmp', 'Env': ENV,
               'Cmd': command}
    created = engine.json('POST', '/containers/' + cid + '/exec', request)
    eid = created['Id']
    require(type(eid) is str and re.fullmatch('[a-f0-9]{64}', eid), 'UI actual audit exec ID')
    initial = engine.json('GET', '/exec/' + eid + '/json')
    exec_identity(initial, eid, cid, False, mode)
    raw, _ = engine.request('POST', '/exec/' + eid + '/start', value={'Detach': False, 'Tty': False},
                            statuses=(200,), maximum_response=4096)
    final = engine.json('GET', '/exec/' + eid + '/json')
    exec_identity(final, eid, cid, True, mode)
    require(final['Running'] is False and type(final['ExitCode']) is int and final['ExitCode'] == 0,
            'UI actual completed audit zero exit')
    receipt = decode(streams(raw))
    return {'execId': eid, 'initial': initial, 'final': final, 'receipt': receipt}


def run(engine, cid, pin):
    record = execute(engine, cid, 'material')
    receipt = record['receipt']
    exact(receipt, ['version', 'rootSha256', 'files', 'bytes'])
    require(receipt == {'version': 1, 'rootSha256': pin['root_sha256'], 'files': pin['files'], 'bytes': pin['bytes']}
            and type(receipt['version']) is int and type(receipt['files']) is int and type(receipt['bytes']) is int,
            'UI full actual419 material audit correspondence')
    return record


def run_proc(engine, cid):
    record = execute(engine, cid, 'proc')
    receipt = record['receipt']
    exact(receipt, ['format', 'version', 'sysrqTrigger'])
    require(receipt['format'] == 'zryna.playground-ui-proc-audit.v1' and
            type(receipt['version']) is int and receipt['version'] == 1 and
            receipt['sysrqTrigger'] in ['absent-enoent', 'present-readonly'], 'UI exact native proc proof receipt')
    proof = _ProcProof()
    _PROC_PROOFS[proof] = (cid, copy.deepcopy(record))
    return proof
