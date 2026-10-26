"""Complete source censuses with unchanged per-file and selected-capture bounds."""
from pathlib import Path
import hashlib
import json
import os
import re

from capture_io import ENTRY_LIMIT, REPORT_LIMIT, SELECTED_LIMIT, parse_json, sha, usage

FORMAT = 'zryna.m5.source-snapshot-shards.v1'
HEADER = {'candidate', 'checkoutCommit', 'tree', 'clean'}
ROW = {'path', 'mode', 'blob', 'bytes', 'sha256'}
ENCODER = json.JSONEncoder(ensure_ascii=False, separators=(',', ':'))


def _integer(value, maximum, minimum=0):
    assert type(value) is int and minimum <= value <= maximum


def _hex(value, length):
    assert isinstance(value, str) and re.fullmatch('[0-9a-f]{' + str(length) + '}', value)


def _name(name):
    assert name in ('SOURCE_BEFORE.json', 'SOURCE_AFTER.json')


def _header(value):
    assert isinstance(value, dict) and set(value) == HEADER
    for key in ('candidate', 'checkoutCommit', 'tree'):
        _hex(value[key], 40)
    assert value['clean'] is True


def _rows(rows):
    assert isinstance(rows, list) and rows
    previous = None
    for row in rows:
        assert isinstance(row, dict) and set(row) == ROW
        path = row['path']
        assert isinstance(path, str) and path and '\0' not in path
        assert not path.startswith('/') and '..' not in Path(path).parts
        key = path.encode('utf-8')
        assert previous is None or previous < key
        previous = key
        assert row['mode'] in ('100644', '100755')
        _hex(row['blob'], 40)
        _integer(row['bytes'], 2 * 1024 * 1024)
        _hex(row['sha256'], 64)


def _validate(value):
    assert isinstance(value, dict) and set(value) == HEADER | {'files'}
    _header({key: value[key] for key in HEADER})
    _rows(value['files'])


def _encoded(value):
    return (ENCODER.encode(value) + '\n').encode('utf-8')


def _fingerprint(value):
    digest, length = hashlib.sha256(), 0
    # Do not allocate the complete unsharded JSON representation.
    for chunk in ENCODER.iterencode(value):
        data = chunk.encode('utf-8')
        length += len(data)
        assert length < SELECTED_LIMIT
        digest.update(data)
    length += 1
    assert length <= SELECTED_LIMIT
    digest.update(b'\n')
    return {'bytes': length, 'sha256': digest.hexdigest(), 'files': len(value['files'])}


def _shard_name(name, index):
    return name[:-5] + '.files-' + format(index, '05d') + '.json'


def _plan(name, value):
    _name(name)
    _validate(value)
    full = _fingerprint(value)
    if full['bytes'] <= REPORT_LIMIT:
        return [(name, _encoded(value))]
    # Plan every bounded shard and the bounded index before the first exclusive write.
    prefix, suffix = b'{"files":[', b']}\n'
    shards, pins, chunks, length, count = [], [], [], len(prefix) + len(suffix), 0

    def retain():
        data = prefix + b','.join(chunks) + suffix
        assert len(data) <= REPORT_LIMIT
        path = _shard_name(name, len(shards))
        shards.append((path, data))
        pins.append({'path': path, 'bytes': len(data), 'sha256': sha(data), 'files': count})
        assert len(shards) < ENTRY_LIMIT

    for row in value['files']:
        data = ENCODER.encode(row).encode('utf-8')
        assert len(prefix) + len(data) + len(suffix) <= REPORT_LIMIT
        extra = len(data) + bool(count)
        if length + extra > REPORT_LIMIT:
            retain()
            chunks, length, count = [], len(prefix) + len(suffix), 0
            extra = len(data)
        chunks.append(data)
        length += extra
        count += 1
    retain()
    index = {'format': FORMAT, 'header': {key: value[key] for key in value if key != 'files'},
             'snapshot': full, 'shards': pins}
    data = _encoded(index)
    assert len(data) <= REPORT_LIMIT
    shards.append((name, data))  # Root index is the final publication, never a partial census.
    assert sum(len(raw) for _, raw in shards) <= SELECTED_LIMIT
    return shards


def _reserve(parent, outputs):
    parent.check()
    baseline = usage(parent.path)
    assert baseline['entries'] + len(outputs) <= ENTRY_LIMIT
    assert baseline['retainedBytes'] + sum(len(data) for _, data in outputs) <= SELECTED_LIMIT
    unit = os.fstatvfs(parent.fd).f_frsize
    assert unit > 0
    allocated = sum((len(data) + unit - 1) // unit * unit for _, data in outputs)
    assert baseline['allocatedBytes'] + allocated <= SELECTED_LIMIT
    for name, _ in outputs:
        try:
            os.stat(name, dir_fd=parent.fd, follow_symlinks=False)
        except FileNotFoundError:
            continue
        raise FileExistsError(name)


def write_snapshot(parent, name, value):
    outputs = _plan(name, value)
    _reserve(parent, outputs)
    for path, data in outputs:
        parent.write(path, data)
        usage(parent.path)  # Actual allocated/retained bounds remain mandatory after writes.
    assert read_snapshot(parent, name) == value
    data = outputs[-1][1]
    return {'path': name, 'bytes': len(data), 'sha256': sha(data)}


def read_snapshot(parent, name):
    _name(name)
    raw = parent.read(name, REPORT_LIMIT)
    value = parse_json(raw)
    assert isinstance(value, dict)
    assert _encoded(value) == raw
    if 'format' not in value:
        _validate(value)
        return value
    assert set(value) == {'format', 'header', 'snapshot', 'shards'}
    assert value['format'] == FORMAT
    _header(value['header'])
    full, pins = value['snapshot'], value['shards']
    assert isinstance(full, dict) and set(full) == {'bytes', 'sha256', 'files'}
    _integer(full['bytes'], SELECTED_LIMIT, REPORT_LIMIT + 1)
    _integer(full['files'], SELECTED_LIMIT, 1)
    _hex(full['sha256'], 64)
    assert isinstance(pins, list) and 0 < len(pins) < ENTRY_LIMIT
    length, count = 0, 0
    # Validate names, complete count and total byte budget before any shard read.
    for index, pin in enumerate(pins):
        assert isinstance(pin, dict) and set(pin) == {'path', 'bytes', 'sha256', 'files'}
        assert pin['path'] == _shard_name(name, index)
        _integer(pin['bytes'], REPORT_LIMIT, 1)
        _integer(pin['files'], full['files'], 1)
        _hex(pin['sha256'], 64)
        length += pin['bytes']
        count += pin['files']
        assert length <= SELECTED_LIMIT and count <= full['files']
    assert length + len(raw) <= SELECTED_LIMIT and count == full['files']
    rows = []
    for pin in pins:
        data = parent.read(pin['path'], REPORT_LIMIT)
        assert len(data) == pin['bytes'] and sha(data) == pin['sha256']
        shard = parse_json(data)
        assert isinstance(shard, dict) and set(shard) == {'files'}
        assert _encoded(shard) == data
        assert isinstance(shard['files'], list) and len(shard['files']) == pin['files']
        rows.extend(shard['files'])
    restored = dict(value['header'], files=rows)
    _validate(restored)  # Includes unique ordered paths across every shard boundary.
    assert _fingerprint(restored) == full
    return restored
