// Dedicated local Cargo no-run capture. Artifacts are discovery observations, never approvals.
import assert from 'node:assert/strict';
import { posix } from 'node:path';
import { exact, MAX_LOG, portable, sha256 } from './input.mjs';
import { sourceUnchanged } from './source-census-v2.mjs';
import { validateBinary } from './release-build-input.mjs';
import { parseLifecycleCargoOutput, validateLifecycleCargoArtifacts } from './lifecycle-owner-prebuild.mjs';
import { BUILD_ARGS, DEFAULT_OBSERVATIONS, PACKAGE, TARGET, validateCaptureInput } from './lifecycle-owner-capture-io.mjs';
const { basename, dirname, join } = posix;

export async function captureLifecyclePrebuild(input, observations = {}) {
  input = structuredClone(input);
  const targetDirectory = validateCaptureInput(input);
  assert(Object.keys(observations).every(key => Object.hasOwn(DEFAULT_OBSERVATIONS, key)), 'closed observation adapters');
  assert(Object.values(observations).every(value => typeof value === 'function'));
  const io = { ...DEFAULT_OBSERVATIONS, ...observations };
  assert.deepEqual(io.host(), { platform: 'linux', arch: 'x64', node: 'v22.22.1' }, 'native Linux Node22 capture only');
  io.prepare(input);
  const write = io.writer(input.output), commands = [], failures = [];
  const { root, tools, environment, source } = input;
  const mode = Object.keys(observations).length ? 'synthetic-observation-adapters' : 'local-observed-capture';
  write('input.json', { ...input, observationMode: mode });
  let stage = 'preflight', buildStarted = false, postflightAttempted = false;
  let unconfirmedCommand = null;
  let before, after, beforeGraph, beforeMaterials, build;
  const selected = path => {
    const pin = io.pin(path, 128 * 1024 * 1024);
    return { path, bytes: pin.bytes, sha256: pin.sha256 };
  };
  async function observe(name, executable, args, timeoutMs = 120000) {
    const command = { executable, args: [...args], cwd: root, environment: { ...environment }, timeoutMs, shell: false };
    write(name + '.command.json', command); commands.push(name);
    let result;
    try { result = await io.run(executable, [...args], { root, env: { ...environment }, timeoutMs }); }
    catch (error) { unconfirmedCommand = name; throw error; }
    if (result?.error !== null || result?.signal !== null || !Number.isSafeInteger(result?.exitCode)
      || result.exitCode < 0 || result.exitCode > 255) unconfirmedCommand = name;
    assert(Buffer.isBuffer(result.stdout) && Buffer.isBuffer(result.stderr), 'original child byte streams');
    assert(result.stdout.length + result.stderr.length <= MAX_LOG, 'combined command output bound');
    const stdout = Buffer.alloc(result.stdout.length), stderr = Buffer.alloc(result.stderr.length);
    result.stdout.copy(stdout); result.stderr.copy(stderr);
    const { stdout: ignoredOut, stderr: ignoredErr, ...termination } = result;
    write(name + '.stdout.raw', stdout); write(name + '.stderr.raw', stderr);
    write(name + '.result.json', { termination, stdout: { bytes: stdout.length, sha256: sha256(stdout) },
      stderr: { bytes: stderr.length, sha256: sha256(stderr) } });
    exact(result, ['exitCode', 'signal', 'error', 'elapsedMs', 'stdout', 'stderr'], 'complete command result');
    assert(Number.isFinite(result.elapsedMs) && result.elapsedMs >= 0);
    return { command, termination, stdout, stderr };
  }
  function success(result) {
    assert.deepEqual([result.termination.exitCode, result.termination.signal, result.termination.error],
      [0, null, null], 'command, EOF and owned cleanup completed');
  }
  async function query(name, executable, args) {
    const result = await observe(name, executable, args); success(result); return result.stdout;
  }
  function snapshot(suffix) {
    const value = {}, errors = [];
    const capture = (name, read) => {
      try { value[name] = read(); write(name + '-' + suffix + '.json', value[name]); }
      catch (error) { errors.push(name + ': ' + String(error.message)); }
    };
    capture('source', () => io.census({ root, directory: join(input.output, 'source-' + suffix), executable: tools.git, env: environment }));
    capture('inputs', () => ({ source: value.source && { commit: value.source.commit, tree: value.source.tree },
      tools: Object.fromEntries(Object.entries(tools).map(([name, path]) => [name, selected(path)])),
      inputs: { manifest: selected(join(root, 'Cargo.toml')), lock: selected(join(root, 'Cargo.lock')) } }));
    capture('configuration', () => io.configuration(root, environment.CARGO_HOME));
    capture('workspace', () => { io.workspace(root); return { exactSecurityInputsMatched: true }; });
    if (errors.length) { write('snapshot-errors-' + suffix + '.json', errors); throw Error(errors.join('; ')); }
    return value;
  }
  async function graph(suffix) {
    const metadata = await query('metadata-' + suffix, tools.cargo,
      ['metadata', '--format-version=1', '--locked', '--offline', '--filter-platform', TARGET]);
    const parsed = JSON.parse(metadata.toString('utf8'));
    assert(Buffer.from(JSON.stringify(parsed) + '\n').equals(metadata), 'original complete metadata JSON');
    const tree = await query('tree-' + suffix, tools.cargo,
      ['tree', '--locked', '--offline', '--target', TARGET, '--package', PACKAGE,
        '--edges', 'normal,build', '--prefix', 'none', '--format', '{p}']);
    const proof = io.treeProof(tree, parsed, root); write('tree-' + suffix + '.json', proof);
    return { metadata, parsed, tree, proof };
  }
  function materials(value, suffix) {
    const result = io.materials(input, value);
    write('cache-closure-' + suffix + '.json', result.closure);
    for (const file of result.licenses) { portable(file.path); write('licenses-' + suffix + '/' + file.path, file.data); }
    return result;
  }
  async function postflight() {
    postflightAttempted = true;
    const unavailable = () => {
      failures.push('after-state unavailable: unconfirmed termination of ' + unconfirmedCommand);
      write('after-unavailable.json', { command: unconfirmedCommand, reason: 'termination or cleanup unconfirmed; no subsequent subprocesses' });
    };
    if (unconfirmedCommand) { unavailable(); return; }
    try {
      const value = await graph('after'), captured = materials(value, 'after');
      assert.deepEqual(beforeGraph.parsed, value.parsed); assert.deepEqual(beforeGraph.proof, value.proof);
      assert(beforeMaterials.closure.equals(captured.closure), 'unchanged complete crate cache and licenses');
    } catch (error) { failures.push('materials-after: ' + String(error.message)); }
    finally {
      if (unconfirmedCommand) unavailable();
      else try { after = snapshot('after'); } catch (error) { failures.push('snapshot-after: ' + String(error.message)); }
    }
  }
  try {
    before = snapshot('before');
    assert.deepEqual({ commit: before.source.commit, tree: before.source.tree }, source, 'selected actual source');
    assert(before.configuration.every(row => !row.present), 'all Cargo configurations absent before queries');
    const versions = { cargo: await query('cargo-version', tools.cargo, ['--version']),
      rustc: await query('rustc-version', tools.rustc, ['-vV']) };
    assert.equal(versions.cargo.toString().trim(), 'cargo 1.97.1 (c980f4866 2026-06-30)');
    const rustc = versions.rustc.toString().trim().split('\n');
    assert.equal(rustc[0], 'rustc 1.97.1 (8bab26f4f 2026-07-14)'); assert(rustc.includes('host: ' + TARGET));
    await query('rustc-sysroot', tools.rustc, ['--print', 'sysroot']);
    await query('rustc-target-libdir', tools.rustc, ['--print', 'target-libdir']);
    beforeGraph = await graph('before'); beforeMaterials = materials(beforeGraph, 'before');
    stage = 'fixed-no-run-build'; buildStarted = true;
    build = await observe('build', tools.cargo, BUILD_ARGS, 20 * 60 * 1000);
    stage = 'postflight'; await postflight();
    success(build); assert.deepEqual(failures, [], 'all postflight observations available and unchanged');
    sourceUnchanged(before.source, after.source);
    assert.deepEqual(before.inputs, after.inputs, 'stable selected tools and Cargo inputs');
    assert.deepEqual(before.configuration, after.configuration);
    assert(after.configuration.every(row => !row.present), 'all Cargo configurations remain absent');
    stage = 'artifact-discovery';
    const rows = parseLifecycleCargoOutput(build.stdout, beforeGraph.parsed);
    const testRows = rows.filter(row => row.reason === 'compiler-artifact' && row.target?.name === 'lifecycle' && row.executable);
    assert.equal(testRows.length, 1);
    assert.equal(dirname(testRows[0].executable), join(targetDirectory, 'release/deps'));
    assert.match(basename(testRows[0].executable), /^lifecycle-[a-f0-9]{16}$/u);
    const paths = { test: testRows[0].executable, compiler: join(targetDirectory, 'release', PACKAGE) };
    const artifactPins = Object.fromEntries(Object.entries(paths).map(([key, path]) => [key, io.pin(path, 128 * 1024 * 1024)]));
    write('artifact-pins.json', artifactPins);
    for (const [key, pin] of Object.entries(artifactPins)) { assert.equal(pin.path, paths[key]); validateBinary(pin, TARGET); }
    const discoveredArtifacts = Object.fromEntries(Object.entries(artifactPins).map(([key, pin]) =>
      [key, { path: pin.path, bytes: pin.bytes, sha256: pin.sha256 }]));
    write('artifacts-discovered.json', discoveredArtifacts);
    const cargoTools = Object.fromEntries(['cargo', 'rustc'].map(key => [key, before.inputs.tools[key]]));
    validateLifecycleCargoArtifacts({ root, tools: cargoTools, artifacts: discoveredArtifacts },
      { metadata: beforeGraph.metadata, stdout: build.stdout, stderr: build.stderr });
    const result = { format: 'zryna.lifecycle-local-prebuild-discovery.v1', observationMode: mode, source,
      targetDirectory, tools: before.inputs.tools, cargoInputs: before.inputs.inputs, discoveredArtifacts,
      environment, commands, rawValidated: true, unchangedSource: true, cacheValidatedBeforeAfter: true,
      toolsOriginAuthenticated: false, artifactsIndependentlyApproved: false, sourceAuthenticated: false,
      hostAuthenticated: false, executionApproved: false, runtimeApproved: false, testExecuted: false, ownerExecuted: false, proofQualified: false,
      performanceQualified: false, baselineAuthority: null, thresholds: null, issueClosed: false };
    write('discovery.json', result); return result;
  } catch (error) {
    if (buildStarted && !postflightAttempted) await postflight();
    write('failure.json', { stage, message: String(error.stack ?? error), commands, postflightFailures: failures,
      observationMode: mode, sourceAuthenticated: false, hostAuthenticated: false, executionApproved: false,
      testExecuted: false, ownerExecuted: false, proofQualified: false });
    throw error;
  }
}
