"""Consume one independent maintainer approval through an immutable protected tag.

This host-only helper never writes a tag or manufactures an approval. A separate
maintainer reviews actual preparation bytes before creating the selected tag.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys
import time
import urllib.error
import urllib.request


def require(value, reason):
    if not value:
        raise ValueError('M6-REMOTE-APPROVAL: ' + reason)


def exact(value, keys):
    require(type(value) is dict and set(value) == set(keys), 'closed record')


def digest(value, length=64):
    require(type(value) is str and re.fullmatch('[a-f0-9]{' + str(length) + '}', value), 'digest syntax')


def decode(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False) + '\n').encode()


def relative(root, name):
    require(type(name) is str and 0 < len(name) <= 512 and not name.startswith('/'), 'relative path')
    require(all(re.fullmatch('[A-Za-z0-9_.+-]+', part) and part not in ('.', '..')
                for part in name.split('/')), 'canonical relative path')
    path = root.joinpath(*name.split('/'))
    require(path.resolve() == path, 'no linked path')
    for parent in [root, *path.parents]:
        info = parent.lstat()
        require(stat.S_ISDIR(info.st_mode) and info.st_uid == 0 and not info.st_mode & 0o022,
                'trusted private directory and ancestry')
    return path


def ordinary(path, maximum, minimum=1):
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_uid == 0 and before.st_nlink == 1
            and not before.st_mode & 0o022 and minimum <= before.st_size <= maximum, 'ordinary bounded root-owned input')
    with path.open('rb') as stream:
        raw = stream.read(maximum + 1)
    after = path.lstat()
    require(all(getattr(before, key) == getattr(after, key) for key in
                ('st_dev', 'st_ino', 'st_uid', 'st_gid', 'st_mode', 'st_nlink', 'st_size', 'st_mtime_ns', 'st_ctime_ns'))
            and len(raw) == before.st_size, 'stable whole input')
    return raw


def pin(raw):
    return {'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}


def create_only(path, raw):
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o400)
    with os.fdopen(descriptor, 'wb') as stream:
        os.fchmod(stream.fileno(), 0o400)
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())
    require(stat.S_IMODE(path.lstat().st_mode) == 0o400 and ordinary(path, len(raw)) == raw,
            'complete immutable-mode output readback')


def select(manifest):
    value = manifest['approval']
    exact(value, ['phase', 'source', 'operatorCommit', 'run', 'root', 'prepared', 'approved'])
    require(value['phase'] == 'public', 'closed phase')
    exact(value['source'], ['commit', 'tree'])
    for item in [value['source']['commit'], value['source']['tree'], value['operatorCommit']]:
        digest(item, 40)
    exact(value['run'], ['id', 'attempt'])
    require(all(type(v) is int and 0 < v < 10**15 for v in value['run'].values()), 'actual run identity')
    exact(manifest['source'], ['path', 'commit', 'tree'])
    require(manifest['source']['path'] == '/workspace/zryna' and
            value['source'] == {key: manifest['source'][key] for key in ['commit', 'tree']}, 'same top-level selected source')
    exact(manifest['run'], ['id', 'attempt'])
    require(all(type(item) is str and re.fullmatch('[1-9][0-9]{0,14}', item)
                for item in manifest['run'].values()), 'top-level actual run selectors')
    require(value['run'] == {key: int(manifest['run'][key]) for key in ['id', 'attempt']}, 'same top-level selected run')
    require(os.environ.get('GITHUB_RUN_ID') == manifest['run']['id'] and
            os.environ.get('GITHUB_RUN_ATTEMPT') == manifest['run']['attempt'] and
            os.environ.get('GITHUB_SHA') == value['operatorCommit'], 'same actual GitHub job and operator commit')
    root = Path(value['root'])
    require(root.is_absolute() and root.resolve() == root and str(root).startswith('/workspace/scratch/'),
            'private task composition root')
    relative(root, value['prepared'])
    require(value['approved'] == [
        {'candidate': 'approval.public.candidate.json', 'output': 'approval.json'},
        {'candidate': 'ui-approval.candidate.json', 'output': 'ui-approval.json'}], 'fixed two approval subjects')
    return value, root


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, message, headers, url):
        raise ValueError('M6-REMOTE-APPROVAL: redirect refused')


def api(path, deadline, absent=False):
    require(path.startswith('/repos/zryna/zryna/') and time.monotonic() < deadline, 'fixed repository and deadline')
    headers = {'Accept': 'application/vnd.github+json', 'User-Agent': 'zryna-m6-independent-runtime-approval',
               'X-GitHub-Api-Version': '2022-11-28'}
    token = os.environ.get('GITHUB_TOKEN')
    if token:
        headers['Authorization'] = 'Bearer ' + token
    request = urllib.request.Request('https://api.github.com' + path, headers=headers)
    try:
        with urllib.request.build_opener(NoRedirect).open(request, timeout=min(15, deadline - time.monotonic())) as response:
            raw = response.read(1024**2 + 1)
            require(response.status == 200 and 0 < len(raw) <= 1024**2
                    and time.monotonic() < deadline, 'bounded successful GitHub response')
    except urllib.error.HTTPError as error:
        if absent and error.code == 404:
            return None
        raise ValueError('M6-REMOTE-APPROVAL: GitHub HTTP ' + str(error.code)) from None
    return decode(raw)


def protected(rule):
    require(rule.get('id') == 22824582 and rule.get('name') == 'immutable-release-tags'
            and rule.get('target') == 'tag' and rule.get('enforcement') == 'active', 'active immutable tag ruleset')
    refs = rule.get('conditions', {}).get('ref_name', {})
    require('refs/tags/v*' in refs.get('include', []) and refs.get('exclude') == [], 'complete approval-tag coverage')
    require({'creation', 'update', 'deletion'} <= {row.get('type') for row in rule.get('rules', [])},
            'tag creation/update/deletion restricted')


def inspect_approval(value, root, tag, tag_ref, tag_object, main, commit):
    require(tag_ref.get('ref') == 'refs/tags/' + tag and tag_ref.get('object', {}).get('type') == 'tag',
            'actual annotated approval tag')
    digest(tag_ref['object']['sha'], 40)
    require(tag_object.get('sha') == tag_ref['object']['sha'] and tag_object.get('tag') == tag
            and tag_object.get('object') == {'type': 'commit', 'sha': value['operatorCommit'],
                                            'url': 'https://api.github.com/repos/zryna/zryna/git/commits/' + value['operatorCommit']},
            'approval targets exact reviewed operator')
    require(main.get('name') == 'main' and main.get('protected') is True
            and main.get('commit', {}).get('sha') == value['source']['commit']
            and commit.get('sha') == value['source']['commit']
            and commit.get('tree', {}).get('sha') == value['source']['tree'], 'same actual protected source')
    message = tag_object.get('message')
    require(type(message) is str and 0 < len(message.encode()) <= 262144, 'bounded approval message')
    approved = decode(message)
    exact(approved, ['format', 'version', 'phase', 'source', 'operatorCommit', 'run',
                     'prepared', 'approvals', 'independentReviewSha256'])
    require(canonical(approved) == message.encode() and approved['format'] == 'zryna.m6-remote-runtime-approval.v1'
            and type(approved['version']) is int and approved['version'] == 1, 'canonical approval domain')
    for key in ['phase', 'source', 'operatorCommit', 'run']:
        require(approved[key] == value[key], 'same approval context: ' + key)
    digest(approved['independentReviewSha256'])
    prepared_raw = ordinary(relative(root, value['prepared']), 2 * 1024**2)
    require(approved['prepared'] == pin(prepared_raw), 'independently selected actual preparation bytes')
    prepared = decode(prepared_raw)
    exact(prepared, ['format', 'version', 'phase', 'source', 'operatorCommit', 'run', 'files'])
    require(prepared['format'] == 'zryna.m6-remote-preparation.v1'
            and type(prepared['version']) is int and prepared['version'] == 1, 'actual preparation inventory domain')
    for key in ['phase', 'source', 'operatorCommit', 'run']:
        require(prepared[key] == value[key], 'same prepared context: ' + key)
    require(type(prepared['files']) is list and 2 <= len(prepared['files']) <= 2048, 'finite complete preparation inventory')
    names, inventory, total = set(), {}, 0
    for row in prepared['files']:
        exact(row, ['path', 'bytes', 'sha256'])
        require(type(row['bytes']) is int and 0 <= row['bytes'] <= 128 * 1024**2, 'bounded ordinary file bytes')
        digest(row['sha256'])
        require(row['path'] not in names and row['path'] != value['prepared'], 'unique nonselffreferential subject')
        raw = ordinary(relative(root, row['path']), 128 * 1024**2, minimum=0)
        require(pin(raw) == {'bytes': row['bytes'], 'sha256': row['sha256']}, 'whole prepared file identity')
        total += len(raw)
        require(total <= 512 * 1024**2, 'total preparation byte bound')
        names.add(row['path'])
        inventory[row['path']] = raw
    require(type(approved['approvals']) is list and len(approved['approvals']) == 2, 'exact two approvals')
    copies = []
    for expected, actual in zip(value['approved'], approved['approvals']):
        exact(actual, ['candidate', 'output', 'bytes', 'sha256'])
        require(all(actual[key] == expected[key] for key in ['candidate', 'output']), 'selected fixed approval subjects')
        require(expected['candidate'] in inventory, 'candidate in approved complete inventory')
        raw = inventory[expected['candidate']]
        require(len(raw) <= 262144 and pin(raw) == {'bytes': actual['bytes'], 'sha256': actual['sha256']}, 'whole approval bytes')
        output = relative(root, expected['output'])
        require(not output.exists(), 'create-only approval output')
        copies.append((output, raw))
    return copies, approved


def main():
    require(len(sys.argv) == 2 and os.geteuid() == 0, 'fixed root-owned host invocation')
    manifest = decode(ordinary(Path(sys.argv[1]), 1024**2))
    value, root = select(manifest)
    deadline = time.monotonic() + 600
    tag = 'v-m6-410-' + value['phase'] + '-approval-' + str(value['run']['id']) + '-' + str(value['run']['attempt'])
    while True:
        tag_ref = api('/repos/zryna/zryna/git/ref/tags/' + tag, deadline, absent=True)
        if tag_ref is not None:
            break
        require(time.monotonic() + 10 < deadline, 'independent approval did not arrive before deadline')
        time.sleep(10)
    rule = api('/repos/zryna/zryna/rulesets/22824582', deadline)
    protected(rule)
    tag_object = api('/repos/zryna/zryna/git/tags/' + tag_ref['object']['sha'], deadline)
    main_branch = api('/repos/zryna/zryna/branches/main', deadline)
    commit = api('/repos/zryna/zryna/git/commits/' + value['source']['commit'], deadline)
    copies, approved = inspect_approval(value, root, tag, tag_ref, tag_object, main_branch, commit)
    for name, data in [('APPROVAL_RULES.json', rule), ('APPROVAL_TAG_REF.json', tag_ref),
                       ('APPROVAL_TAG_OBJECT.json', tag_object), ('APPROVAL_SOURCE.json', main_branch)]:
        create_only(root / name, canonical(data))
    for output, raw in copies:
        create_only(output, raw)
    receipt = {'format': 'zryna.m6-remote-approval-consumption.v1', 'version': 1, 'tag': tag,
               'tagObject': tag_object['sha'], 'prepared': approved['prepared'],
               'independentReviewSha256': approved['independentReviewSha256'],
               'guestExecution': False, 'scope': 'Consumed an actual separately approved protected tag; runtime admission remains mandatory.'}
    create_only(root / 'APPROVAL_ACCEPTANCE.json', canonical(receipt))


if __name__ == '__main__':
    main()
