"""Own one Linux CI compiler/refusal observation; never admit sanitizer runtime."""
from pathlib import Path
import argparse
import hashlib
import json
import os
import platform
import re
import subprocess
import sys
sys.dont_write_bytecode = True  # Keep standalone capture imports outside tracked source effects.
from capture_io import Directory, REPORT_LIMIT, SELECTED_LIMIT, namespace, parse_json, read, sha, write_json
from capture_providers import capture, walk, copy_hash, TOOL_ROOTS, verify_inventory
from capture_runtime import Resources, run
from fixed_tools import select as select_fixed_tools
from prefix_packet import validate

SELECTOR = ('native::native_c_v0::foreign_library::tests::execution::rust_abi::ordinary::'
            'sanitizers::tests::ordinary_rust_byte_c_sanitizer_compiler_prefix_refuses_unavailable_bindings_before_run')
REPO = Path(__file__).resolve().parents[2]


def git(*args):
    result = subprocess.run(['git', *args], cwd=REPO, capture_output=True,
                            timeout=5, shell=False)
    assert result.returncode == 0 and len(result.stdout) <= 4 * 1024 * 1024
    assert len(result.stderr) <= 65536
    return result.stdout


def source(candidate):
    head = git('rev-parse', 'HEAD').decode().strip()
    tree = git('rev-parse', 'HEAD^{tree}').decode().strip()
    assert git('status', '--porcelain=v1', '--untracked-files=all') == b''
    assert git('rev-parse', '--verify', candidate + '^{commit}').decode().strip() == candidate
    assert git('rev-parse', candidate + '^{tree}').decode().strip() == tree
    rows = []
    for item in git('ls-files', '--stage', '-z').split(b'\0'):
        if not item:
            continue
        meta, raw_path = item.split(b'\t')
        mode, blob, stage = meta.decode().split()
        path = raw_path.decode('utf-8')
        assert stage == '0' and mode in ('100644', '100755')
        assert not path.startswith('/') and '..' not in Path(path).parts
        data = read(REPO / path, 2 * 1024 * 1024)
        expected = hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()
        assert expected == blob
        rows.append({'path': path, 'mode': mode, 'blob': blob,
                     'bytes': len(data), 'sha256': sha(data)})
    assert rows
    return {'candidate': candidate, 'checkoutCommit': head, 'tree': tree,
            'clean': True, 'files': rows}


def context():
    assert sys.platform == 'linux' and platform.machine() == 'x86_64'
    assert Path.cwd() == REPO
    value = {name: os.environ[name] for name in (
        'GITHUB_REPOSITORY', 'GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT', 'GITHUB_JOB',
        'GITHUB_SHA', 'GITHUB_REF', 'GITHUB_EVENT_NAME', 'M5_PREFIX_CANDIDATE')}
    assert value['GITHUB_REPOSITORY'] == 'zryna/zryna' and value['GITHUB_JOB'] == 'preflight'
    for name in ('GITHUB_SHA', 'M5_PREFIX_CANDIDATE'):
        assert re.fullmatch('[0-9a-f]{40}', value[name])
    for name in ('GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT'):
        assert value[name].isdecimal() and int(value[name]) > 0
    value['runnerImage'] = os.environ.get('ImageOS')
    value['runnerImageVersion'] = os.environ.get('ImageVersion')
    assert value['runnerImage'] and value['runnerImageVersion']
    value['host'] = {'os': 'linux', 'architecture': platform.machine(),
                     'kernel': platform.release()}
    return value


def verify_materials(report, roots):
    for alias in report['aliases']:
        fd, canonical, epoch = walk(alias['tracePath'], roots)
        try:
            assert canonical == alias['canonicalPath'] and epoch == alias['componentEpoch']
            size, digest, _ = copy_hash(fd, 64 * 1024 * 1024)
            assert (size, digest) == (alias['material']['bytes'], alias['material']['sha256'])
        finally:
            os.close(fd)


def versions(pins, selected, output, resources):
    values = []
    for index, pin in enumerate(pins):
        assert select_fixed_tools(pins) == selected
        env = {'LANG': 'C', 'LC_ALL': 'C', 'TZ': 'UTC', 'SOURCE_DATE_EPOCH': '0',
               'PATH': '/usr/bin:/bin', 'TMPDIR': str(output)}
        program = selected[index]['canonicalPath']  # Derived only from fixed source aliases.
        stdout, stderr = run([program, '--version'], output, 'tool-' + str(index),
                             resources, env=env, timeout=5, maximum=65536)
        assert stdout and stderr == b''
        assert select_fixed_tools(pins) == selected
        values.append({'sourceAlias': selected[index]['sourceAlias'], 'program': program,
                       'argv': ['--version'],
                       'stdoutBytes': len(stdout), 'stdoutSha256': sha(stdout),
                       'stderrBytes': len(stderr), 'stderrSha256': sha(stderr)})
    return values


def collect(output):
    workflow = context()
    output = Path(output)
    temporary = Path(os.environ['RUNNER_TEMP'])
    assert output == temporary / 'm5-ordinary-sanitizer-prefix'
    with Directory(temporary) as held:
        os.mkdir(output.name, mode=0o700, dir_fd=held.fd)
        held.check()
    resources = Resources(output)
    with Directory(output) as held:
        write_json(held, 'INTENT.json', {'workflow': workflow,
            'purpose': 'Compiler/refusal evidence only', 'policy': 'None', 'targets': 'UNRUN',
            'maximumSecondsIncludingCleanup': 300, 'producerSeconds': 290,
            'maximumSelectedBytes': SELECTED_LIMIT, 'resourceBaseline': resources.baseline})
        os.mkdir('observations', mode=0o700, dir_fd=held.fd)
        before = source(workflow['M5_PREFIX_CANDIDATE'])
        assert before['checkoutCommit'] == workflow['GITHUB_SHA']
        write_json(held, 'SOURCE_BEFORE.json', before)
    env = os.environ.copy()
    env['ZRYNA_RUST_ORDINARY_EVIDENCE_PARENT'] = str(output / 'observations')
    env['ZRYNA_RUST_ORDINARY_PREFIX_CAPTURE'] = '1'
    args = ['cargo', 'test', '--locked', '-p', 'zryna-driver', '--lib', SELECTOR,
            '--', '--exact', '--nocapture']
    stdout, _ = run(args, output, 'owning-test', resources, env=env)
    text = stdout.decode('utf-8')
    markers = re.findall(r'\[zryna-m5-prefix-report\] (\{[^\r\n]*\})', text)
    assert len(markers) == 1
    descriptor = parse_json(markers[0].encode())
    assert set(descriptor) == {'path', 'bytes', 'sha256'}
    assert descriptor['path'] == 'compiler-prefix-report.json'
    report_bytes = read(output / 'observations' / descriptor['path'], REPORT_LIMIT)
    assert len(report_bytes) == descriptor['bytes'] and sha(report_bytes) == descriptor['sha256']
    assert re.search(r'test ' + re.escape(SELECTOR) + r' \.\.\.', text)
    footers = re.findall(r'^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;', text, re.M)
    assert footers == [('ok', '1', '0', '0')]
    observations, tools = validate(parse_json(report_bytes), output / 'observations')
    selected_tools = select_fixed_tools(tools)
    observed_paths = observations[0]['trace']['paths']
    assert all(item['trace']['paths'] == observed_paths for item in observations)
    original = namespace(output)
    # Budget all required tool bytes and the full provider/report bound before copying.
    tool_bytes = sum(pin['bytes'] for pin in tools)
    assert tool_bytes <= 64 * 1024 * 1024
    assert original['retainedBytes'] + tool_bytes + 32 * 1024 * 1024 + 8 * REPORT_LIMIT <= SELECTED_LIMIT
    with Directory(output) as held:
        os.mkdir('tools', mode=0o700, dir_fd=held.fd)
        os.mkdir('providers', mode=0o700, dir_fd=held.fd)
    tool_materials = capture([x['canonicalPath'] for x in selected_tools],
                             output / 'tools', tools, TOOL_ROOTS)
    provider_materials = capture(observed_paths, output / 'providers')
    tool_versions = versions(tools, selected_tools, output, resources)
    verify_materials(tool_materials, TOOL_ROOTS)
    verify_materials(provider_materials, ('/usr/lib', '/usr/lib64', '/lib', '/lib64'))
    verify_inventory(tool_materials, output / 'tools')
    verify_inventory(provider_materials, output / 'providers')
    # Repeat originating receipt/product validation after all material reads and versions.
    assert read(output / 'observations' / descriptor['path'], REPORT_LIMIT) == report_bytes
    again, again_tools = validate(parse_json(report_bytes), output / 'observations')
    assert again == observations and again_tools == tools
    after = source(workflow['M5_PREFIX_CANDIDATE'])
    assert after == before
    with Directory(output) as held:
        write_json(held, 'SOURCE_AFTER.json', after)
        write_json(held, 'OBSERVED_MACHINE.json', {'cases': observations,
            'fullArraysRetained': True, 'reviewedBindings': False, 'targets': 'UNRUN'})
        resource = resources.check()
        inventory = namespace(output)
        result = {'format': 'zryna.m5.owning-linux-prefix-capture.v1',
            'capturePassed': False, 'workflow': workflow,
            'candidate': before['candidate'], 'checkoutCommit': before['checkoutCommit'],
            'tree': before['tree'], 'sourceFiles': len(before['files']),
            'compilerOnlyPassingTests': 1, 'invokedExactSelector': SELECTOR,
            'prefixReport': descriptor, 'cases': [x['case'] for x in observations],
            'actualProviderCount': provider_materials['uniqueMaterialCount'],
            'actualProviderBytes': provider_materials['uniqueMaterialBytes'],
            'toolVersions': tool_versions, 'inventoryBeforeResult': inventory,
            'finalResources': resource, 'policy': 'None', 'targetsChecked': False,
            'targetExecution': 'UNRUN', 'wholeSanitizerTestPass': False,
            'providerProfileAdmitted': False, 'loaderSelectionProven': False,
            'completeInputUse': False, 'privateRecipeQualified': False,
            'scope': 'Originating ordinary C compiler/refusal data; not five-case runtime/private/production qualification'}
        write_json(held, 'CAPTURE_RESULT.json', result)
        final = namespace(output)
        assert final['retainedBytes'] <= SELECTED_LIMIT and final['allocatedBytes'] <= SELECTED_LIMIT
        resources.check()
        held.check()
        result['capturePassed'] = True
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
        # Partial data and original raw outputs remain; no failed packet acquires PASS.
        directory = Path(args.output)
        if directory.is_dir() and not directory.is_symlink():
            try:
                with Directory(directory) as held:
                    write_json(held, 'CAPTURE_FAILURE.json', {'capturePassed': False,
                        'kind': type(error).__name__, 'message': str(error)[:2048],
                        'policy': 'None', 'targets': 'UNRUN'})
            except Exception:
                pass
        raise
