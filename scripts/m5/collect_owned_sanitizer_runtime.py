"""Own a fixed source-reviewed C-only runtime fixture; no generic compiler admission."""
from pathlib import Path
import argparse
import json
import os
import sys
sys.dont_write_bytecode = True
from capture_io import Directory, REPORT_LIMIT, SELECTED_LIMIT, namespace, parse_json, read, sha, write_json
from capture_runtime import Resources, run
from capture_providers import capture, walk, copy_hash, tool_identity, TOOL_ROOTS, ROOTS, verify_inventory
from fixed_tools import select
from collect_ordinary_sanitizer_prefix import REPO, context, source, verify_materials
from owned_compile import compile_owned, recheck
from owned_packet import parse_runtime_output, validate


def fixed_tools(catalog):
    from fixed_tools import ALIASES
    assert len(catalog['tools']) == len(ALIASES) == 3
    pins = []
    for alias, fixed in zip(ALIASES, catalog['tools']):
        fd, canonical, _ = walk(alias, TOOL_ROOTS)
        try:
            length, digest, _ = copy_hash(fd, 64 * 1024 * 1024)
            assert (canonical, length, digest) == (fixed['canonicalPath'], fixed['bytes'], fixed['sha256'])
            pins.append({'path': canonical, 'bytes': length, 'sha256': digest,
                         'identity': tool_identity(os.fstat(fd))})
        finally:
            os.close(fd)
    return pins, select(pins)


def provider_table(materials, catalog):
    expected = {p['canonicalPath']: (p['bytes'], p['sha256']) for p in catalog['providers']}
    assert len(expected) == len(catalog['providers']) == 15
    actual = {p['canonicalPath']: (p['bytes'], p['sha256']) for p in materials['retainedMaterials']}
    assert actual == expected
    assert materials['uniqueMaterialCount'] <= 64 and materials['uniqueMaterialBytes'] <= 32 * 1024 * 1024


def trace_aliases(paths, materials):
    expected = {p['canonicalPath']: p for p in materials['retainedMaterials']}
    aliases = []
    assert 0 < len(paths) <= 64 and len(set(paths)) == len(paths)
    for path in paths:
        fd, canonical, epoch = walk(path, ROOTS)
        try:
            assert canonical in expected
            length, digest, _ = copy_hash(fd, 32 * 1024 * 1024)
            material = expected[canonical]
            assert (length, digest) == (material['bytes'], material['sha256'])
            aliases.append({'tracePath': path, 'canonicalPath': canonical,
                            'componentEpoch': epoch, 'material': material})
        finally:
            os.close(fd)
    assert {a['canonicalPath'] for a in aliases} == set(expected)
    return aliases


def collect(output):
    workflow = context()
    output = Path(output)
    temporary = Path(os.environ['RUNNER_TEMP'])
    assert output == temporary / 'm5-ordinary-sanitizer-runtime'
    with Directory(temporary) as held:
        os.mkdir(output.name, mode=0o700, dir_fd=held.fd)
        held.check()
    resources = Resources(output)
    before = source(workflow['M5_PREFIX_CANDIDATE'])
    assert before['checkoutCommit'] == workflow['GITHUB_SHA']
    catalog_bytes = read(REPO / 'scripts/m5/owned_sanitizer_runtime_profile.json', REPORT_LIMIT)
    catalog = parse_json(catalog_bytes)
    assert catalog['format'] == 'zryna.m5.source-owned-sanitizer-profile.v1'
    assert [c['case'] for c in catalog['cases']] == ['sanitizer-positive',
        'sanitizer-heap-overrun', 'sanitizer-signed-overflow']
    pins, selected = fixed_tools(catalog)
    # Fresh source-selected bytes precede compilation/runtime. No report can select executable paths.
    with Directory(output) as held:
        write_json(held, 'INTENT.json', {'workflow': workflow,
            'purpose': 'Explicit source-reviewed ordinary C-only five-attempt fixture',
            'maximumSecondsIncludingCleanup': 300, 'producerSeconds': 290,
            'maximumSelectedBytes': SELECTED_LIMIT, 'resourceBaseline': resources.baseline,
            'defaultCompilerPrefixPolicy': 'None', 'runtimeExecuted': False,
            'profileSourceSha256': sha(catalog_bytes)})
        write_json(held, 'SOURCE_BEFORE.json', before, compact=True)
        for name in ('observations', 'tools', 'providers'):
            os.mkdir(name, mode=0o700, dir_fd=held.fd)
    tool_materials = capture([s['canonicalPath'] for s in selected], output / 'tools', pins, TOOL_ROOTS)
    provider_materials = capture([p['canonicalPath'] for p in catalog['providers']], output / 'providers')
    provider_table(provider_materials, catalog)
    verify_materials(tool_materials, TOOL_ROOTS)
    verify_materials(provider_materials, ROOTS)
    assert fixed_tools(catalog) == (pins, selected)
    sealed = compile_owned(REPO, output, resources, before)
    # This sole invocation comes from the new successful fixed compiler product seal.
    recheck(REPO, sealed)
    assert fixed_tools(catalog) == (pins, selected)
    verify_materials(provider_materials, ROOTS)
    env = os.environ.copy()
    env['ZRYNA_RUST_ORDINARY_EVIDENCE_PARENT'] = str(output / 'observations')
    stdout, stderr = run([sealed['product']['path'], str(output / 'observations')],
        output, 'owning-runtime', resources, env=env, maximum=REPORT_LIMIT)
    report = parse_runtime_output(stdout, stderr, output / 'observations')
    with Directory(output / 'observations') as held:
        report_pin = write_json(held, 'runtime-report.json', report)
    observations, observed_tools = validate(report, output / 'observations', catalog)
    assert observed_tools == pins
    paths = observations[0]['trace']['paths']
    assert all(o['trace']['paths'] == paths for o in observations)
    aliases = trace_aliases(paths, provider_materials)
    recheck(REPO, sealed)
    verify_materials(tool_materials, TOOL_ROOTS)
    verify_materials(provider_materials, ROOTS)
    assert fixed_tools(catalog) == (pins, selected)
    assert trace_aliases(paths, provider_materials) == aliases
    assert read(REPO / 'scripts/m5/owned_sanitizer_runtime_profile.json', REPORT_LIMIT) == catalog_bytes
    after = source(workflow['M5_PREFIX_CANDIDATE'])
    assert after == before
    again, again_tools = validate(report, output / 'observations', catalog)
    assert again == observations and again_tools == observed_tools
    with Directory(output) as held:
        write_json(held, 'SOURCE_AFTER.json', after, compact=True)
        write_json(held, 'TRACE_PROVIDER_ALIASES.json', {'aliases': aliases,
            'sameCanonicalFullBytesBeforeAfter': True, 'scripts': 'UNRESOLVED_OPAQUE_DATA',
            'loaderSelectionProven': False, 'completeInputUse': False})
        write_json(held, 'OBSERVED_RUNTIME.json', {'cases': observations,
            'dataInspectionIsNotRuntimeAuthority': True, 'runtimeAuthority': 'Compiled source-reviewed Rust literals'})
        final_resources = resources.check()
        inventory = namespace(output)
        result = {'format': 'zryna.m5.owning-linux-sanitizer-runtime-capture.v1',
            'capturePassed': False, 'ordinaryFiveAttemptsSucceeded': False,
            'workflow': workflow, 'candidate': before['candidate'],
            'checkoutCommit': before['checkoutCommit'], 'tree': before['tree'],
            'sourceFiles': len(before['files']), 'runtimeReport': report_pin,
            'caseOrder': [a['case'] for a in report['attempts']], 'actualToolRecords': 27,
            'actualReceipts': 6, 'compilerProduct': sealed['product'],
            'actualRunStatuses': [a['actual_run_status_raw'] for a in report['attempts']],
            'providerCount': provider_materials['uniqueMaterialCount'],
            'providerBytes': provider_materials['uniqueMaterialBytes'],
            'inventoryBeforeResult': inventory, 'finalResources': final_resources,
            'defaultCompilerPrefixPolicy': 'None', 'RustInstrumented': False,
            'ZrynaInstrumented': False, 'detachedCallRedirectExecution': 'NEVER_RUN',
            'loaderSelectionProven': False, 'completeInputUse': False,
            'privateRecipeQualified': False, 'productionSeal': False,
            'wholeResourceMatrixQualified': False,
            'scope': 'Fresh owning ordinary C-only five attempts; no generic GCC, loader/private/production or issue completion'}
        write_json(held, 'CAPTURE_RESULT.json', result)
        final = namespace(output)
        assert final['retainedBytes'] <= SELECTED_LIMIT and final['allocatedBytes'] <= SELECTED_LIMIT
        resources.check()
        held.check()
        result['capturePassed'] = result['ordinaryFiveAttemptsSucceeded'] = True
        final_bytes = (json.dumps(result, indent=2, ensure_ascii=False) + '\n').encode()
        assert len(final_bytes) <= REPORT_LIMIT
        held.finalize_reserved('CAPTURE_RESULT.json', final_bytes)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    try:
        collect(args.output)
    except Exception as error:
        directory = Path(args.output)
        if directory.is_dir() and not directory.is_symlink():
            try:
                with Directory(directory) as held:
                    write_json(held, 'CAPTURE_FAILURE.json', {'capturePassed': False,
                        'kind': type(error).__name__, 'message': str(error)[:2048],
                        'ordinaryFiveAttemptsSucceeded': False})
            except Exception:
                pass
        raise
