"""Explicit operator-only full360 import; reserved result/tag, no runtime launch or approval."""
import json
import os
import re
import secrets
import stat
import time
from urllib.parse import urlencode
from cli_common import require
from cli_engine import Engine, decode
from cli_image_plan import CONFIG, path
from cli_image_sources import Sources, directory_state
from cli_image_emit import selected, prepare, emit_selected
from cli_identity import image as validate_image


class ImageEngine(Engine):
    """Separate finite180s operator preparation budget; runtime Owner rejects this deadline."""
    def __init__(self):
        super().__init__(65534, seconds=10)
        self.deadline = time.monotonic() + 180


def transport_limit(document):
    return document['bytes'] + (360 + len(document['directories']) + 1) * 1024 + 20480


def import_image(plan_path, plan_bytes, plan_sha256, record_path, engine=None):
    path(record_path, absolute=True)
    sources = Sources()
    record_fd = cache_fd = None
    cache_identity = None
    token = secrets.token_hex(16)
    tag = 'zryna410-candidate/cli-parity:' + token
    record = {'format': 'zryna.playground-cli-image-import-observation.v1', 'version': 1,
        'status': 'unapproved-operator-image', 'tag': tag, 'image': None, 'cache': None,
        'planBytes': plan_bytes, 'planSha256': plan_sha256, 'emission': None, 'failure': None,
        'mutationAttempted': False, 'runtimeApproved': False, 'cliOrProviderExecuted': False}
    phase = 'reserve-record'
    try:
        parent = sources.parent(record_path)
        record_fd = os.open(record_path.rsplit('/', 1)[1],
            os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o400, dir_fd=parent)
        os.fchmod(record_fd, 0o400)
        phase = 'selected-plan'
        raw, document = selected(sources, plan_path, plan_bytes, plan_sha256)
        selected_engine = engine if engine is not None else ImageEngine()
        require(selected_engine.engine_uid == 65534 and time.monotonic() < selected_engine.deadline <=
                time.monotonic() + 180, 'fixed peer and finite operator image deadline')
        # No filesystem materialization or daemon mutation before exact plan selection.
        phase = 'create-inert-cache'
        cache_leaf = 'zryna410-cli-image-cache-' + token
        os.mkdir(cache_leaf, 0o700, dir_fd=parent)
        cache = record_path.rsplit('/', 1)[0] + '/' + cache_leaf
        cache_fd = os.open(cache_leaf, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=parent)
        cache_identity = directory_state(os.fstat(cache_fd))
        require(stat.S_ISDIR(os.fstat(cache_fd).st_mode) and os.fstat(cache_fd).st_uid == os.geteuid() and
                os.fstat(cache_fd).st_mode & 0o7777 == 0o700, 'private creation-owned inert cache')
        record['cache'] = cache
        phase = 'full349-and-runtime-preflight'
        prepared_raw, document, captured = prepare(sources, plan_path, plan_bytes, plan_sha256,
            cache, cache_fd, selected_engine.deadline)
        require(prepared_raw == raw, 'same selected plan through complete preflight')
        phase = 'create-only-tag-absence'
        query = urlencode({'all': '1', 'filters': json.dumps({'reference': [tag]}, separators=(',', ':'))})
        require(selected_engine.json('GET', '/images/json?' + query) == [], 'new random image tag must be absent')
        changes = ['USER 65534:65534', 'ENTRYPOINT ["/supervisor"]', 'CMD ["idle"]', 'WORKDIR /work/project']
        changes += ['ENV ' + entry for entry in CONFIG['Env']]
        repository, suffix = tag.split(':')
        query = urlencode([('fromSrc', '-'), ('repo', repository), ('tag', suffix), ('platform', 'linux/amd64'),
                           *(('changes', value) for value in changes)])
        phase, record['mutationAttempted'] = 'import', True
        body, emission = selected_engine.request('POST', '/images/create?' + query,
            emitter=lambda output: emit_selected(sources, raw, document, captured, plan_path,
                plan_bytes, plan_sha256, output, selected_engine.deadline),
            stream_maximum=transport_limit(document), statuses=(200,))
        record['emission'] = emission
        events = [decode(line) for line in body.splitlines() if line]
        require(0 < len(events) <= 512 and all(type(value) is dict for value in events) and
                all('error' not in value and 'errorDetail' not in value for value in events), 'finite successful image events')
        ids = [value['status'] for value in events if re.fullmatch('sha256:[a-f0-9]{64}', str(value.get('status')))]
        require(len(ids) == 1, 'one actual imported image ID')
        record['image'] = ids[0]
        phase = 'actual-image-readback'
        state = selected_engine.json('GET', '/images/' + ids[0] + '/json')
        validate_image(state, {'image': ids[0], 'layer_sha256': emission['archiveSha256']})
        require(tag in state.get('RepoTags', []), 'actual new tag names the imported image')
        record['actualImage'] = state
        sources.verify(document['archive']); sources.revalidate_files()
        require(sources.read_plan(plan_path, plan_bytes, plan_sha256) == raw, 'final selected plan identity')
        require(directory_state(os.fstat(cache_fd)) == cache_identity and
                directory_state(os.stat(cache_leaf, dir_fd=parent, follow_symlinks=False)) == cache_identity,
                'original private cache still named')
        record['sourceIdentitiesBeforeAfterEqual'] = True
        record['sourceIdentities'] = {name: list(value) for name, value in sources.files.items()}
        require(time.monotonic() < selected_engine.deadline, 'whole image operation deadline')
    except BaseException as error:
        record['failure'] = {'phase': phase, 'type': type(error).__name__[:64]}
    finally:
        try:
            if cache_fd is not None:
                os.close(cache_fd)
            sources.close()
        except BaseException as error:
            record['failure'] = record['failure'] or {'phase': 'source-close', 'type': type(error).__name__[:64]}
        if record_fd is not None:
            try:
                data = (json.dumps(record, separators=(',', ':'), ensure_ascii=True) + '\n').encode()
                require(len(data) <= 262144, 'bounded create-only image result')
                while data:
                    n = os.write(record_fd, data)
                    require(n > 0, 'complete reserved image result')
                    data = data[n:]
            finally:
                os.close(record_fd)
    return record
