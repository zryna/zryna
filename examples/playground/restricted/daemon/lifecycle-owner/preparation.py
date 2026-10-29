"""Private operator transport only: no guest starts, runtime approval or source execution."""
import copy
import re
import secrets
from urllib.parse import urlencode
from preparation_primitives import Sources, emit, load, transport_limit
from preparation_plan import CONFIG, FLAGS, require, validate


def checked(selection, sources):
    image, materials, manifest = validate(selection)
    for item in selection['provenance'].values():
        sources.verify({'sourcePath': item['path'], 'bytes': item['bytes'], 'sha256': item['sha256']})
    for item in image['files'] + materials['files']:
        sources.verify(item)
    # A declared prefix is checked against the exact pinned file, not merely copied to a report.
    actual = {row['path']: row for row in image['files']}
    actual.update({'materials/' + row['path']: row for row in materials['files']})
    for row in selection['runtime']['elf']:
        item = actual[row['path'].lstrip('/')]
        fd, before = sources.open(item['sourcePath'])
        try:
            import os
            require(before[6] == 1, 'ordinary source must have one link')
            prefix = bytes.fromhex(row['prefixHex'])
            require(os.read(fd, len(prefix)) == prefix, 'actual pinned ELF prefix differs')
            sources.finish(item['sourcePath'], fd, before)
        finally:
            os.close(fd)
    sources.revalidate_files()
    return image, materials, manifest


def image_state(state, identity, layer):
    require(state['RootFS'] == {'Type': 'layers', 'Layers': ['sha256:' + layer]}, 'exact emitted complete layer')
    config = state['Config']
    require(type(config) is dict and all(config.get(key) == value for key, value in CONFIG.items()), 'closed lifecycle image config')
    projected = copy.deepcopy(state)
    projected['Config'].update(load('plan').CONFIG)
    # Lifecycle fields were checked above; retain the unchanged complete unknown/inherited
    # authority check for every remaining actual Config field, plus image target identity.
    load('install').validate_runtime_image(projected, identity)


def imported(engine, selection, sources, document, record):
    tag = 'zryna418-candidate/lifecycle:' + secrets.token_hex(16)
    record['tag'] = tag  # Retain a recovery handle before the first mutating request.
    repository, suffix = tag.split(':')
    changes = ['USER 65534:65534', 'ENTRYPOINT ["/lifecycle/supervisor"]', 'CMD ["idle"]', 'WORKDIR /tmp']
    changes.extend('ENV ' + entry for entry in CONFIG['Env'])
    query = urlencode([('fromSrc', '-'), ('repo', repository), ('tag', suffix),
                       ('platform', 'linux/amd64'), *(('changes', value) for value in changes)])
    def output(stream):
        require(validate(selection)[0] == document, 'selection changed before emission')
        value = emit(document, sources, stream)
        checked(selection, sources)
        return value
    raw, emission = engine.request('POST', '/images/create?' + query, emitter=output,
        stream_maximum=transport_limit(document), statuses=(200,))
    record['emission'] = emission
    events = [load('engine').decode(line) for line in raw.splitlines() if line]
    require(0 < len(events) <= 512 and all(type(event) is dict and 'error' not in event
            and 'errorDetail' not in event for event in events), 'finite successful image import')
    ids = [event.get('status') for event in events if re.fullmatch('sha256:[a-f0-9]{64}', str(event.get('status')))]
    require(len(ids) == 1, 'one actual imported image identity')
    record['image'] = {'id': ids[0], 'layerSha256': emission['archiveSha256']}
    state = engine.json('GET', '/images/' + ids[0] + '/json')
    image_state(state, ids[0], emission['archiveSha256'])
    record['imageReadback'] = state
    return record['image']


def prepare(selection, engine_expected, peer_anchor=None):
    """Use the sibling fixed UID0 Engine; return observations, never a launch capability.

    The operator must run authenticate-materials.mjs freshly and retain its raw verification
    evidence before this transport. Vendor/source review is external; no boolean substitutes.
    """
    import time
    import owner
    from preparation_volume import populate
    selection = copy.deepcopy(selection)
    # Engine creation does not connect. All input/source checks precede the first Engine I/O.
    record = {'format': 'zryna.lifecycle-preparation-observation.v1', 'flags': dict(FLAGS),
              'tag': None, 'image': None, 'materials': None, 'failure': None, 'requests': []}
    sources = Sources()
    try:
        image, materials, manifest = checked(selection, sources)
        engine = owner._load('lifecycle_engine').Engine(engine_expected, time.monotonic() + 180,
                                                       record['requests'], peer_anchor)
        record['engineBefore'] = engine.audit()
        identity = imported(engine, selection, sources, image, record)
        populate(engine, identity, materials, manifest, sources, record)
        checked(selection, sources)
        record['engineAfter'] = engine.audit()
        record['peer'] = list(engine.peer_anchor)
    except BaseException as error:
        record['failure'] = type(error).__name__ + ': ' + str(error)[:1024]
        # Never delete uncertain objects or force-remove a holder: retain exact IDs for recovery.
    finally:
        try:
            sources.close()
        except BaseException as error:
            record['failure'] = (str(record['failure'] or '') + '; source cleanup: ' + type(error).__name__ + ': ' + str(error))[:2048]
    return record
