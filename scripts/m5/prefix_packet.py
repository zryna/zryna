"""Authenticate complete originating compile-only receipts before material selection."""
from pathlib import Path
import os
import stat
from capture_io import Directory, REPORT_LIMIT, read, parse_json, sha
from inspection import observe
from link_trace import select_trace

CASES = ('sanitizer-positive', 'sanitizer-heap-overrun', 'sanitizer-signed-overflow')
C_RECORDS = ('compile-client', 'compile-runtime', 'link', 'inspect-shim')
RUST_RECORDS = ('rust-version', 'rust-compile')
SOURCE_SHA = '26c4f32993bd1aabb118b8a3dfe66902bc623852564f562776218c366758826b'


def records(value, names):
    assert [x['name'] for x in value] == list(names)
    assert all(x['success'] is True and x['diagnostic'] is None and
               x['error_raw_fields_available'] is True for x in value)
    assert not any(x['name'] == 'run' for x in value)


def receipt(path, root, expected_bytes, expected_sha):
    original_path = path
    path = Path(path)
    assert str(path) == original_path
    assert path.is_absolute() and path.is_relative_to(root) and '..' not in path.parts
    raw = read(path / 'receipt.json', REPORT_LIMIT)
    assert len(raw) == expected_bytes and sha(raw) == expected_sha
    value = parse_json(raw)
    assert value['kind'] == 'ordinary-trusted-distinct-rust-fixture-observation'
    assert value['rust_targets_checked'] is False
    assert value['production_admission'] is False and value['private_recipe_qualified'] is False
    assert value['complete_input_use'] is False and value['complete_provider_inventory'] is False
    inventory, total = {}, len(raw)
    with Directory(path) as held:
        for item in value['artifacts']:
            name = item['path']
            assert name not in inventory and name.casefold() not in {x.casefold() for x in inventory}
            assert name and '/' not in name and name not in ('.', '..', 'receipt.json')
            data = held.read(name, 96 * 1024 * 1024)
            assert len(data) == item['bytes'] and sha(data) == item['sha256']
            inventory[name] = data
            total += len(data)
        assert len(inventory) < 64 and total <= 96 * 1024 * 1024
        assert {x.name for x in path.iterdir()} == set(inventory) | {'receipt.json'}
        held.check()
    return value, inventory


def validate(report, root):
    root = Path(root)
    assert report['format'] == 'zryna.m5.ordinary-initial-sanitizer-compiler-observation.v1'
    assert report['policy'] == 'None' and report['targets_checked'] is False
    assert report['sanitizer_target_execution'] == 'UNRUN' and report['test_pass'] is False
    assert report['source_bytes'] == 1696 and report['source_sha256'] == SOURCE_SHA
    assert len(report['cases']) == 3 and [x['case'] for x in report['cases']] == list(CASES)
    records(report['rust_records'], RUST_RECORDS)
    rust, rust_files = receipt(report['fresh_rust_evidence'], root,
        report['fresh_rust_receipt_bytes'], report['fresh_rust_receipt_sha256'])
    assert rust['comparator_stage'] == 'fresh-rust-profile-reference' and rust['diagnostics'] == []
    assert rust['records'] == report['rust_records']
    assert len(rust_files['foreign.rs']) == 1696 and sha(rust_files['foreign.rs']) == SOURCE_SHA
    assert sha(rust_files['foreign-original.o']) == report['fresh_rust_object_sha256']
    assert rust_files['rust-compile.stdout.log'] == rust_files['rust-compile.stderr.log'] == b''
    assert rust_files['rust-version.stderr.log'] == b''
    assert rust['rust_tool_observations'] == report['rust_observations']
    seen, observations, all_trace = {report['fresh_rust_evidence']}, [], None
    expected_files = {'compiler-prefix-report.json'}
    receipts = [(Path(report['fresh_rust_evidence']), rust_files)]
    for item in report['cases']:
        assert item['evidence'] not in seen
        seen.add(item['evidence'])
        assert item['expected_none_refusal'] == 'ZRYNA-C4102'
        assert item['phase'] == 'unavailable-independent-sanitizer-bindings'
        assert item['targets_checked'] is False and item['sanitizer_target_execution'] == 'UNRUN'
        assert item['independently_reviewed_bindings'] is False
        assert item['whole_sanitizer_test_pass'] is False and item['cleanup_errors'] is False
        assert item['compiler_inspection_records_complete'] is True
        records(item['records'], C_RECORDS)
        value, files = receipt(item['evidence'], root, item['receipt_bytes'], item['receipt_sha256'])
        assert value['records'] == item['records'] and value['diagnostics'] == ['ZRYNA-C4102']
        assert value['comparator_stage'] == item['phase']
        assert value['ordinary_tool_observations'] == report['ordinary_tools']
        assert value['rust_prerequisite']['retained_evidence'] == report['fresh_rust_evidence']
        assert value['rust_prerequisite']['original_receipt_sha256'] == report['fresh_rust_receipt_sha256']
        products = {x['name']: x for x in item['products']}
        assert len(products) == len(item['products'])
        assert set(files) == set(products) | {n + s for n in C_RECORDS for s in ('.stdout.log', '.stderr.log')}
        for name, product in products.items():
            assert len(files[name]) == product['bytes'] and sha(files[name]) == product['sha256']
        assert sha(files['foreign.o']) == report['fresh_rust_object_sha256']
        assert sha(files['program.o']) == report['genuine_zryna_object_sha256']
        assert all(files[n + '.stderr.log'] == b'' for n in C_RECORDS)
        scope = parse_json(files['sanitizer-scope.json'])
        assert scope['case'] == item['case'] and scope['instrumented'] == 'C client and private runtime only'
        assert scope['independent_hook_material_bindings_available'] is False
        assert all(scope[k] is False for k in ('Rust_instrumented', 'Zryna_instrumented',
            'production_seal', 'loader_selection_proven', 'complete_input_use', 'private_recipe_qualified'))
        link = next(x for x in value['records'] if x['name'] == 'link')
        trace = select_trace(files['link.stdout.log'], link)
        if all_trace is None:
            all_trace = trace['occurrences']
        assert trace['occurrences'] == all_trace
        machine = observe(files['final.elf'], files['inspect-shim.stdout.log'],
                          item['observed_undefined_dynamic_symbols'])
        observations.append({'case': item['case'], 'trace': trace, 'machine': machine,
                             'receiptSha256': item['receipt_sha256']})
        receipts.append((Path(item['evidence']), files))
    for directory, files in receipts:
        relative = directory.relative_to(root)
        expected_files.add(str(relative / 'receipt.json'))
        expected_files.update(str(relative / name) for name in files)
    named_files = set()
    with Directory(root) as held:
        for parent, dirs, files in os.walk(root, followlinks=False):
            for name in dirs + files:
                path = Path(parent) / name
                value = path.lstat()
                assert not stat.S_ISLNK(value.st_mode)
                assert stat.S_ISREG(value.st_mode) or stat.S_ISDIR(value.st_mode)
                if stat.S_ISREG(value.st_mode):
                    named_files.add(str(path.relative_to(root)))
        assert named_files == expected_files
        held.check()
    tools = report['ordinary_tools']['pins']
    assert len(tools) == 3 and len({x['path'] for x in tools}) == 3
    assert report['ordinary_tools']['complete_input_use'] is False
    return observations, tools
