"""Explicit operator-only trusted UI image import; never admits a runtime or starts a process."""
import re
import secrets
from urllib.parse import urlencode
from archive import emit
from engine import decode
from handles import Sources
from install import transport_limit
from plan import MAX_PLAN, decode as decode_plan, digest, integer, path, require
from ui_plan import CONFIG, validate


def run(plan_path, plan_bytes, plan_sha256, output):
    path(plan_path, absolute=True)
    integer(plan_bytes, MAX_PLAN, 1)
    digest(plan_sha256)
    sources = Sources()
    try:
        raw = sources.read_plan(plan_path, plan_bytes, plan_sha256)
        document = validate(decode_plan(raw))
        receipt = emit(document, sources, output)
        require(sources.read_plan(plan_path, plan_bytes, plan_sha256) == raw, 'UI plan changed')
        receipt.update({'format': 'zryna.playground-ui-archive-emission.v1', 'version': 1,
                        'planBytes': plan_bytes, 'planSha256': plan_sha256,
                        'sourceInventoryBeforeAfterEqual': True,
                        'scope': 'trusted qualification root transport only; no runtime approval'})
    finally:
        sources.close()
    return receipt


def image(state, identity, layer):
    require(state['Id'] == identity and state['Os'] == 'linux' and state['Architecture'] == 'amd64'
            and state.get('Variant', '') == '' and state.get('OsVersion', '') == '', 'UI image target')
    require(state['RootFS'] == {'Type': 'layers', 'Layers': ['sha256:' + layer]}, 'exact UI image layer')
    config = state['Config']
    require(type(config) is dict and set(CONFIG) <= set(config), 'complete UI image config record')
    for key, value in CONFIG.items():
        require(config.get(key) == value if key != 'Cmd' else config.get(key) in [None, []],
                'fixed UI image configuration')
    for key, value in config.items():
        if key in CONFIG:
            continue
        if key in ['Hostname', 'Domainname', 'MacAddress', 'StopSignal']:
            allowed = value == ''
        elif key in ['AttachStdin', 'AttachStdout', 'AttachStderr', 'Tty', 'OpenStdin',
                     'StdinOnce', 'ArgsEscaped', 'NetworkDisabled']:
            allowed = value is False
        elif key in ['Volumes', 'ExposedPorts', 'Healthcheck', 'Labels']:
            allowed = value is None or type(value) is dict and not value
        elif key in ['OnBuild', 'Shell']:
            allowed = value is None or type(value) is list and not value
        elif key == 'StopTimeout':
            allowed = value is None
        elif key == 'Image':
            allowed = type(value) is str and (value == '' or re.fullmatch('sha256:[a-f0-9]{64}', value))
        else:
            allowed = False
        require(allowed, 'unknown or inherited UI image authority')


def import_image(engine, plan_path, plan_bytes, plan_sha256):
    path(plan_path, absolute=True)
    integer(plan_bytes, MAX_PLAN, 1)
    digest(plan_sha256)
    sources = Sources()
    try:
        document = validate(decode_plan(sources.read_plan(plan_path, plan_bytes, plan_sha256)))
        for item in document['files']:
            sources.verify(item)
        sources.revalidate_files()
    finally:
        sources.close()
    tag = 'zryna410-candidate/trusted-ui:' + secrets.token_hex(16)
    repository, suffix = tag.split(':')
    changes = ['USER 65534:65534', 'ENTRYPOINT ["/ui-supervisor"]', 'CMD []', 'WORKDIR /tmp']
    changes.extend('ENV ' + entry for entry in CONFIG['Env'])
    query = urlencode([('fromSrc', '-'), ('repo', repository), ('tag', suffix),
                       ('platform', 'linux/amd64'), *(('changes', change) for change in changes)])
    record = {'status': 'unapproved-trusted-ui-image', 'tag': tag, 'image': None,
              'planBytes': plan_bytes, 'planSha256': plan_sha256, 'emission': None, 'failure': None}
    try:
        raw, emission = engine.request('POST', '/images/create?' + query,
            emitter=lambda output: run(plan_path, plan_bytes, plan_sha256, output),
            stream_maximum=transport_limit(document), statuses=(200,))
        record['emission'] = emission
        events = [decode(line) for line in raw.splitlines() if line]
        require(0 < len(events) <= 512 and all(type(event) is dict for event in events), 'finite UI import events')
        require(all('error' not in event and 'errorDetail' not in event for event in events), 'UI import failed')
        ids = [event.get('status') for event in events if re.fullmatch(r'sha256:[a-f0-9]{64}', str(event.get('status')))]
        require(len(ids) == 1, 'one actual imported UI image identity')
        record['image'] = ids[0]
        state = engine.json('GET', '/images/' + ids[0] + '/json')
        image(state, ids[0], emission['archiveSha256'])
        record.update({'rootFs': state['RootFS'], 'config': state['Config']})
    except BaseException as error:
        record['failure'] = type(error).__name__ + ': ' + str(error)[:1024]
    return record
