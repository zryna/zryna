"""Creation-owned pair. Cleanup uncertainty permanently stops this invocation."""
import re
import secrets
from cli_common import require
from lifecycle_inputs import FLAGS, validate, configuration, container_identity
from lifecycle_backend import Backend


def error_record(error):
    return {'type': type(error).__name__, 'message': str(error)[:4096], 'errno': getattr(error, 'errno', None)}


def never_started(state):
    value = state['State']
    return (value['Status'] == 'created' and value['Running'] is False and value['Pid'] == 0
            and value['StartedAt'] == value['FinishedAt'] == '0001-01-01T00:00:00Z')


def same_init(first, after):
    require(all(first[key] == after[key] for key in ['pid', 'ticks', 'cid', 'startedAt', 'membership', 'cgroupPath']),
            'original private init birth/membership throughout fixed steps')


def cleanup(backend, kernel, selected, phase, expected, init, quiescent):
    end = backend.clock() + 2
    record = {'confirmed': False, 'deadline': end, 'requests': []}
    phase['cleanup'] = record
    if not phase['createAttempted']:
        record.update(confirmed=True, disposition='no-create-attempt')
        return True
    cid = phase.get('cid')
    try:
        engine = backend.engine(end, record['requests'])
        state = backend.lookup(engine, cid or phase['name'])
        # An unanswered create followed by name404 is not proof the create cannot complete later.
        require(state is not None, 'creation outcome cannot be inferred from name/CID absence')
        cid = state['Id']; require(re.fullmatch('[a-f0-9]{64}', cid), 'full actual cleanup object ID')
        container_identity(state, cid, phase['name'], expected, selected)
        if phase.get('cid') is not None:
            require(cid == phase['cid'], 'no replacement object during cleanup')
        phase['cid'] = cid
        unused = never_started(state)
        capture_error = None
        if state['State']['Running'] and not quiescent:
            try:
                after = backend.observe(engine, kernel, cid,
                    lambda value: container_identity(value, cid, phase['name'], expected, selected), end)
                if init is not None:
                    same_init(init, after)
                init, quiescent = after, True
                record['quiescence'] = after
            except Exception as error:
                capture_error = error_record(error)
                record['quiescenceUnavailable'] = capture_error
        if state['State']['Running']:
            engine.request('POST', '/containers/' + cid + '/kill?signal=KILL', statuses=(204,))
        if not unused:
            record['wait'] = engine.json('POST', '/containers/' + cid + '/wait?condition=not-running')
            require(type(record['wait']['StatusCode']) is int and record['wait'].get('Error') in [None, {'Message': ''}], 'actual completed owned wait')
        stopped = backend.lookup(engine, cid)
        require(stopped is not None, 'owned stopped object retained before removal')
        container_identity(stopped, cid, phase['name'], expected, selected)
        require(stopped['State']['Running'] is False and stopped['State']['Pid'] == 0, 'actual owned parent stopped')
        if not unused:
            require(stopped['State']['Status'] == 'exited' and stopped['State']['ExitCode'] == record['wait']['StatusCode'], 'wait/inspect same actual exit')
        engine.request('DELETE', '/containers/' + cid + '?force=0&v=0', statuses=(204,))
        require(backend.lookup(engine, cid) is None, 'actual nonforce removal followed by404')
        record['removed'] = True
        if not unused:
            require(init is not None and quiescent and capture_error is None, 'unknown child/zombie cleanup cannot become success')
            record['kernel'] = kernel.gone(init, end)
        backend.audit(engine, record.setdefault('after', {}))
        require(backend.clock() < end, 'cleanup finished within shared2s')
        record.update(confirmed=True, disposition='never-started-removed' if unused else 'reaped-killed-waited-removed')
    except Exception as error:
        record['failure'] = error_record(error)
    return record['confirmed']


def run_mode(backend, selected, mode):
    end = backend.clock() + 10
    nonce = secrets.token_hex(16)
    phase = {'mode': mode['id'], 'name': 'zryna418-lifecycle-' + nonce, 'deadline': end,
             'createAttempted': False, 'requests': [], 'kernel': [], 'steps': [], 'completed': False}
    engine = backend.engine(end, phase['requests'])
    kernel = backend.kernel(phase['kernel'])
    expected = configuration(selected, nonce, backend.seccomp)
    phase['configuration'] = expected
    init, quiescent = None, False
    def inspect(state):
        container_identity(state, phase['cid'], phase['name'], expected, selected)
    try:
        backend.audit(engine, phase.setdefault('before', {}))
        require(backend.clock() < end, 'precreation observations share10s')
        phase['createAttempted'] = True
        created = engine.json('POST', '/containers/create?name=' + phase['name'], expected)
        cid = created['Id']; require(type(cid) is str and re.fullmatch('[a-f0-9]{64}', cid), 'full creation-owned container ID')
        require(created.get('Warnings') in [None, []], 'no container creation warnings')
        phase['cid'] = cid
        initial = engine.json('GET', '/containers/' + cid + '/json'); inspect(initial)
        phase['initial'] = initial
        require(never_started(initial), 'fresh private parent never started')
        engine.request('POST', '/containers/' + cid + '/start', statuses=(204,))
        init = backend.observe(engine, kernel, cid, inspect, end)
        phase['originalInit'] = init; quiescent = True
        # All apps, including audit/bootstrap, follow complete actual kernel admission.
        for step in ['audit-before', 'python-bootstrap', 'test', 'audit-after']:
            before = backend.observe(engine, kernel, cid, inspect, end)
            same_init(init, before)
            item = {'step': step, 'before': before}; phase['steps'].append(item)
            quiescent = False
            backend.execute(engine, cid, mode, step, item)
            after = backend.observe(engine, kernel, cid, inspect, end)
            item['after'] = after; same_init(init, after); quiescent = True
        backend.audit(engine, phase.setdefault('after', {}))
        # The final independent audit must not hide a subsequent task/init change.
        final = backend.observe(engine, kernel, cid, inspect, end); same_init(init, final)
        phase['final'] = final
        require(backend.clock() < end, 'all fixed steps within original10s')
        phase['completed'] = True
    except Exception as error:
        phase['failure'] = error_record(error)
    finally:
        cleanup(backend, kernel, selected, phase, expected, init, quiescent)
    return phase


def run_pair(selected, _backend=None):
    selected = validate(selected)
    backend = Backend(selected) if _backend is None else _backend
    require(_backend is None or backend.synthetic is True, 'injected tests are explicitly unqualified')
    result = {'format': 'zryna.private-lifecycle-pair.v1', **FLAGS,
              'observationMode': 'synthetic-adapter' if _backend is not None else 'private-native-observation',
              'selection': {'planSha256': selected['planSha256'], 'image': selected['image'],
                            'capture': selected['capture'], 'materials': selected['materials'],
                            'nativeVmSha256': selected['nativeVm']['sha256']}, 'phases': [], 'completed': False}
    for mode in selected['plan']['modes']:
        phase = run_mode(backend, selected, mode); result['phases'].append(phase)
        if not phase['completed'] or not phase['cleanup']['confirmed']:
            result['status'] = 'refused-or-cleanup-unknown'
            return result
    result.update(status='captured-lifecycle-pair', completed=True)
    return result
