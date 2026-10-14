"""Pure complete originating five-attempt data checks; no executable nominations."""
from pathlib import Path
import os
import stat
from capture_io import Directory, REPORT_LIMIT, read, parse_json, sha
from prefix_packet import C_RECORDS, RUST_RECORDS, SOURCE_SHA
from inspection import observe
from link_trace import select_trace

ORDER = ('sanitizer-positive', 'sanitizer-heap-overrun', 'sanitizer-positive',
         'sanitizer-signed-overflow', 'sanitizer-positive')


def digest32(value):
    assert isinstance(value, list) and len(value) == 32
    assert all(type(byte) is int and 0 <= byte <= 255 for byte in value)
    return bytes(value).hex()


def parse_runtime_output(stdout, stderr, root):
    assert isinstance(stdout, bytes) and 0 < len(stdout) <= REPORT_LIMIT
    assert isinstance(stderr, bytes) and len(stderr) <= REPORT_LIMIT
    report = parse_json(stdout)
    assert isinstance(report, dict)
    assert report['format'] == 'zryna.m5.ordinary-explicit-sanitizer-five-attempt-observation.v1'
    assert report['existing_assertions_completed'] is True
    assert isinstance(report['attempts'], list)
    assert [a['case'] for a in report['attempts']] == list(ORDER)
    root = Path(root)
    assert root.is_absolute() and '..' not in root.parts
    paths = [report['fresh_rust_evidence']] + [a['evidence'] for a in report['attempts']]
    assert len(paths) == 6 and all(isinstance(p, str) for p in paths)
    assert len(set(paths)) == 6
    for value in paths:
        directory = Path(value)
        assert str(directory) == value and directory.is_absolute()
        assert directory != root and directory.is_relative_to(root) and '..' not in directory.parts
        assert not any(ord(c) < 32 or ord(c) == 127 for c in value)
    expected = ''.join('m5-ordinary-rust-evidence ' + value + '\n' for value in paths).encode('utf-8')
    assert stderr == expected, 'exact six originating evidence markers'
    return report


def records(value, names, negative=False):
    assert [r['name'] for r in value] == list(names)
    for row in value:
        assert row['diagnostic'] is None and row['error_raw_fields_available'] is True
        assert row['success'] is (not (negative and row['name'] == 'run'))
        assert row['child_env_cleared'] is True
        assert row['child_env'] == {'LANG': 'C', 'LC_ALL': 'C', 'TZ': 'UTC',
            'SOURCE_DATE_EPOCH': '0', 'PATH': '/usr/bin:/bin', 'TMPDIR': row['directory']}


def receipt(path, root, size, digest, targets):
    assert isinstance(path, str) and str(Path(path)) == path
    directory = Path(path)
    assert directory.is_absolute() and directory.is_relative_to(root) and '..' not in directory.parts
    raw = read(directory / 'receipt.json', REPORT_LIMIT)
    assert len(raw) == size and sha(raw) == digest
    value = parse_json(raw)
    assert value['kind'] == 'ordinary-trusted-distinct-rust-fixture-observation'
    assert value['rust_targets_checked'] is targets and value['diagnostics'] == []
    assert all(value[k] is False for k in ('production_admission', 'private_recipe_qualified',
        'complete_input_use', 'complete_provider_inventory', 'GCC_origin_for_Rust'))
    files, total = {}, len(raw)
    with Directory(directory) as held:
        for item in value['artifacts']:
            name = item['path']
            assert name and name not in ('.', '..', 'receipt.json') and '/' not in name
            assert name not in files and name.casefold() not in {n.casefold() for n in files}
            data = held.read(name, 32 * 1024 * 1024)
            assert len(data) == item['bytes'] and sha(data) == item['sha256']
            total += len(data)
            assert total <= 96 * 1024 * 1024 and len(files) < 64
            files[name] = data
        assert {p.name for p in directory.iterdir()} == set(files) | {'receipt.json'}
        held.check()
    return value, files


def validate(report, root, catalog):
    root = Path(root)
    assert [c['case'] for c in catalog['cases']] == [ORDER[0], ORDER[1], ORDER[3]]
    assert report['format'] == 'zryna.m5.ordinary-explicit-sanitizer-five-attempt-observation.v1'
    assert report['existing_assertions_completed'] is True
    assert report['sanitizer_instrumented'] == 'C client/private runtime only'
    assert all(report[k] is False for k in ('Rust_instrumented', 'Zryna_instrumented',
        'private_recipe_qualified', 'production_seal', 'loader_selection_proven', 'complete_input_use'))
    assert report['provider_script_references'] == 'UNRESOLVED_OPAQUE_DATA'
    assert report['source_bytes'] == 1696 and report['source_sha256'] == SOURCE_SHA
    assert [a['case'] for a in report['attempts']] == list(ORDER)
    rust_object = digest32(report['fresh_rust_object_sha256'])
    zryna_object = digest32(report['genuine_zryna_object_sha256'])
    records(report['rust_records'], RUST_RECORDS)
    rust, rust_files = receipt(report['fresh_rust_evidence'], root,
        report['fresh_rust_receipt_bytes'], report['fresh_rust_receipt_sha256'], False)
    assert rust['records'] == report['rust_records'] and rust['comparator_stage'] == 'fresh-rust-profile-reference'
    assert len(rust_files['foreign.rs']) == 1696 and sha(rust_files['foreign.rs']) == SOURCE_SHA
    assert sha(rust_files['foreign-original.o']) == rust_object
    assert rust_files['rust-version.stderr.log'] == b''
    assert rust_files['rust-compile.stdout.log'] == rust_files['rust-compile.stderr.log'] == b''
    observed, receipts, seen, traces = [], [(Path(report['fresh_rust_evidence']), rust_files)], {report['fresh_rust_evidence']}, None
    for attempt in report['attempts']:
        case = attempt['case']
        negative = case != 'sanitizer-positive'
        assert attempt['evidence'] not in seen
        seen.add(attempt['evidence'])
        assert attempt['targets_checked'] is True and attempt['diagnostics_empty'] is True
        assert attempt['detached_call_redirect_execution'] == 'NEVER_RUN'
        names = C_RECORDS + ('run',)
        records(attempt['records'], names, negative)
        value, files = receipt(attempt['evidence'], root, attempt['receipt_bytes'],
                               attempt['receipt_sha256'], True)
        assert value['records'] == attempt['records'] and value['comparator_stage'] == 'run'
        assert value['ordinary_tool_observations'] == report['ordinary_tools']
        assert value['rust_prerequisite']['retained_evidence'] == report['fresh_rust_evidence']
        assert value['rust_prerequisite']['original_receipt_sha256'] == report['fresh_rust_receipt_sha256']
        assert value['rust_prerequisite']['records'] == report['rust_records']
        assert value['rust_prerequisite']['rust_tool_observations'] == rust['rust_tool_observations']
        products = {p['name']: p for p in attempt['products']}
        assert len(products) == len(attempt['products'])
        assert set(files) == set(products) | {n + suffix for n in names for suffix in ('.stdout.log', '.stderr.log')}
        for name, product in products.items():
            assert len(files[name]) == product['bytes'] and sha(files[name]) == product['sha256']
        assert sha(files['foreign.o']) == rust_object
        assert sha(files['program.o']) == zryna_object
        assert all(files[n + '.stderr.log'] == b'' for n in C_RECORDS)
        out, err = bytes(attempt['actual_run_stdout']), bytes(attempt['actual_run_stderr'])
        assert out == files['run.stdout.log'] and err == files['run.stderr.log']
        assert len(out) <= 4096 and len(err) <= 65536
        status = attempt['actual_run_status_raw']
        assert isinstance(status, int) and 0 <= status <= 65535 and (status != 0) is negative
        if negative:
            assert out == b''
            text = err.decode('utf-8')
            if case == 'sanitizer-heap-overrun':
                assert 'm5-c-sanitizer:heap-overrun\n' in text
                assert 'AddressSanitizer: heap-buffer-overflow' in text and 'READ of size 1' in text
            else:
                assert 'm5-c-sanitizer:signed-overflow\n' in text
                assert 'runtime error: signed integer overflow:' in text
        else:
            assert out == b'ordinary-rust-bytes:4entered4prechecks\n' and err == b''
        scope = parse_json(files['sanitizer-scope.json'])
        assert scope['case'] == case and scope['instrumented'] == 'C client and private runtime only'
        assert scope['independent_hook_material_bindings_available'] is True
        assert all(scope[k] is False for k in ('Rust_instrumented', 'Zryna_instrumented',
            'production_seal', 'loader_selection_proven', 'complete_input_use', 'private_recipe_qualified'))
        binding = next(c for c in catalog['cases'] if c['case'] == case)
        machine = observe(files['final.elf'], files['inspect-shim.stdout.log'], binding['undefined'])
        assert machine['orderedMnemonics'] == binding['mnemonics']
        assert set(c['name'] for c in machine['calls']) == set(binding['hooks']) | {'sum_bytes'}
        changed = files['detached-call-redirect-never-run.elf']
        assert len(changed) == len(files['final.elf']) and changed != files['final.elf']
        try:
            observe(changed, files['inspect-shim.stdout.log'], binding['undefined'])
        except AssertionError:
            pass
        else:
            raise AssertionError('detached call-redirection must reject without running')
        link = next(r for r in value['records'] if r['name'] == 'link')
        trace = select_trace(files['link.stdout.log'], link)
        if traces is None:
            traces = trace['occurrences']
        assert trace['occurrences'] == traces
        observed.append({'case': case, 'trace': trace, 'machineDataOnly': machine,
            'receiptSha256': attempt['receipt_sha256'], 'actualRunStatusRaw': status,
            'detachedExecution': 'NEVER_RUN'})
        receipts.append((Path(attempt['evidence']), files))
    expected = {'runtime-report.json'}
    for directory, files in receipts:
        relative = directory.relative_to(root)
        expected.add(str(relative / 'receipt.json'))
        expected.update(str(relative / name) for name in files)
    actual = set()
    with Directory(root) as held:
        for parent, dirs, names in os.walk(root, followlinks=False):
            for name in dirs + names:
                p = Path(parent) / name
                s = p.lstat()
                assert not stat.S_ISLNK(s.st_mode) and (stat.S_ISDIR(s.st_mode) or stat.S_ISREG(s.st_mode))
                assert s.st_dev == os.fstat(held.fd).st_dev
                if stat.S_ISREG(s.st_mode):
                    actual.add(str(p.relative_to(root)))
        assert actual == expected
        held.check()
    assert sum(len(a['records']) for a in report['attempts']) + len(report['rust_records']) == 27
    tools = report['ordinary_tools']['pins']
    assert len(tools) == 3
    for actual, fixed in zip(tools, catalog['tools']):
        assert (actual['path'], actual['bytes'], actual['sha256']) == (
            fixed['canonicalPath'], fixed['bytes'], fixed['sha256'])
    return observed, tools
