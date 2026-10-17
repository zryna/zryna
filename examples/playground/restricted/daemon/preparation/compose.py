"""Reusable candidate composition from explicit operator-selected source descriptors."""
import hashlib
import json
from handles import Sources
from plan import CONFIG, MOUNTS, require, validate


def descriptor(source_path, destination, size, sha256, mode):
    """Verify an explicit externally selected source pin, never infer authority from bytes."""
    item = {'path': destination, 'sourcePath': source_path, 'bytes': size,
            'sha256': sha256, 'mode': mode}
    from plan import digest, integer, path
    path(source_path, absolute=True)
    path(destination)
    integer(size, 512 * 1024**2, 1)
    digest(sha256)
    require(type(mode) is int and mode in [292, 365], 'fixed file mode')
    sources = Sources()
    try:
        sources.verify(item)
    finally:
        sources.close()
    return item


def candidate(kind, role, files):
    files = sorted(files, key=lambda item: item['path'])
    directories = set(MOUNTS) if kind == 'runtime' else set()
    for item in files:
        parts = item['path'].split('/')
        directories.update('/'.join(parts[:index]) for index in range(1, len(parts)))
    return validate({'format': 'zryna.playground-preparation-plan.v1', 'version': 1,
                     'kind': kind, 'role': role, 'directories': sorted(directories),
                     'files': files, 'bytes': sum(item['bytes'] for item in files),
                     'config': CONFIG if kind == 'runtime' else None})


def material_manifest(document):
    validate(document)
    require(document['kind'] == 'materials', 'material plan required')
    value = {'format': 'zryna.playground-materials.v1', 'version': 1,
             'role': document['role'], 'directories': document['directories'],
             'files': [{key: item[key] for key in ['path', 'bytes', 'sha256', 'mode']}
                       for item in document['files']]}
    return json.dumps(value, separators=(',', ':'), ensure_ascii=True).encode('ascii')


def verify_sources(document):
    validate(document)
    sources = Sources()
    try:
        for item in document['files']:
            sources.verify(item)
        sources.revalidate()
    finally:
        sources.close()


def encode(document):
    validate(document)
    raw = json.dumps(document, separators=(',', ':'), ensure_ascii=True).encode('ascii')
    return raw, {'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}
