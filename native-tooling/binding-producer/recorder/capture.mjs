// Trusted external recorder only. Root authenticates this bootstrap and Node BEFORE invocation.
import assert from 'node:assert/strict';
import { constants } from 'node:fs';
import { lstat, mkdir, open, realpath } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { dirname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { performance } from 'node:perf_hooks';
import { pathToFileURL } from 'node:url';
import { selectors, project, lsp, formatter, cargoArgs, auxiliary } from './catalogue.mjs';
import { selectHarness, selectAuxiliary } from './select-cargo.mjs';

const SHA = /^[a-f0-9]{64}$/, OID = /^[a-f0-9]{40}$/;
const ENV = new Set(['PATH', 'Path', 'LANG', 'LC_ALL', 'TZ', 'HOME', 'USERPROFILE',
  'TMPDIR', 'TEMP', 'TMP', 'SystemRoot', 'WINDIR', 'ProgramFiles', 'COMSPEC', 'PATHEXT', 'ZRYNA_NODE',
  'CARGO_HOME', 'RUSTUP_HOME', 'RUSTUP_TOOLCHAIN', 'CARGO_TARGET_DIR', 'CARGO_PROFILE_DEV_DEBUG', 'CARGO_PROFILE_TEST_DEBUG', 'CARGO_BUILD_JOBS', 'RUST_TEST_THREADS']);
const MODULES = ["process.mjs", "owner-client.mjs", "files.mjs", "owner-linux.py", "owner-windows.ps1", "owner-windows-job.ps1"];
const OWNER_SHA = ["9dbd77dee161d409c3d655fc98d34868623aa9a86ae75b4bc2694a1eedf3e32e", "f1cc8d9a0febe3e474862818e928bb72903ff06aadd5d9c4123116bb27bce4f1", "336f4328efdb8f6e9b2e2dd3930d370af55c4377440dd88f7eebead881e849ef", "4d5eba68b7c1c4b69e02d69154cf7c85c33e3a25ec44f5c1ebff153356da04df", "e3a88b725f5820bbe48e00df6a1f7dcdd96f8d066b29f60fcc713eb11c4d87ab", "e1a78c8b3210ca19126b1bcdb4f5ae904d016f7b978b709b7f95d76f136ea1e6"];
const clock = deadline => assert(performance.now() < deadline, 'Family admission deadline expired.');
const bytes = value => Buffer.from(JSON.stringify(value));
const exact = (value, keys) => assert(value && typeof value === 'object' && !Array.isArray(value)
  && Object.keys(value).sort().join(',') === [...keys].sort().join(','), 'Closed input shape.');
const identity = s => ({ dev: String(s.dev), ino: String(s.ino), bytes: String(s.size),
  links: String(s.nlink), mode: String(s.mode), modifiedNs: String(s.mtimeNs), changedNs: String(s.ctimeNs) });
const regular = s => assert(s.isFile() && !s.isSymbolicLink() && s.nlink === 1n && s.ino !== 0n);
async function directory(path) {
  assert(isAbsolute(path) && resolve(path) === path);
  for (let current = path;; current = dirname(current)) {
    const s = await lstat(current); assert(s.isDirectory() && !s.isSymbolicLink());
    assert.equal(await realpath(current), current);
    if (dirname(current) === current) break;
  }
}
async function read(path, maximum, deadline, expected = null, collect = false) {
  clock(deadline); await directory(dirname(path));
  const before = await lstat(path, { bigint: true }); regular(before);
  assert(before.size <= BigInt(maximum));
  const fd = await open(path, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0));
  const hash = createHash('sha256'), chunks = [], chunk = Buffer.alloc(65536); let total = 0;
  try {
    assert.deepEqual(identity(await fd.stat({ bigint: true })), identity(before));
    for (;;) {
      clock(deadline); const { bytesRead } = await fd.read(chunk, 0, chunk.length, total);
      if (!bytesRead) break;
      total += bytesRead; assert(total <= maximum); hash.update(chunk.subarray(0, bytesRead));
      if (collect) chunks.push(Buffer.from(chunk.subarray(0, bytesRead)));
    }
    const after = await fd.stat({ bigint: true }), named = await lstat(path, { bigint: true });
    regular(after); regular(named); assert.deepEqual(identity(after), identity(before));
    assert.deepEqual(identity(named), identity(before)); assert.equal(BigInt(total), before.size);
    const result = { path, bytes: total, sha256: hash.digest('hex'), identity: identity(before) };
    if (expected) assert.deepEqual(result, expected, 'Pinned file changed.');
    return { pin: result, data: collect ? Buffer.concat(chunks) : null };
  } finally { await fd.close(); }
}
async function verify(pin, deadline, collect = false) {
  exact(pin, ['path', 'bytes', 'sha256', 'identity']);
  assert(isAbsolute(pin.path) && pin.path === resolve(pin.path) && SHA.test(pin.sha256));
  assert(Number.isSafeInteger(pin.bytes) && pin.bytes >= 0 && pin.bytes <= 536870912);
  return read(pin.path, pin.bytes, deadline, pin, collect);
}
async function observe(pin, deadline) {
  // Independent root byte/provenance authentication precedes fresh local identity observation.
  exact(pin, ['path', 'bytes', 'sha256']);
  assert(isAbsolute(pin.path) && pin.path === resolve(pin.path) && SHA.test(pin.sha256));
  assert(Number.isSafeInteger(pin.bytes) && pin.bytes >= 0 && pin.bytes <= 536870912);
  const actual = await read(pin.path, pin.bytes, deadline);
  assert.equal(actual.pin.bytes, pin.bytes); assert.equal(actual.pin.sha256, pin.sha256);
  return actual.pin;
}
async function checkAll(pins, deadline) { for (const pin of pins) await verify(pin, deadline); }
async function syncDirectory(path) {
  await directory(path);
  // No Windows directory-flush gate or local crash-durability claim is introduced.
  if (process.platform === 'win32') return;
  const fd = await open(path, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0));
  try { await fd.sync(); } finally { await fd.close(); }
}
async function writeNew(path, value) {
  const fd = await open(path, constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL
    | (constants.O_NOFOLLOW ?? 0), 0o400);
  try { await fd.writeFile(bytes(value)); await fd.sync(); } finally { await fd.close(); }
  await syncDirectory(dirname(path));
}
async function runtimePath(env) {
  if (process.platform === 'win32') return realpath(join(env.ProgramFiles, 'PowerShell', '7', 'pwsh.exe'));
  for (const root of env.PATH.split(':')) {
    try { return await realpath(join(root, 'python3')); } catch {}
  }
  throw new Error('Fixed original owner runtime unavailable.');
}
async function preflight(binding, family, catalogue, deadline) {
  // This null template refuses BEFORE filesystem writes, native imports, or process effects.
  assert(binding !== null, 'UNBOUND: actual final source/tools/prebuild/artifacts are required.');
  exact(binding, ['format', 'family', 'platform', 'source', 'node', 'cargo', 'ownerRuntime', 'environment',
    'modules', 'artifacts', 'auxiliaryExecutables', 'additionalPins', 'prebuildReview', 'outputRoot']);
  assert.equal(binding.format, 'private.root-reviewed-fixed-cargo-recorder.v1');
  // Clone before observation; caller pins are never replaced with post-command identities.
  binding = structuredClone(binding);
  assert.equal(binding.family, family); assert.equal(binding.platform, process.platform);
  assert(['linux', 'win32'].includes(process.platform) && process.arch === 'x64');
  exact(binding.source, ['root', 'commit', 'tree', 'cleanSourceProof']);
  assert(OID.test(binding.source.commit) && OID.test(binding.source.tree));
  await directory(binding.source.root);
  const stable = [];
  for (const [object, key] of [[binding.source, 'cleanSourceProof'], [binding, 'node'],
    [binding, 'cargo'], [binding, 'ownerRuntime'], [binding, 'prebuildReview']]) {
    object[key] = await observe(object[key], deadline); stable.push(object[key]);
  }
  assert.equal(process.execPath, binding.node.path); assert.equal(process.version, 'v22.22.1');
  const env = binding.environment; assert(env && typeof env === 'object' && !Array.isArray(env));
  assert(Object.keys(env).every(key => ENV.has(key) && typeof env[key] === 'string'
    && env[key].length <= 32768 && !env[key].includes('\0')));
  assert(!(env.PATH && env.Path));
  assert.equal(env.RUSTUP_TOOLCHAIN, '1.97.1');
  for (const [key, value] of [['CARGO_PROFILE_DEV_DEBUG', '0'], ['CARGO_PROFILE_TEST_DEBUG', '0'],
    ['CARGO_BUILD_JOBS', '2'], ['RUST_TEST_THREADS', '2']]) {
    if (env[key] !== undefined) assert.equal(env[key], value);
  } assert.equal(env.ZRYNA_NODE, binding.node.path);
  assert(process.platform === 'win32' ? typeof env.ProgramFiles === 'string' : typeof env.PATH === 'string');
  if (process.platform === 'win32') assert.equal(env.SystemRoot, process.env.SystemRoot);
  assert.equal(await runtimePath(env), binding.ownerRuntime.path);
  await directory(env.CARGO_TARGET_DIR);
  const insideTarget = path => {
    const local = relative(env.CARGO_TARGET_DIR, path);
    assert(local && local !== '..' && !local.startsWith('..' + sep) && !isAbsolute(local));
  };
  exact(binding.modules, MODULES);
  for (const name of MODULES) {
    assert.equal(binding.modules[name].path, join(binding.source.root, 'scripts/m6/installed-editor', name));
    assert.equal(binding.modules[name].sha256, OWNER_SHA[MODULES.indexOf(name)]);
    binding.modules[name] = await observe(binding.modules[name], deadline); stable.push(binding.modules[name]);
  }
  assert(Array.isArray(binding.additionalPins) && binding.additionalPins.length > 0
    && binding.additionalPins.length <= 4096);
  const observedInputs = [];
  for (const pin of binding.additionalPins) observedInputs.push(await observe(pin, deadline));
  binding.additionalPins = observedInputs; stable.push(...observedInputs);
  const roles = [...new Set(catalogue.map(row => row[1]))]; exact(binding.artifacts, roles);
  for (const role of roles) {
    const selected = binding.artifacts[role], [pkg, target, kind, source] = selectors[role];
    exact(selected, ['packageName', 'packageId', 'executable', 'prebuildStdout', 'prebuildStderr']);
    assert.equal(selected.packageName, pkg); assert.equal(typeof selected.packageId, 'string');
    for (const key of ['executable', 'prebuildStdout', 'prebuildStderr']) {
      selected[key] = await observe(selected[key], deadline); stable.push(selected[key]);
    }
    insideTarget(selected.executable.path);
    assert(selected.prebuildStdout.bytes <= 4194304);
    const raw = await verify(selected.prebuildStdout, deadline, true);
    const manifestPath = join(dirname(dirname(join(binding.source.root, source))), 'Cargo.toml');
    assert.equal(selectHarness(raw.data, { packageId: selected.packageId, target, kind,
      sourcePath: join(binding.source.root, source), manifestPath }), selected.executable.path);
  }
  exact(binding.auxiliaryExecutables, Object.keys(auxiliary[family]));
  for (const [name, [pkg, target, kind, source]] of Object.entries(auxiliary[family])) {
    const item = binding.auxiliaryExecutables[name];
    exact(item, ['packageName', 'packageId', 'executable', 'prebuildStdout', 'prebuildStderr']);
    assert.equal(item.packageName, pkg); assert.equal(typeof item.packageId, 'string');
    for (const key of ['executable', 'prebuildStdout', 'prebuildStderr']) {
      item[key] = await observe(item[key], deadline); stable.push(item[key]);
    }
    insideTarget(item.executable.path);
    assert(item.prebuildStdout.bytes <= 4194304);
    const raw = await verify(item.prebuildStdout, deadline, true);
    assert.equal(selectAuxiliary(raw.data, { packageId: item.packageId, target, kind,
      sourcePath: join(binding.source.root, source),
      manifestPath: join(dirname(dirname(join(binding.source.root, source))), 'Cargo.toml') }), item.executable.path);
  }
  clock(deadline); await directory(dirname(binding.outputRoot));
  assert(isAbsolute(binding.outputRoot) && resolve(binding.outputRoot) === binding.outputRoot);
  await checkAll(stable, deadline); return { binding, stable, env };
}
async function retainCommand(root, name, deadline) {
  const suffixes = ['stdout', 'stderr', 'owner.jsonl', 'owner.stderr', 'json', 'before-start.json'];
  const pins = {};
  for (const suffix of suffixes) {
    const path = join(root, `${name}.${suffix}`);
    const fd = await open(path, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0));
    try { if (process.platform !== 'win32') await fd.sync(); } finally { await fd.close(); }
    pins[suffix] = (await read(path, 4194304, deadline)).pin;
  }
  await syncDirectory(root); return pins;
}
async function run(binding, family, catalogue) {
  const started = performance.now(), deadline = started + 600000;
  assert.equal(process.argv.length, 2, 'No selectors or arguments.');
  const admitted = await preflight(binding, family, catalogue, deadline);
  ({ binding } = admitted); const { stable, env } = admitted;
  const root = binding.outputRoot; await mkdir(root, { mode: 0o700 });
  await syncDirectory(root); await syncDirectory(dirname(root));
  const result = await open(join(root, 'RESULT.json'), constants.O_WRONLY | constants.O_CREAT
    | constants.O_EXCL | (constants.O_NOFOLLOW ?? 0), 0o400);
  const observations = []; let failure = null;
  try {
    await result.sync(); await syncDirectory(root);
    await writeNew(join(root, 'INTENT.json'), { family, source: binding.source,
      catalogue, initialLocalIdentities: stable, argvKind: 'cargo-plus-harness-command',
      mayRecompile: true, selectedExecutableChangeRefuses: true, originalOwnerLimitsUnchanged: true,
      caseAssembly: 'Root independent review only; no case pass counts produced.' });
    const commands = join(root, 'commands'); await mkdir(commands, { mode: 0o700 });
    await syncDirectory(commands); await syncDirectory(root);
    const { command } = await import(pathToFileURL(binding.modules['process.mjs'].path).href);
    for (const [name, role, filter] of catalogue) {
      clock(deadline); await checkAll(stable, deadline);
      let commandError = null;
      try { await command(binding.cargo.path, cargoArgs(role, filter),
        { cwd: binding.source.root, env,
          output: commands, stage: root, name, deadline }); }
      catch (error) { commandError = String(error.message).slice(0, 2048); }
      // Original owner has already retained full streams (up to its unchanged 1MiB) here.
      const pins = await retainCommand(commands, name, deadline + 15000);
      const raw = await read(pins.json.path, 4194304, deadline + 15000, pins.json, true);
      const owner = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(raw.data));
      const exits = owner.events.filter(event => event.kind === 'root-exit');
      observations.push({ name, role, filter, pins, ownerError: commandError,
        targetExits: exits, cleanupConfirmed: owner.cleanupConfirmed });
      assert.deepEqual(owner.pins.target, binding.cargo);
      assert.deepEqual(owner.pins.runtime, binding.ownerRuntime);
      assert.deepEqual(owner.pins.helper, binding.modules[process.platform === 'win32'
        ? 'owner-windows.ps1' : 'owner-linux.py']);
      if (process.platform === 'win32') assert.deepEqual(owner.pins.jobHelper, binding.modules['owner-windows-job.ps1']);
      assert.equal(commandError, null); assert.equal(owner.cleanupConfirmed, true);
      assert.equal(exits.length, 1); assert.equal(exits[0].status, 0);
      assert.equal(owner.stdoutBytes, pins.stdout.bytes); assert.equal(owner.stderrBytes, pins.stderr.bytes);
      assert(pins.stdout.bytes <= 262144 && pins.stderr.bytes <= 262144,
        'Final carrier oversize: original full raw retained; later admission stopped.');
      await checkAll(stable, deadline); clock(deadline);
    }
    await checkAll(stable, deadline);
  } catch (error) { failure = String(error.message).slice(0, 2048); process.exitCode = 1; }
  finally {
    // Recheck even when a command/output assertion failed; no identity is adopted after execution.
    let finalIdentityFailure = null;
    try { await checkAll(stable, deadline + 15000); }
    catch (error) { finalIdentityFailure = String(error.message).slice(0, 2048);
      failure ??= finalIdentityFailure; process.exitCode = 1; }
    const value = { format: 'private.native-tooling-command-recordings.v1', family,
      status: failure ? 'stopped-preserve-original-evidence' : 'commands-recorded-not-qualified',
      source: binding.source, observations, failure, finalIdentityFailure, elapsedMs: performance.now() - started,
      casePassCounts: null, rawStreamMeaning: 'Complete Cargo plus harness command stdout/stderr; summary is own stdout.' };
    try { await result.writeFile(bytes(value)); await result.sync(); }
    finally { await result.close(); }
    await syncDirectory(root);
    process.stdout.write(JSON.stringify({ family, status: value.status,
      recordedCommands: observations.length, failure, qualification: false }) + '\n');
  }
}
export const recordProject = binding => run(binding, 'project', project);
export const recordLsp = binding => run(binding, 'lsp', lsp);
export const recordFormatter = binding => run(binding, 'formatter', formatter);
// Read/metadata utilities shared only with the fixed no-run preparation entrypoint.
export { verify as verifyPinned, directory as ordinaryDirectory, writeNew as writeMetadata,
  runtimePath as resolveOwnerRuntime, observe as observeAuthenticatedPin };
export const originalOwnerSha256 = Object.freeze(Object.fromEntries(MODULES.map((name, i) => [name, OWNER_SHA[i]])));
