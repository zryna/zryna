// Prospective trusted-root metadata producer; no native execution or authentication API.
import assert from 'node:assert/strict';
import { mkdir, open } from 'node:fs/promises';
import { constants } from 'node:fs';
import { join, dirname, resolve, isAbsolute } from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';
import { observeAuthenticatedPin, verifyPinned, ordinaryDirectory, originalOwnerSha256, observeCargoOutput } from './recorder/capture.mjs';
import { selectors, auxiliary, project, lsp, formatter } from './recorder/catalogue.mjs';
import { selectHarness, selectAuxiliary } from './recorder/select-cargo.mjs';
import templatePins from './template-pins.mjs';

const bytePin = ({ path, bytes, sha256 }) => ({ path, bytes, sha256 });
const exact = (v, keys) => assert(v && typeof v === 'object' && !Array.isArray(v)
  && Object.keys(v).sort().join(',') === [...keys].sort().join(','));
const json = value => Buffer.from(JSON.stringify(value, null, 2) + '\n');
const ENV = new Set(['PATH', 'Path', 'LANG', 'LC_ALL', 'TZ', 'HOME', 'USERPROFILE', 'TMPDIR',
  'TEMP', 'TMP', 'SystemRoot', 'WINDIR', 'ProgramFiles', 'COMSPEC', 'PATHEXT', 'ZRYNA_NODE',
  'CARGO_HOME', 'RUSTUP_HOME', 'RUSTUP_TOOLCHAIN', 'CARGO_TARGET_DIR', 'CARGO_PROFILE_DEV_DEBUG',
  'CARGO_PROFILE_TEST_DEBUG', 'CARGO_BUILD_JOBS', 'RUST_TEST_THREADS',
  ...(process.platform === 'win32' ? ['ProgramFiles(x86)', 'ProgramW6432', 'SystemDrive'] : [])]);
async function read(pin, deadline, parse = false) {
  assert(pin.bytes <= 4194304, 'Metadata/raw bound.');
  const observed = await observeAuthenticatedPin(pin, deadline);
  const data = (await verifyPinned(observed, deadline, true)).data;
  return parse ? JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(data)) : data;
}
async function write(path, data) {
  const fd = await open(path, constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL
    | (constants.O_NOFOLLOW ?? 0), 0o400);
  try { await fd.writeFile(data); await fd.sync(); } finally { await fd.close(); }
}
async function input(value, deadline) {
  assert(value !== null, 'UNBOUND: root-selected authenticated run/source/tool/closure input required.');
  exact(value, ['format', 'authority', 'source', 'tools', 'environment', 'externalClosure', 'outputParent']);
  assert.equal(value.format, 'private.root-selected-tooling-run-input.v1');
  const selected = structuredClone(value);
  assert(['linux', 'win32'].includes(process.platform) && process.arch === 'x64');
  exact(selected.source, ['root', 'commit', 'tree', 'checkoutObservation']);
  assert(/^[a-f0-9]{40}$/.test(selected.source.commit) && /^[a-f0-9]{40}$/.test(selected.source.tree));
  await ordinaryDirectory(selected.source.root); await ordinaryDirectory(selected.outputParent);
  // The trusted caller authenticates authority BEFORE invocation; this read is correspondence only.
  await read(selected.authority, deadline);
  const checkout = await read(selected.source.checkoutObservation, deadline, true);
  exact(checkout, ['format', 'root', 'commit', 'tree', 'head', 'treeOutput', 'status', 'trackedFiles']);
  assert.equal(checkout.format, 'private.exact-checkout-command-observations.v1');
  assert.equal(checkout.root, selected.source.root);
  assert.equal(checkout.commit, selected.source.commit); assert.equal(checkout.tree, selected.source.tree);
  for (const [key, expected] of [['head', selected.source.commit], ['treeOutput', selected.source.tree]]) {
    const data = await read(checkout[key], deadline);
    assert(data.equals(Buffer.from(expected + '\n')) || data.equals(Buffer.from(expected + '\r\n')));
  }
  assert.equal((await read(checkout.status, deadline)).length, 0);
  const inventory = await read(checkout.trackedFiles, deadline);
  assert(inventory.length > 0 && inventory.length <= 1048576 && inventory.at(-1) === 0);
  const names = new TextDecoder('utf-8', { fatal: true }).decode(inventory).slice(0, -1).split('\0');
  assert(names.length <= 4096 && new Set(names).size === names.length);
  const env = selected.environment;
  assert(env && typeof env === 'object' && !Array.isArray(env));
  assert(Object.entries(env).every(([k, v]) => ENV.has(k) && typeof v === 'string'
    && v.length <= 32768 && !v.includes('\0')));
  exact(selected.tools, ['node', 'cargo', 'ownerRuntime', 'versionObservation']);
  const versions = await read(selected.tools.versionObservation, deadline, true);
  // A version string is not tool authentication. Trusted Actions/run provenance is an external input.
  assert.equal(versions.nodeVersion, 'v22.22.1'); assert.equal(versions.rustRelease, '1.97.1');
  assert.equal(versions.cargoRelease, '1.97.1'); assert.equal(process.version, versions.nodeVersion);
  assert(Array.isArray(selected.externalClosure) && selected.externalClosure.length > 0);
  const modules = {};
  for (const [name, sha256] of Object.entries(originalOwnerSha256)) {
    const path = join(selected.source.root, 'scripts/m6/installed-editor', name);
    // Fixed module byte counts come from the reviewed template's original owner constants.
    const bytes = {"process.mjs": 1161, "owner-client.mjs": 9757, "files.mjs": 7036, "owner-linux.py": 13262, "owner-windows.ps1": 14737, "owner-windows-job.ps1": 8586}[name];
    modules[name] = bytePin(await observeAuthenticatedPin({ path, bytes, sha256 }, deadline));
  }
  const { descriptor } = await import(pathToFileURL(modules['files.mjs'].path).href);
  const observe = async path => bytePin(await descriptor(path, deadline));
  const sourcePins = [];
  for (const name of names) {
    assert(name && !name.includes('\\') && !isAbsolute(name)
      && !name.split('/').some(part => ['', '.', '..'].includes(part)));
    sourcePins.push(await observe(join(selected.source.root, name)));
  }
  const externalPins = [];
  for (const pin of selected.externalClosure) externalPins.push(bytePin(await observeAuthenticatedPin(pin, deadline)));
  assert(sourcePins.length + externalPins.length + 9 <= 4096);
  const tools = {};
  for (const key of ['node', 'cargo', 'ownerRuntime']) tools[key] = await observe(selected.tools[key]);
  assert.equal(process.execPath, tools.node.path);
  assert.equal(env.ZRYNA_NODE, tools.node.path); assert.equal(env.RUSTUP_TOOLCHAIN, '1.97.1');
  assert.equal(env.CARGO_TARGET_DIR, join(dirname(selected.outputParent), 'native-tooling-warm-target'));
  const proofs = [selected.authority, selected.source.checkoutObservation, ...['head', 'treeOutput', 'status', 'trackedFiles']
    .map(k => checkout[k]), selected.tools.versionObservation];
  for (const pin of proofs) await read(pin, deadline);
  return { selected, tools, modules, sourcePins, externalPins, proofs, observe };
}
async function templates(deadline) {
  const data = {};
  for (const [name, pin] of Object.entries(templatePins)) data[name] = await read({
    path: fileURLToPath(new URL('./recorder/' + name, import.meta.url)), ...pin }, deadline);
  return data;
}
async function capsule(path, files, replacement) {
  await mkdir(path, { mode: 0o700 });
  for (const [name, data] of Object.entries(files)) {
    if (name === replacement.name) continue;
    await write(join(path, name), data);
  }
  await write(join(path, replacement.name), Buffer.from(replacement.source));
}
export async function prepare(value) {
  assert.equal(process.argv.length, 2); const deadline = performance.now() + 600000;
  const x = await input(value, deadline), files = await templates(deadline), parent = x.selected.outputParent;
  const evidence = join(parent, 'PREPARATION_INPUT_PROVENANCE.json');
  const selection = { format: 'private.root-reviewed-byte-pinned-no-run-preparation.v1',
    platform: process.platform, sourceRoot: x.selected.source.root, commit: x.selected.source.commit,
    tree: x.selected.source.tree, ...x.tools, cleanSourceProof: x.selected.source.checkoutObservation,
    independentMainAndPublicationReview: x.selected.authority, environment: x.selected.environment,
    modules: x.modules, inputPins: [...x.sourcePins, ...x.externalPins, ...x.proofs], outputParent: parent };
  await write(evidence, json({ authentication: 'Root-selected trusted run/Actions/source authority; hashes are observations, not self-authentication.',
    selection, originalOwnerUnchanged: true, sourcePins: x.sourcePins, externalPins: x.externalPins,
    actualNativeExecution: false, casePassCounts: null }));
  await capsule(join(parent, 'PREPARATION_CAPSULE'), files,
    { name: 'preparation.mjs', source: files['preparation.mjs'].toString('utf8')
      .replace('export const selection = null;', `export const selection = ${JSON.stringify(selection)};`) });
}
export async function bind(value) {
  assert.equal(process.argv.length, 2); const deadline = performance.now() + 600000;
  const x = await input(value, deadline), files = await templates(deadline), parent = x.selected.outputParent;
  const prepared = await x.observe(join(parent, 'PREPARATION_INPUT_PROVENANCE.json'));
  const preparation = await read(prepared, deadline, true), s = preparation.selection;
  assert.equal(s.platform, process.platform); assert.equal(s.sourceRoot, x.selected.source.root);
  assert.equal(s.commit, x.selected.source.commit); assert.equal(s.tree, x.selected.source.tree);
  assert.deepEqual(s.cleanSourceProof, x.selected.source.checkoutObservation);
  assert.deepEqual(s.independentMainAndPublicationReview, x.selected.authority);
  assert.equal(s.outputParent, parent);
  assert.deepEqual(s.inputPins, [...x.sourcePins, ...x.externalPins, ...x.proofs]);
  assert.deepEqual(s.environment, x.selected.environment); assert.deepEqual(s.modules, x.modules);
  for (const key of ['node', 'cargo', 'ownerRuntime']) assert.deepEqual(s[key], x.tools[key]);
  for (const pin of s.inputPins) await observeAuthenticatedPin(pin, deadline);
  const root = join(parent, 'OWNED_NO_RUN_PREBUILD');
  const result = await x.observe(join(root, 'RESULT.json')), summary = await read(result, deadline, true);
  assert.equal(summary.status, 'no-run-recorded-not-approved'); assert.equal(summary.failure, null);
  assert.equal(summary.finalIdentityFailure, null); assert.equal(summary.commit, s.commit); assert.equal(summary.tree, s.tree);
  const records = {};
  for (const role of Object.keys(selectors)) {
    const owner = await x.observe(join(root, `${role}.json`)), stdout = await x.observe(join(root, `${role}.stdout`));
    const stderr = await x.observe(join(root, `${role}.stderr`));
    const actual = await read(owner, deadline, true), exits = actual.events.filter(e => e.kind === 'root-exit');
    const [pkg, target, kind] = selectors[role];
    const args = ['test', '--locked', '--offline', '-p', pkg,
      ...(kind === 'lib' ? ['--lib'] : [kind === 'bin' ? '--bin' : '--test', target]),
      '--no-run', '--message-format=json'];
    assert.deepEqual(actual.requested, { program: s.cargo.path, args, cwd: s.sourceRoot });
    assert.equal(actual.config.program, s.cargo.path); assert.deepEqual(actual.config.args, args);
    assert.equal(actual.config.cwd, s.sourceRoot); assert.equal(actual.config.stage, root);
    assert(actual.config.timeoutMs > 0 && actual.config.timeoutMs <= 600000 && actual.config.cleanupMs === 15000);
    assert.equal(actual.ownerExit.status, 0); assert.equal(actual.ownerExit.signal, null);
    assert.equal(actual.complete.status, 0); assert.equal(actual.complete.cleanupConfirmed, true);
    assert.deepEqual(actual.complete.failures, []);
    assert.equal(actual.cleanupConfirmed, true); assert.equal(actual.failure, null);
    assert(exits.length === 1 && exits[0].status === 0);
    for (const [key, pin] of [['target', s.cargo], ['runtime', s.ownerRuntime],
      ['helper', s.modules[process.platform === 'win32' ? 'owner-windows.ps1' : 'owner-linux.py']]]) {
      assert.deepEqual(bytePin(actual.pins[key]), pin); await verifyPinned(actual.pins[key], deadline);
    }
    assert.equal(actual.stdoutBytes, stdout.bytes); assert.equal(actual.stderrBytes, stderr.bytes);
    assert(stdout.bytes <= 1048576 && stderr.bytes <= 1048576);
    const support = [];
    for (const suffix of ['owner.jsonl', 'owner.stderr', 'before-start.json']) {
      support.push(await x.observe(join(root, `${role}.${suffix}`)));
    }
    records[role] = { owner, stdout, stderr, support, rows: await read(stdout, deadline) };
  }
  async function artifact(role, normal = false) {
    const [packageName, target, kind, source] = normal ? auxiliary[role][Object.keys(auxiliary[role])[0]] : selectors[role];
    const raw = records[normal ? role === 'project' ? 'cli' : 'process' : role];
    const sourcePath = join(s.sourceRoot, source), manifestPath = join(dirname(dirname(sourcePath)), 'Cargo.toml');
    const matches = new TextDecoder('utf-8', { fatal: true }).decode(raw.rows).split(/\r?\n/).filter(Boolean).map(line => JSON.parse(line)).filter(row =>
      row.reason === 'compiler-artifact' && row.target?.name === target && row.manifest_path === manifestPath
      && row.target.src_path === sourcePath && row.profile?.test === !normal
      && JSON.stringify(row.target.kind) === JSON.stringify([kind]));
    assert(matches.length === 1 && typeof matches[0].package_id === 'string');
    const expected = { packageId: matches[0].package_id, target, kind, sourcePath, manifestPath };
    const executable = (normal ? selectAuxiliary : selectHarness)(raw.rows, expected);
    return { packageName, packageId: expected.packageId, executable: bytePin(await observeCargoOutput(executable, join(s.environment.CARGO_TARGET_DIR, 'debug'), deadline)),
      prebuildStdout: raw.stdout, prebuildStderr: raw.stderr };
  }
  const review = join(parent, 'ACTUAL_PREBUILD_BINDING_PROVENANCE.json');
  await write(review, json({ rootAuthority: x.selected.authority, prepared, result,
    actualCommandRecords: Object.fromEntries(Object.entries(records).map(([k, v]) => [k, { owner: v.owner, stdout: v.stdout, stderr: v.stderr, support: v.support }])),
    scope: 'Actual no-run preparation correspondence, not independent root review or case success.' }));
  const reviewPin = await x.observe(review);
  for (const [family, commands] of Object.entries({ project, lsp, formatter })) {
    const artifacts = {}, auxiliaryExecutables = {};
    for (const role of new Set(commands.map(row => row[1]))) artifacts[role] = await artifact(role);
    if (family !== 'formatter') auxiliaryExecutables[family === 'project' ? 'cli' : 'server'] = await artifact(family, true);
    const binding = { format: 'private.root-reviewed-fixed-cargo-recorder.v1', family, platform: process.platform,
      source: { root: s.sourceRoot, commit: s.commit, tree: s.tree, cleanSourceProof: s.cleanSourceProof },
      ...x.tools, environment: s.environment, modules: s.modules, artifacts, auxiliaryExecutables,
      additionalPins: [...s.inputPins, prepared, result, ...Object.values(records).flatMap(v => [v.owner, ...v.support])], prebuildReview: reviewPin,
      outputRoot: join(parent, `RECORDED_${family.toUpperCase()}`) };
    assert(binding.additionalPins.length <= 4096);
    await capsule(join(parent, `${family.toUpperCase()}_CAPSULE`), files,
      { name: 'bindings.mjs', source: `export default ${JSON.stringify(binding)};\n` });
  }
}
