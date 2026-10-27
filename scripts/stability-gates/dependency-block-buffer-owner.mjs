// Actual local regression owner only; it never classifies or authenticates an advisory.
import assert from 'node:assert/strict';
import { mkdirSync, realpathSync, writeFileSync } from 'node:fs';
import { dirname, isAbsolute, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { TextDecoder } from 'node:util';
import { execute } from './process.mjs';
import { canonical, MAX_LOG, readSafe, sha256 } from './input.mjs';
import { inspectCargoConfiguration, outside, pinFile, validateBinary, validateVersions } from './release-build-input.mjs';

const REGISTRY = 'registry+https://github.com/rust-lang/crates.io-index';
const CHECKSUM = '3078c7629b62d3f0439517fa394996acacc5cbc91c5a20d8c658e77abd503a71';
const PROBE = 'tests/dependency-audit/block-buffer-owner.rs';
const SELF = 'scripts/stability-gates/dependency-block-buffer-owner.mjs';
const PROBE_SHA = '0c5b8fd9c0eea321d89c921ec185e44627238b7af31d9e2a54268102f1801a6d';
const INPUTS = [['Cargo.toml', 3384, '765ba86bff634ffa0dfebf662c9fc3be00585b0b053c6b6a3e9ec48368e2e568'],
  ['Cargo.lock', 52083, 'a128e241fa1968308f2d5e2e828c94c29c413e718a7b96368e7a4032ac019751']];
const SOURCES = [SELF, PROBE, 'scripts/stability-gates/process.mjs', 'scripts/stability-gates/input.mjs',
  'scripts/stability-gates/release-build-input.mjs', 'scripts/stability-gates/interruption.mjs',
  'scripts/stability-gates/linux-tree.mjs'];
const REGISTRY_PINS = [['Cargo.toml', 917, 'c76e4bd298cd6223882238449e31939d75613e82757ca87e249df843decf5f9a'],
  ['src/lib.rs', 10700, 'b437252d389042d894b1ef17859b043e827d2bd2a1bca4b5311c67dff565274d'],
  ['src/sealed.rs', 2159, '9327ee3e567ae779b83c2b7f4f3ff56fcb2117163e023e7b0e37759e7e8d1f5c']];
export const TESTS = Object.freeze(['every_partial_position_and_input_byte_preserves_cursor_on_panic',
  'exact_advisory_eager_panic_keeps_separate_cursor', 'generator_and_padding_panics_keep_valid_separate_cursors']);
const GUARDS = Object.freeze({ cargoVersion: 3000, rustcVersion: 3000, metadata: 12000,
  build: 40000, compile: 15000, probe: 5000 });
function json(bytes) {
  assert(Buffer.isBuffer(bytes) && bytes.length <= MAX_LOG, 'bounded complete Cargo JSON');
  const text = new TextDecoder('utf-8', { fatal: true }).decode(bytes), value = JSON.parse(text);
  assert.equal(JSON.stringify(value) + '\n', text.replace(/\r\n$/u, '\n'), 'producer JSON, no duplicate keys');
  return value;
}
export function selectOwnerGraph(bytes) {
  const metadata = json(bytes);
  assert(Array.isArray(metadata.packages) && metadata.packages.length <= 4096);
  const selected = {};
  for (const [key, name, version] of [['sha2', 'sha2', '0.10.9'], ['digest', 'digest', '0.10.7'],
    ['blockBuffer', 'block-buffer', '0.10.4']]) {
    const matches = metadata.packages.filter(row => row.name === name);
    assert.equal(matches.length, 1, 'one exact selected dependency generation');
    const row = matches[0]; assert.equal(row.version, version); assert.equal(row.source, REGISTRY);
    assert.equal(row.id, `${REGISTRY}#${name}@${version}`); assert(isAbsolute(row.manifest_path));
    selected[key] = { id: row.id, name, version, source: row.source, manifestPath: row.manifest_path };
  }
  assert(Array.isArray(metadata.resolve?.nodes) && metadata.resolve.nodes.length <= 4096);
  for (const [parent, child] of [['sha2', 'digest'], ['digest', 'blockBuffer']]) {
    const nodes = metadata.resolve.nodes.filter(row => row.id === selected[parent].id);
    assert.equal(nodes.length, 1);
    assert.equal(nodes[0].deps.filter(row => row.pkg === selected[child].id).length, 1, 'actual resolved dependency edge');
  }
  return selected;
}
export function selectSha2Artifact(bytes, graph, target) {
  assert(Buffer.isBuffer(bytes) && bytes.length <= MAX_LOG);
  const lines = new TextDecoder('utf-8', { fatal: true }).decode(bytes).split(/\r?\n/u);
  assert.equal(lines.pop(), ''); assert(lines.length > 0 && lines.length <= 4096);
  const rows = lines.map(line => json(Buffer.from(line + '\n')));
  const finished = rows.filter(row => row.reason === 'build-finished');
  assert.equal(finished.length, 1); assert.equal(finished[0], rows.at(-1)); assert.equal(finished[0].success, true);
  const matches = rows.filter(row => row.reason === 'compiler-artifact' && row.package_id === graph.sha2.id);
  assert.equal(matches.length, 1);
  const row = matches[0]; assert.equal(row.target.name, 'sha2');
  assert.deepEqual(row.target.kind, ['lib']); assert.deepEqual(row.target.crate_types, ['lib']);
  assert.equal(row.target.src_path, join(dirname(graph.sha2.manifestPath), 'src', 'lib.rs'));
  assert.equal(row.profile.test, false); assert.equal(row.profile.opt_level, '3');
  const library = join(target, 'debug', 'libsha2.rlib');
  assert.equal(row.filenames.filter(path => path === library).length, 1, 'actual fresh target rlib');
  return library;
}
export function validateProbe(stdout, stderr) {
  assert(Buffer.isBuffer(stdout) && stdout.length <= MAX_LOG); assert(Buffer.isBuffer(stderr) && stderr.length === 0);
  const lines = new TextDecoder('utf-8', { fatal: true }).decode(stdout).split(/\r?\n/u).filter(Boolean);
  assert.equal(lines.length, 5); assert.equal(lines[0], 'running 3 tests');
  assert.deepEqual(lines.slice(1, 4).sort(), TESTS.map(name => `test ${name} ... ok`));
  assert.match(lines[4], /^test result: ok\. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in [0-9]+\.[0-9]+s$/u);
}
export function ownerEnvironment(input, rustc, target) {
  const keys = ['PATH', 'HOME', 'CARGO_HOME', 'RUSTUP_HOME', 'LANG', 'LC_ALL', 'TMPDIR', 'SystemRoot', 'WINDIR',
    'TEMP', 'TMP', 'USERPROFILE', 'INCLUDE', 'LIB', 'LIBPATH', 'WindowsSdkDir', 'WindowsSDKVersion',
    'UniversalCRTSdkDir', 'UCRTVersion', 'VCToolsInstallDir', 'VCToolsVersion'];
  const env = Object.fromEntries(keys.filter(key => input[key] !== undefined).map(key => [key, input[key]]));
  for (const value of Object.values(env)) assert(typeof value === 'string' && value.length > 0 && value.length <= 32768 && !value.includes('\0'));
  for (const key of ['HOME', 'CARGO_HOME', 'RUSTUP_HOME']) assert(isAbsolute(env[key]), 'explicit absolute cache/home');
  assert(env.PATH); if (process.platform === 'win32') assert(env.SystemRoot);
  return { ...env, RUSTC: rustc, RUSTUP_TOOLCHAIN: '1.97.1', CARGO_TARGET_DIR: target,
    CARGO_BUILD_JOBS: '2', CARGO_INCREMENTAL: '0', CARGO_NET_OFFLINE: 'true' };
}
function checkedPins(root, specs) {
  return specs.map(([path, bytes, hash]) => {
    const pin = pinFile(join(root, path), 2 * 1024 * 1024);
    assert.equal(pin.bytes, bytes); assert.equal(pin.sha256, hash, `reviewed exact source: ${path}`); return pin;
  });
}
function document(path, value) { writeFileSync(path, canonical(value), { flag: 'wx', mode: 0o600 }); }
function successful(result) {
  assert.equal(result.termination.exitCode, 0); assert.equal(result.termination.signal, null); assert.equal(result.termination.error, null);
}
export async function runBlockBufferOwner({ root, output, cargo, rustc }) {
  const receipt = { format: 'zryna.block-buffer-owner.v1', status: 'failed', root, output,
    host: { platform: process.platform, arch: process.arch, target: process.platform === 'win32'
      ? 'x86_64-pc-windows-msvc' : 'x86_64-unknown-linux-gnu' },
    package: { name: 'block-buffer', version: '0.10.4', source: REGISTRY, checksum: CHECKSUM },
    environment: null, guards: GUARDS, inputs: { before: null, after: null }, proofSources: { before: [], after: [] },
    tools: { before: null, after: null }, registry: { before: [], after: [] }, cargoConfiguration: { before: [], after: [] },
    graph: null, artifacts: { sha2: null, binary: null }, commands: {}, tests: TESTS, error: null,
    sourceAuthenticated: false, hostAuthenticated: false, fullSecurityQualified: false, advisoryDispositionIssued: false };
  let created = false;
  const inputs = () => Object.fromEntries(checkedPins(root, INPUTS).map((pin, i) => [i ? 'lock' : 'manifest', pin]));
  const sources = () => SOURCES.map(path => pinFile(join(root, path), 256 * 1024));
  const tools = () => Object.fromEntries(Object.entries({ cargo, rustc, node: process.execPath }).map(([key, path]) => [key, pinFile(path)]));
  const registry = () => checkedPins(dirname(receipt.graph.blockBuffer.manifestPath), REGISTRY_PINS);
  try {
    assert.equal(process.version, 'v22.22.1'); assert(['linux', 'win32'].includes(process.platform) && process.arch === 'x64');
    assert(isAbsolute(root) && root === realpathSync(root)); assert.equal(fileURLToPath(import.meta.url), join(root, SELF));
    assert(isAbsolute(output) && output === resolve(output) && dirname(output) === realpathSync(dirname(output)));
    assert(outside(root, output), 'fresh external output');
    receipt.inputs.before = inputs(); receipt.proofSources.before = sources(); receipt.tools.before = tools();
    assert.equal(sha256(readSafe(root, PROBE)), PROBE_SHA, 'reviewed complete probe bytes');
    mkdirSync(output, { mode: 0o700 }); created = true;
    const target = join(output, 'block-buffer-target'); mkdirSync(target, { mode: 0o700 });
    const binary = join(output, process.platform === 'win32' ? 'block-buffer-owner.exe' : 'block-buffer-owner');
    const env = ownerEnvironment(process.env, rustc, target); receipt.environment = env;
    receipt.cargoConfiguration.before = inspectCargoConfiguration(root, env.CARGO_HOME);
    assert(receipt.cargoConfiguration.before.every(row => !row.present), 'no Cargo configuration overrides');
    async function observe(name, executable, args) {
      const prefix = join(output, `block-buffer-${name}`), timeoutMs = GUARDS[name];
      const command = { executable, args, cwd: root, timeoutMs };
      document(`${prefix}.command.json`, command);
      const { stdout, stderr, ...termination } = await execute(executable, args, { root, env, timeoutMs });
      const logs = {};
      for (const [stream, bytes] of Object.entries({ stdout, stderr })) {
        assert(Buffer.isBuffer(bytes) && bytes.length <= MAX_LOG);
        const path = `${prefix}.${stream}.log`; writeFileSync(path, bytes, { flag: 'wx', mode: 0o600 });
        logs[stream] = { path, bytes: bytes.length, sha256: sha256(bytes) };
      }
      const value = { command, termination, logs }; document(`${prefix}.result.json`, value);
      receipt.commands[name] = value; successful(value); return { stdout, stderr };
    }
    const cargoVersion = await observe('cargoVersion', cargo, ['--version']);
    const rustcVersion = await observe('rustcVersion', rustc, ['-vV']);
    validateVersions({ cargo: { stdout: cargoVersion.stdout.toString('utf8') }, rustc: { stdout: rustcVersion.stdout.toString('utf8') },
      node: { stdout: process.version } }, receipt.host.target);
    const metadata = await observe('metadata', cargo, ['metadata', '--offline', '--locked', '--format-version', '1']);
    receipt.graph = selectOwnerGraph(metadata.stdout); receipt.registry.before = registry();
    const build = await observe('build', cargo, ['build', '--offline', '--locked', '-p', 'sha2', '--lib', '--message-format=json']);
    const library = selectSha2Artifact(build.stdout, receipt.graph, target); receipt.artifacts.sha2 = pinFile(library);
    await observe('compile', rustc, ['--edition=2024', '--test', join(root, PROBE), '--extern', `sha2=${library}`,
      '-L', `dependency=${join(dirname(library), 'deps')}`, '-o', binary]);
    receipt.artifacts.binary = pinFile(binary); validateBinary(receipt.artifacts.binary, receipt.host.target);
    const probe = await observe('probe', binary, ['--test-threads=1', '--color', 'never']);
    validateProbe(probe.stdout, probe.stderr); receipt.status = 'passed';
  } catch (error) { receipt.error = String(error.message).slice(0, 2048); }
  if (created) {
    try {
      receipt.inputs.after = inputs(); receipt.proofSources.after = sources(); receipt.tools.after = tools();
      if (receipt.graph) receipt.registry.after = registry();
      if (receipt.environment) receipt.cargoConfiguration.after = inspectCargoConfiguration(root, receipt.environment.CARGO_HOME);
      for (const key of ['inputs', 'proofSources', 'tools', 'registry', 'cargoConfiguration'])
        assert.deepEqual(receipt[key].after, receipt[key].before, `stable ${key}`);
      if (receipt.artifacts.sha2) assert.deepEqual(pinFile(receipt.artifacts.sha2.path), receipt.artifacts.sha2);
      if (receipt.artifacts.binary) assert.deepEqual(pinFile(receipt.artifacts.binary.path), receipt.artifacts.binary);
    } catch (error) { receipt.status = 'failed'; receipt.error = String(error.message).slice(0, 2048); }
    document(join(output, 'block-buffer-owner.json'), receipt);
  }
  return receipt;
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [root, output, cargo, rustc, ...extra] = process.argv.slice(2);
  if (!root || !output || !cargo || !rustc || extra.length) throw new Error('root output cargo rustc required');
  const receipt = await runBlockBufferOwner({ root, output, cargo, rustc });
  process.stdout.write(canonical(receipt)); if (receipt.status !== 'passed') process.exitCode = 1;
}
