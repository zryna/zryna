"""Compile only a fixed source-exact ordinary harness; never execute Cargo test artifacts."""
from pathlib import Path
import os
import tomllib
from capture_io import Directory, REPORT_LIMIT, parse_json, read, sha, write_json
from capture_runtime import run
from owned_inputs import file_pin, snapshot_inputs, check_inputs, mirror, check_mirror

RUST = '1.97.1-x86_64-unknown-linux-gnu'
OFFICIAL = (('bin/rustc', 645000, 'd3a664c970a9fd8361b64194861bebc1ae37b9054e5ee3400dc1c9e691797eea'),
            ('lib/librustc_driver-832cf6cfb1386559.so', 157802496,
             'b0726217c199be52842c1318a340f0fc68a2c2ca0e3fa4886191dbf6ee192a58'),
            ('bin/cargo', 42839824,
             '828980723df339d62434390e9fb8ef8831036583343ae2316b7ab5646b5c1953'))
TARGET = 'x86_64-unknown-linux-gnu'
OVERRIDES = {'CARGO_BUILD_JOBS': '2', 'RUST_TEST_THREADS': '2', 'CARGO_INCREMENTAL': '0',
             'CARGO_PROFILE_DEV_DEBUG': '0', 'CARGO_PROFILE_TEST_DEBUG': '0'}


def rust_tools():
    base = Path(os.environ.get('RUSTUP_HOME', str(Path.home() / '.rustup')))
    selected = base / 'toolchains' / RUST
    root = selected.resolve(strict=True)
    assert root.is_absolute() and len(str(root)) <= 4096
    with Directory(root) as held:
        held.check()
    pins = []
    for relative, length, digest in OFFICIAL:
        pin = file_pin(root / relative, length)
        assert (pin['bytes'], pin['sha256']) == (length, digest)
        pins.append(pin)
    return {'selected': str(selected), 'sysroot': str(root), 'pins': pins}


def feature_vector(value):
    assert isinstance(value, list) and all(isinstance(item, str) and item for item in value)
    assert value == sorted(set(value)), 'sorted unique Cargo feature vector'
    return value


def source_features(spec, workspace):
    if isinstance(spec, str):
        spec = {}
    assert isinstance(spec, dict)
    inherited = workspace if spec.get('workspace') is True else {}
    if isinstance(inherited, str):
        inherited = {}
    assert isinstance(inherited, dict)
    requested = set()
    for declaration in (inherited, spec):
        features = declaration.get('features', [])
        assert isinstance(features, list) and all(isinstance(item, str) and item for item in features)
        assert len(features) == len(set(features))
        requested.update(features)
        assert isinstance(declaration.get('default-features', True), bool)
    defaults = (inherited.get('default-features', True) or spec.get('default-features', False)
                if spec.get('workspace') is True else spec.get('default-features', True))
    return requested, defaults


def direct_externs(repo, metadata, messages, dependency_directory):
    package = next(p for p in metadata['packages'] if p['manifest_path'] ==
                   str(Path(repo) / 'crates/zryna-driver/Cargo.toml'))
    assert package['name'] == 'zryna-driver'
    manifest = tomllib.loads(read(Path(package['manifest_path']), REPORT_LIMIT).decode())
    workspace = tomllib.loads(read(Path(repo) / 'Cargo.toml', REPORT_LIMIT).decode())['workspace']['dependencies']
    required = set(manifest['dependencies']) | set(manifest['dev-dependencies'])
    nodes = [n for n in metadata['resolve']['nodes'] if n['id'] == package['id']]
    assert len(nodes) == 1, 'unique originating driver resolve node'
    node = nodes[0]
    rows = []
    for name in sorted(required):
        matches = [d for d in node['deps'] if d['name'] == name.replace('-', '_') and any(
            kind['kind'] in (None, 'dev') and kind['target'] is None for kind in d['dep_kinds'])]
        assert len(matches) == 1, ('exact direct dependency package ID', name)
        dependency = matches[0]
        resolved = [n for n in metadata['resolve']['nodes'] if n['id'] == dependency['pkg']]
        assert len(resolved) == 1, ('unique direct dependency resolve node', name)
        features = feature_vector(resolved[0]['features'])
        packages = [p for p in metadata['packages'] if p['id'] == dependency['pkg']]
        assert len(packages) == 1, ('unique direct dependency package', name)
        assert isinstance(packages[0]['features'], dict)
        for section, kind in (('dependencies', None), ('dev-dependencies', 'dev')):
            if name not in manifest[section]:
                continue
            spec = manifest[section][name]
            if isinstance(spec, dict) and spec.get('workspace') is True:
                assert name in workspace, ('source workspace dependency', name)
            requested, defaults = source_features(spec, workspace.get(name, {}))
            declarations = [d for d in package['dependencies'] if
                            (d.get('rename') or d['name']) == name and
                            d['kind'] == kind and d['target'] is None]
            assert len(declarations) == 1, ('exact source direct declaration', name, kind)
            declaration = declarations[0]
            declared = declaration['features']
            assert isinstance(declared, list) and all(isinstance(f, str) and f for f in declared)
            assert len(declared) == len(set(declared)) and set(declared) == requested
            assert declaration['uses_default_features'] is defaults
            assert requested <= set(features), ('source-requested direct features', name)
            if defaults and 'default' in packages[0]['features']:
                assert 'default' in features, ('source-requested default features', name)
        artifacts = [m for m in messages if m.get('reason') == 'compiler-artifact' and
                     m['package_id'] == dependency['pkg'] and m['target']['kind'] == ['lib']]
        eligible = []
        for artifact in artifacts:
            actual_features = feature_vector(artifact['features'])
            assert artifact['profile']['opt_level'] == '0' and not artifact['profile']['test']
            assert artifact['profile']['debuginfo'] == 0
            for filename in artifact['filenames']:
                path = Path(filename)
                if path.suffix == '.rlib':
                    assert path.parent == dependency_directory
                    if actual_features == features:
                        eligible.append((artifact, path))
        assert len(eligible) == 1, ('unambiguous actual compiled direct library', name)
        artifact, path = eligible[0]
        pin = file_pin(path)
        rows.append({'name': dependency['name'], 'packageID': dependency['pkg'],
                     'features': artifact['features'], 'input': pin})
    assert len(rows) == len(required)
    return rows, package['version']


def cargo_environment(tools, inherited):
    for name in ('RUSTC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_TARGET'):
        assert not inherited.get(name), ('unsupported compiler substitution', name)
    env = dict(inherited)
    env['RUSTC'] = tools['pins'][0]['path']
    return env


def product_format(data):
    assert len(data) >= 64 and data[:7] == b'\x7fELF\x02\x01\x01'
    assert int.from_bytes(data[16:18], 'little') == 3
    assert int.from_bytes(data[18:20], 'little') == 62
    assert int.from_bytes(data[20:24], 'little') == 1
    return {'format': 'ELF64LE', 'type': 3, 'machine': 62}


def compile_owned(repo, output, resources, source):
    repo, output = Path(repo), Path(output)
    for name, value in OVERRIDES.items():
        assert os.environ.get(name) == value, ('unchanged owning profile', name)
    assert os.environ.get('RUSTFLAGS', '') == os.environ.get('CARGO_ENCODED_RUSTFLAGS', '') == ''
    tools = rust_tools()
    cargo_env = cargo_environment(tools, os.environ)
    cargo_args = [tools['pins'][2]['path'], 'test', '--locked', '-p', 'zryna-driver', '--lib', '--no-run',
                  '--message-format=json']
    raw, _ = run(cargo_args, output, 'cargo-no-run', resources, env=cargo_env)
    messages = [parse_json(line) for line in raw.splitlines() if line]
    assert messages and messages[-1] == {'reason': 'build-finished', 'success': True}
    metadata_raw, _ = run([tools['pins'][2]['path'], 'metadata', '--locked', '--offline', '--format-version', '1',
                           '--filter-platform', TARGET], output, 'cargo-metadata', resources, env=cargo_env)
    metadata = parse_json(metadata_raw)
    target = Path(os.environ.get('CARGO_TARGET_DIR', str(repo / 'target'))).absolute()
    assert Path(metadata['target_directory']) == target
    dependencies = target / 'debug/deps'
    externs, version = direct_externs(repo, metadata, messages, dependencies)
    inputs = snapshot_inputs(dependencies, tools['sysroot'])
    assert rust_tools() == tools
    root, manifest = mirror(repo, output, source)
    binary = output / 'owned-sanitizer-runtime'
    assert not binary.exists()
    args = ['--edition=2024', '--crate-name=zryna_driver_owned_sanitizer_runtime',
            '--crate-type=bin', '--cfg=test', '-Copt-level=0', '-Cdebuginfo=0',
            '--sysroot', tools['sysroot'], '-Ldependency=' + str(dependencies)]
    for item in externs:
        args += ['--extern', item['name'] + '=' + item['input']['path']]
    args += [str(root / 'crates/zryna-driver/src/lib.rs'), '-o', str(binary)]
    assert all('\n' not in arg and '\r' not in arg and not any(c.isspace() for c in arg) for arg in args)
    with Directory(output) as held:
        held.write('rustc-response.args', ('\n'.join(args) + '\n').encode())
        write_json(held, 'COMPILE_INPUTS.json', {'tools': tools, 'dependencies': inputs,
            'directExterns': externs, 'effectiveArgv': [tools['pins'][0]['path']] + args,
            'flags': OVERRIDES, 'cargoRustc': cargo_env['RUSTC'], 'normalCargoArtifactsAreInputsOnly': True})
    check_inputs(inputs, dependencies, tools['sysroot'])
    check_mirror(repo, root, manifest)
    assert rust_tools() == tools
    env = os.environ.copy()
    env['CARGO_MANIFEST_DIR'] = str(root / 'crates/zryna-driver')
    env['CARGO_PKG_VERSION'] = version
    run([tools['pins'][0]['path'], '@' + str(output / 'rustc-response.args')],
        output, 'rustc-owned', resources, env=env)
    check_inputs(inputs, dependencies, tools['sysroot'])
    check_mirror(repo, root, manifest)
    assert rust_tools() == tools
    product = file_pin(binary, 64 * 1024 * 1024)
    data = read(binary, 64 * 1024 * 1024)
    assert len(data) == product['bytes'] and sha(data) == product['sha256']
    binary_format = product_format(data)
    assert file_pin(binary, 64 * 1024 * 1024) == product
    with Directory(output) as held:
        write_json(held, 'COMPILED_PRODUCT.json', {'product': product, 'binaryFormat': binary_format,
            'compilerRecordsClosed': ['cargo-no-run', 'cargo-metadata', 'rustc-owned'],
            'scope': 'Fresh fixed source compiler product only; targets not yet run'})
    return {'product': product, 'tools': tools, 'inputs': inputs,
            'dependencies': str(dependencies), 'mirror': str(root), 'manifest': manifest}


def recheck(repo, sealed):
    assert file_pin(sealed['product']['path'], 64 * 1024 * 1024) == sealed['product']
    assert rust_tools() == sealed['tools']
    check_inputs(sealed['inputs'], sealed['dependencies'], sealed['tools']['sysroot'])
    check_mirror(repo, sealed['mirror'], sealed['manifest'])
