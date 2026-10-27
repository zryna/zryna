// Pure consistency planning only. No files, processes, images or execution capabilities.
import assert from 'node:assert/strict';
import { posix } from 'node:path';
import { canonical, exact, sha256 } from './input.mjs';
import { validateLifecyclePrebuild } from './lifecycle-owner-prebuild.mjs';

const NAME = 'authenticated_large_provider_stages_then_compiles_and_removes_owned_stage';
const PACKAGE = 'zryna-playground-compiler';
const TEST = '/lifecycle/test', PYTHON = '/usr/bin/python3', SUPERVISOR = '/lifecycle/supervisor';
const MIB = 1024 * 1024;
const SHADOW = ['/materials', '/input', '/output', '/tmp', '/dev', '/proc', '/sys'];
const PROVIDERS = [
  ['limits-v3.mjs', 123, 'c3e5d4adea4e08d4f9475269cee3f4e5f870ba052a6ea76ff1e6cb5fb23f6913'],
  ['limits-v4.mjs', 719, '7c6417f75bee60072b65eb5fe7847254835dde4bb07d6500e53c797a7cfba29b'],
  ['node_modules/@typescript/old/lib/typescript.js', 9144216, '569177652966bd528c319171c7dd22860dbf72bde116cbc4f644f1d02bb12e39'],
  ['node_modules/@typescript/old/package.json', 3527, '9332e97c30d3e53ed54910b89207ed657fb444066484df6e5b6965bf130865e9'],
  ['node_modules/@typescript/typescript6/lib/typescript.js', 45, 'd3f3cd2b04b7f466f4484df921b744223f7bd1f3e353ec9110bdf52695b983d5'],
  ['node_modules/@typescript/typescript6/package.json', 841, 'd9b8fb53c67c947fece83bcb73fa8512227ea6a12b110e096fdab8ecdc02b655'],
  ['worker-v3.mjs', 39227, '665ab7eaebae5110377b9832e746859ed4b351c26548c489970ba1d08cd04552'],
  ['worker-v4.mjs', 73118, '80ceea20ee7953a1eb8f85a4724075a6ec801983ceb16ee83752313bb3740a8a'],
  ['worker.mjs', 32112, '404f253a90e070c89fb9a2078e7d0c8144549d34559356cc80d5587e3679e868'],
];
export function lifecycleMaterialPins() {
  return [...PROVIDERS.map(([path, bytes, sha256]) => ({ path: '/materials/lib/zryna/bootstrap/' + path, bytes, sha256, mode: 0o444 })),
    { path: '/materials/runtime/node/bin/node', bytes: 124674920,
      sha256: '243fd8938011479f41b3de101842150fa990f33fbbb3f7aabd330857f2d79e1d', mode: 0o555 }];
}
const contains = (root, path) => path === root || path.startsWith(root + '/');
function path(value) {
  assert(typeof value === 'string' && value.length <= 240 && /^\/[A-Za-z0-9._@+/-]+$/u.test(value), 'bounded absolute ASCII path');
  assert(value.split('/').length <= 17, 'existing image path depth bound');
  assert(value !== '/' && value === posix.normalize(value) && !value.endsWith('/'), 'canonical path'); return value;
}
function source(value) {
  exact(value, ['commit', 'tree'], 'selected source');
  for (const key of ['commit', 'tree']) assert.match(value[key], /^[a-f0-9]{40}$/u);
}
function pin(value) {
  exact(value, ['path', 'bytes', 'sha256'], 'selected file pin'); path(value.path);
  assert(Number.isSafeInteger(value.bytes) && value.bytes > 0 && value.bytes <= 128 * MIB);
  assert.match(value.sha256, /^[a-f0-9]{64}$/u);
}
function ordered(values) {
  assert.deepEqual(values, [...new Set(values)].sort(), 'complete ordered unique inventory');
  assert.equal(new Set(values.map(value => value.toLowerCase())).size, values.length, 'case-unique paths');
}
function runtimeInventory(runtime, expected) {
  exact(runtime, ['files', 'elf', 'python', 'support'], 'declared runtime');
  assert.equal(sha256(Buffer.from(canonical(runtime))), expected.runtimeSha256, 'independently selected declared runtime');
  assert(Array.isArray(runtime.files) && runtime.files.length > 0 && runtime.files.length <= 512);
  ordered(runtime.files.map(file => file.path)); let total = 0;
  const files = new Map(), directories = new Set();
  for (const file of runtime.files) {
    exact(file, ['path', 'bytes', 'sha256', 'mode', 'kind', 'nlink', 'uid', 'gid'], 'ordinary runtime entry');
    pin({ path: file.path, bytes: file.bytes, sha256: file.sha256 });
    assert(file.kind === 'regular' && file.nlink === 1 && file.uid === 0 && file.gid === 0 && [0o444, 0o555].includes(file.mode));
    assert(!SHADOW.some(root => contains(root, file.path)), 'runtime cannot be shadowed by mounts');
    assert(!runtime.files.some(other => other.path !== file.path && contains(file.path, other.path)), 'file/directory collision');
    total += file.bytes; assert(total <= 128 * MIB, 'existing runtime aggregate bound'); files.set(file.path, file);
    for (let parent = posix.dirname(file.path); parent !== '/'; parent = posix.dirname(parent)) directories.add(parent);
  }
  assert(directories.size <= 512, 'existing runtime directory bound');
  for (const [location, selected] of [[TEST, expected.artifacts.test], [expected.artifacts.compiler.path, expected.artifacts.compiler]]) {
    const actual = files.get(location); assert(actual && actual.mode === 0o555);
    assert.equal(actual.bytes, selected.bytes); assert.equal(actual.sha256, selected.sha256, 'Cargo-associated runtime bytes');
  }
  exact(runtime.python, ['entry', 'imports', 'files'], 'declared Python bootstrap');
  assert.equal(runtime.python.entry, PYTHON); assert.deepEqual(runtime.python.imports, ['os', 'resource', 'sys']);
  assert(Array.isArray(runtime.python.files) && runtime.python.files.length <= 512); ordered(runtime.python.files);
  assert(runtime.python.files.includes(PYTHON) && runtime.python.files.every(name => files.has(name)), 'declared Python reachable files');
  assert(Array.isArray(runtime.support) && runtime.support.length <= 512); ordered(runtime.support);
  assert(runtime.support.every(name => files.has(name) && files.get(name).mode === 0o444 &&
    (name.startsWith('/resources/') || name.startsWith('/licenses/'))), 'closed inert support roots');
  assert(Array.isArray(runtime.elf) && runtime.elf.length <= 512); ordered(runtime.elf.map(row => row.path));
  const elf = new Map();
  for (const row of runtime.elf) {
    exact(row, ['path', 'prefixHex', 'interpreter', 'needed'], 'declared ELF closure'); assert(files.has(row.path));
    assert.match(row.prefixHex, /^[a-f0-9]{128,256}$/u); const prefix = Buffer.from(row.prefixHex, 'hex');
    assert.equal(prefix.toString('hex'), row.prefixHex, 'complete ELF prefix encoding');
    assert(prefix.subarray(0, 6).equals(Buffer.from([127, 69, 76, 70, 2, 1])) && prefix.readUInt16LE(18) === 62, 'declared ELF x64');
    assert(row.interpreter === null || files.has(path(row.interpreter)));
    assert(Array.isArray(row.needed) && row.needed.length <= 128); ordered(row.needed.map(item => item.name));
    for (const item of row.needed) { exact(item, ['name', 'path'], 'declared ELF edge'); assert.match(item.name, /^[A-Za-z0-9_.+-]+$/u); assert(files.has(path(item.path))); }
    elf.set(row.path, row);
  }
  const roots = [TEST, expected.artifacts.compiler.path, PYTHON, SUPERVISOR];
  for (const name of roots) assert(elf.has(name) && files.get(name).mode === 0o555, 'fixed executable closure root');
  const visited = new Set(), pending = [...roots, ...runtime.python.files.filter(name => elf.has(name))];
  while (pending.length) {
    const name = pending.pop(); if (visited.has(name)) continue; visited.add(name);
    const row = elf.get(name); assert(row, 'missing declared dependency ELF');
    pending.push(...row.needed.map(item => item.path)); if (row.interpreter) pending.push(row.interpreter);
  }
  assert.equal(visited.size, elf.size, 'no disconnected ELF declarations');
  const used = new Set([...visited, ...runtime.python.files, ...runtime.support]);
  assert.equal(used.size, files.size, 'every declared runtime file has an explicit role'); return total;
}
export function createLifecycleOwnerPlan(input) {
  exact(input, ['expected', 'capture', 'materials', 'runtime'], 'unapproved lifecycle planning input');
  const { expected, capture, materials, runtime } = input;
  exact(expected, ['source', 'root', 'targetDirectory', 'cargoInputs', 'tools', 'artifacts', 'environment', 'runtimeSha256', 'cpus'], 'independent selection');
  source(expected.source); path(expected.root); path(expected.targetDirectory);
  assert(!contains(expected.root, expected.targetDirectory) && !contains(expected.targetDirectory, expected.root), 'separate external target');
  assert(![...SHADOW, '/lifecycle', '/usr', '/lib', '/lib64', '/resources'].some(root => contains(root, expected.targetDirectory)), 'embedded compiler cannot be mount-shadowed');
  exact(expected.tools, ['cargo', 'rustc'], 'selected prebuild tools'); Object.values(expected.tools).forEach(pin);
  exact(expected.cargoInputs, ['manifest', 'lock'], 'selected Cargo inputs'); Object.values(expected.cargoInputs).forEach(pin);
  assert.equal(expected.cargoInputs.manifest.path, expected.root + '/Cargo.toml'); assert.equal(expected.cargoInputs.lock.path, expected.root + '/Cargo.lock');
  exact(expected.artifacts, ['test', 'compiler'], 'selected Cargo outputs'); Object.values(expected.artifacts).forEach(pin);
  assert.equal(expected.artifacts.compiler.path, expected.targetDirectory + '/release/' + PACKAGE);
  assert.match(expected.artifacts.test.path.slice(expected.targetDirectory.length), /^\/release\/deps\/lifecycle-[a-f0-9]{16}$/u);
  assert(expected.artifacts.test.path.startsWith(expected.targetDirectory + '/'));
  const envKeys = ['PATH', 'HOME', 'CARGO_HOME', 'RUSTUP_HOME', 'LANG', 'LC_ALL', 'RUSTC', 'RUSTUP_TOOLCHAIN',
    'CARGO_TARGET_DIR', 'CARGO_NET_OFFLINE', 'CARGO_BUILD_JOBS', 'CARGO_INCREMENTAL', 'CARGO_TERM_COLOR'];
  exact(expected.environment, envKeys, 'controlled prebuild environment');
  for (const value of Object.values(expected.environment)) assert(typeof value === 'string' && value.length > 0 && value.length <= 32768 && !value.includes('\0'));
  for (const [key, value] of Object.entries({ RUSTC: expected.tools.rustc.path, RUSTUP_TOOLCHAIN: '1.97.1',
    CARGO_TARGET_DIR: expected.targetDirectory, CARGO_NET_OFFLINE: 'true', CARGO_BUILD_JOBS: '2', CARGO_INCREMENTAL: '0', CARGO_TERM_COLOR: 'never' })) assert.equal(expected.environment[key], value);
  for (const key of ['HOME', 'CARGO_HOME', 'RUSTUP_HOME']) path(expected.environment[key]);
  assert.match(expected.runtimeSha256, /^[a-f0-9]{64}$/u);
  assert(Array.isArray(expected.cpus) && expected.cpus.length === 2 && expected.cpus[0] < expected.cpus[1]);
  assert(expected.cpus.every(cpu => Number.isSafeInteger(cpu) && cpu >= 0 && cpu <= 4095));
  assert.deepEqual(materials, lifecycleMaterialPins(), 'exact original nine providers plus Node; no mixed v4 tree');
  const materialBytes = materials.reduce((sum, file) => sum + file.bytes, 0); assert(materialBytes <= 192 * MIB);
  validateLifecyclePrebuild(expected, capture); const runtimeBytes = runtimeInventory(runtime, expected);
  const modes = [{ id: 'default', executable: TEST, args: ['--test-threads=1', '--color', 'never'],
    counts: { passed: 2, failed: 0, ignored: 1, measured: 0, filtered: 0 }, requiredIgnored: [NAME],
    requiredPassed: ['inherited_one_mib_hard_limit_rejects_before_material_creation', 'malformed_request_stops_before_material_capture_on_each_host'] },
  { id: 'positive', executable: TEST, args: ['--ignored', '--exact', NAME, '--test-threads=1', '--color', 'never'],
    counts: { passed: 1, failed: 0, ignored: 0, measured: 0, filtered: 2 }, requiredPassed: [NAME] }];
  return JSON.parse(canonical({ format: 'zryna.linux-lifecycle-owner-plan.v1', source: expected.source,
    row: PACKAGE + '/lifecycle/' + NAME, prebuild: { command: capture.command, tools: expected.tools, inputs: expected.cargoInputs,
      artifacts: expected.artifacts, configuration: capture.configuration,
      raw: Object.fromEntries(['metadata', 'stdout', 'stderr'].map(key => [key, { bytes: capture[key].length, sha256: sha256(capture[key]) }])) },
    materials: { files: materials, bytes: materialBytes, origin: 'original authenticated v0.2.3 archive required separately' },
    runtime: { sha256: expected.runtimeSha256, bytes: runtimeBytes, files: runtime.files.length,
      scope: 'Declared expected Python/ELF closure only; actual completeness, bytes, material and image authentication not established' },
    modes, environment: { ZRYNA_PLAYGROUND_TEST_MATERIALS: '/materials', TMPDIR: '/tmp', LANG: 'C.UTF-8', LC_ALL: 'C.UTF-8' },
    parent: { uid: 65534, gid: 65534, readonlyRoot: true, network: 'none', capabilities: [], noNewPrivileges: true,
      privateNamespaces: ['pid', 'mount', 'network', 'cgroup'], memoryBytes: 512 * MIB, swapBytes: 0, tasks: 32,
      seccomp: { bytes: 5459, sha256: 'bf4a72a0da350e14260d8b1534967de60bb40dff11f088c0eeae3e455c74c33b' },
      cpus: expected.cpus, cpuQuota: 200000, cpuPeriod: 100000, executionMs: 10000, cleanupMs: 2000,
      inheritedLimits: { core: 0, fsize: 256 * MIB, nofile: 512 }, stagingFsize: 12 * MIB,
      runtimeLimits: { fsize: 1277956, nofile: 128 }, materialMount: { path: '/materials', readOnly: true, noCopy: true },
      writableBytes: { '/tmp': 64 * MIB, '/input': 2 * MIB, '/output': 2 * MIB, '/dev': MIB, '/dev/shm': MIB } },
    sourceAuthenticated: false, ownerExecuted: false, materialsApproved: false, runtimeApproved: false, hostApproved: false,
    executionApproved: false, proofQualified: false, performanceQualified: false, baselineAuthority: null, thresholds: null }));
}
