// SOURCE ONLY. Executable only in root's preapproved exact-source pinned-Actions run.
import assert from 'node:assert/strict';
import { mkdir, open, realpath, access, readdir, lstat } from 'node:fs/promises';
import { constants } from 'node:fs';
import { join, dirname, delimiter } from 'node:path';
import { fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';
import { command } from './owner/process.mjs';
import { descriptor, capture as copyMaterial } from './owner/files.mjs';
import { ordinaryDirectory, resolveOwnerRuntime, observeAuthenticatedPin, verifyPinned } from './binding-producer/recorder/capture.mjs';
import producerPins from './producer-pins.mjs';
import { providerPins } from './provider-pins.mjs';

const oid = /^[a-f0-9]{40}$/;
const line = data => new TextDecoder('utf-8', { fatal: true }).decode(data).replace(/\r?\n$/, '');
const bytes = value => Buffer.from(JSON.stringify(value, null, 2) + '\n');
const bytePin = ({ path, bytes, sha256 }) => ({ path, bytes, sha256 });
async function write(path, data) {
  const fd = await open(path, constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL
    | (constants.O_NOFOLLOW ?? 0), 0o400);
  try { await fd.writeFile(data); await fd.sync(); } finally { await fd.close(); }
}
async function findGit(path) {
  for (const dir of path.split(delimiter).filter(Boolean)) {
    const candidate = join(dir, process.platform === 'win32' ? 'git.exe' : 'git');
    try { await access(candidate, constants.X_OK); return await realpath(candidate); } catch {}
  }
  throw new Error('Trusted runner Git executable unavailable.');
}
async function main() {
  assert.equal(process.argv.length, 2);
  const selected = { commit: process.env.SELECTED_FINAL_MAIN, tree: process.env.SELECTED_FINAL_TREE,
    operator: process.env.SELECTED_OPERATOR_COMMIT };
  assert(Object.values(selected).every(v => typeof v === 'string' && oid.test(v)),
    'UNBOUND: root must preapprove exact source/main tree and immutable operator workflow.');
  assert(['linux', 'win32'].includes(process.platform) && process.arch === 'x64');
  assert.equal(process.version, 'v22.22.1');
  assert.equal(process.env.GITHUB_REPOSITORY, 'zryna/zryna');
  assert(/^\d+$/.test(process.env.GITHUB_RUN_ID) && /^\d+$/.test(process.env.GITHUB_RUN_ATTEMPT));
  const started = performance.now(), deadline = started + 600000;
  const workspace = await realpath(process.env.GITHUB_WORKSPACE), runnerTemp = await realpath(process.env.RUNNER_TEMP);
  const source = await realpath(join(workspace, 'source')), operator = await realpath(join(workspace, 'operator'));
  await ordinaryDirectory(source); await ordinaryDirectory(operator); await ordinaryDirectory(runnerTemp);
  const root = join(runnerTemp, 'native-tooling-preparation'); await mkdir(root, { mode: 0o700 });
  const result = await open(join(root, 'COLLECTOR_RESULT.json'), constants.O_WRONLY | constants.O_CREAT
    | constants.O_EXCL | (constants.O_NOFOLLOW ?? 0), 0o400);
  let failure = null; const observations = [];
  try {
    await result.sync();
    // Narrow actual environment only. No auth tokens, arbitrary variables or ambient test selectors.
    const env = {};
    for (const key of ['LANG', 'LC_ALL', 'TZ', 'HOME', 'USERPROFILE', 'TMPDIR', 'TEMP', 'TMP',
      'SystemRoot', 'WINDIR', 'ProgramFiles', 'COMSPEC', 'PATHEXT']) {
      if (typeof process.env[key] === 'string') env[key] = process.env[key];
    }
    const userRoot = process.platform === 'win32' ? env.USERPROFILE : env.HOME;
    env.CARGO_HOME = await realpath(process.env.CARGO_HOME ?? join(userRoot, '.cargo'));
    env.RUSTUP_HOME = await realpath(process.env.RUSTUP_HOME ?? join(userRoot, '.rustup'));
    const host = process.platform === 'win32' ? 'x86_64-pc-windows-msvc' : 'x86_64-unknown-linux-gnu';
    const toolchain = await realpath(join(env.RUSTUP_HOME, 'toolchains', `1.97.1-${host}`));
    const suffix = process.platform === 'win32' ? '.exe' : '';
    const node = await realpath(process.execPath), cargo = await realpath(join(toolchain, 'bin', 'cargo' + suffix));
    const rustc = await realpath(join(toolchain, 'bin', 'rustc' + suffix));
    env.PATH = [dirname(node), join(toolchain, 'bin'), process.env.PATH ?? process.env.Path ?? ''].join(delimiter);
    Object.assign(env, { ZRYNA_NODE: node, RUSTUP_TOOLCHAIN: '1.97.1',
      CARGO_TARGET_DIR: join(root, 'OWNED_NO_RUN_PREBUILD', 'target'), CARGO_PROFILE_DEV_DEBUG: '0',
      CARGO_PROFILE_TEST_DEBUG: '0', CARGO_BUILD_JOBS: '2', RUST_TEST_THREADS: '2' });
    const ownerRuntime = await resolveOwnerRuntime(env), git = await findGit(env.PATH);
    const toolPins = {};
    for (const [name, path] of Object.entries({ node, cargo, rustc, git, ownerRuntime })) {
      toolPins[name] = await descriptor(path, deadline);
    }
    await write(join(root, 'COLLECTOR_INTENT.json'), bytes({ selected, environment: env, toolPins,
      scope: 'Preapproved Actions source/test preparation; publication PENDING, no case pass.' }));
    const owned = async (name, program, args, cwd = source) => {
      await command(program, args, { cwd, env, output: root, stage: root, name, deadline });
      const recordPin = await descriptor(join(root, name + '.json'), deadline, 4194304);
      const record = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode((await verifyPinned(recordPin, deadline, true)).data));
      assert.equal(record.cleanupConfirmed, true); assert.equal(record.failure, null);
      const exits = record.events.filter(e => e.kind === 'root-exit');
      assert(exits.length === 1 && exits[0].status === 0);
      const stdout = await descriptor(join(root, name + '.stdout'), deadline, 1048576);
      const stderr = await descriptor(join(root, name + '.stderr'), deadline, 1048576);
      observations.push({ name, program, args, cwd, record: bytePin(recordPin), stdout: bytePin(stdout), stderr: bytePin(stderr) });
      return { pin: bytePin(stdout), data: (await verifyPinned(stdout, deadline, true)).data };
    };
    const head = await owned('GIT_HEAD', git, ['rev-parse', 'HEAD']);
    const tree = await owned('GIT_TREE', git, ['rev-parse', 'HEAD^{tree}']);
    const status = await owned('GIT_STATUS', git, ['status', '--porcelain=v1', '--untracked-files=all']);
    const tracked = await owned('GIT_FILES', git, ['ls-files', '-z']);
    const operatorHead = await owned('OPERATOR_HEAD', git, ['rev-parse', 'HEAD'], operator);
    const operatorStatus = await owned('OPERATOR_STATUS', git, ['status', '--porcelain=v1', '--untracked-files=all'], operator);
    assert.equal(line(head.data), selected.commit); assert.equal(line(tree.data), selected.tree);
    assert.equal(status.data.length, 0); assert.equal(line(operatorHead.data), selected.operator);
    assert.equal(operatorStatus.data.length, 0);
    const nodeVersion = await owned('NODE_VERSION', node, ['--version']);
    const cargoVersion = await owned('CARGO_VERSION', cargo, ['--version']);
    const rustVersion = await owned('RUSTC_VERSION', rustc, ['-vV']);
    assert.equal(line(nodeVersion.data), 'v22.22.1');
    assert(/^cargo 1\.97\.1(?: |\r?\n|$)/.test(line(cargoVersion.data)));
    assert(new TextDecoder('utf-8', { fatal: true }).decode(rustVersion.data).split(/\r?\n/).includes('release: 1.97.1'));
    const externalClosure = [bytePin(toolPins.rustc)];
    // Derive exact provider package paths/digests from the selected owning Rust source, not a C fixture.
    const capturePath = join(source, 'crates/zryna-driver/src/diagnostic_sessions/tooling_execution/capture.rs');
    const capturePin = await descriptor(capturePath, deadline, 131072);
    const capture = new TextDecoder('utf-8', { fatal: true }).decode((await verifyPinned(capturePin, deadline, true)).data);
    const providerCopies = [];
    await mkdir(join(root, 'PROVIDER_BYTES'), { mode: 0o700 });
    for (const expected of providerPins(capture)) {
      const maximum = ({ WRAPPER_MANIFEST: 8192, WRAPPER: 1024,
        TYPESCRIPT_MANIFEST: 8192, TYPESCRIPT: 12582912 })[expected.name];
      const pin = await descriptor(join(source, ...expected.components), deadline, maximum);
      assert.equal(pin.sha256, expected.sha256); externalClosure.push(bytePin(pin));
      const retained = await copyMaterial(pin.path, join(root, 'PROVIDER_BYTES', expected.name + '.bytes'), pin, deadline);
      providerCopies.push({ role: expected.name, ...retained });
    }
    // Actual selected Rust installation library inputs; the ordinary OS remains the runner TCB.
    const libraryPaths = [];
    for (const dir of [join(toolchain, 'lib'), ...(process.platform === 'win32' ? [join(toolchain, 'bin')] : [])]) {
      for (const item of await readdir(dir, { withFileTypes: true })) {
        if (item.isFile() && (/^(?:lib)?(?:rustc_driver|LLVM).*\.(?:so|dylib)(?:\..*)?$/.test(item.name)
          || process.platform === 'win32' && /\.dll$/i.test(item.name))) libraryPaths.push(join(dir, item.name));
      }
    }
    assert(libraryPaths.length <= 64);
    for (const path of [...new Set(libraryPaths)].sort()) externalClosure.push(bytePin(await descriptor(path, deadline)));
    const optionalConfigs = [];
    for (const path of [join(env.CARGO_HOME, 'config'), join(env.CARGO_HOME, 'config.toml')]) {
      try {
        const stat = await lstat(path); assert(stat.isFile() && !stat.isSymbolicLink());
        const pin = bytePin(await descriptor(path, deadline)); externalClosure.push(pin); optionalConfigs.push({ path, pin });
      } catch (error) { if (error.code !== 'ENOENT') throw error; optionalConfigs.push({ path, absent: true }); }
    }
    const adapter = join(source, 'adapters/typescript-6/node_modules/@typescript/typescript6');
    assert.equal(await realpath(adapter), join(source, 'node_modules/.pnpm/@typescript+typescript6@6.0.2/node_modules/@typescript/typescript6'));
    const authorityPath = join(root, 'PREAPPROVED_ACTIONS_RUN_OBSERVATION.json');
    const context = {};
    for (const key of ['GITHUB_REPOSITORY', 'GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT', 'GITHUB_WORKFLOW_REF',
      'GITHUB_JOB', 'GITHUB_SHA', 'RUNNER_OS', 'ImageOS', 'ImageVersion']) {
      if (typeof process.env[key] === 'string') context[key] = process.env[key];
    }
    await write(authorityPath, bytes({ format: 'private.preapproved-actions-tooling-observations.v1', selected, context,
      trustBoundary: 'Root preapproved this immutable operator workflow/code/pinned Actions/source selection. Hashes and this JSON are observations; root authenticates actual run/artifact metadata externally.',
      scope: 'Ordinary source/test preparation, publication PENDING. No final23/public support claim.', toolPins,
      ordinaryOsBoundary: { trustedRunnerImage: true, nodeLoadedModulePaths: process.report.getReport().sharedObjects,
        completeLoaderOrAllOsFileInventoryClaim: false, originalOwnerRuntimeDependencyPolicyUnchanged: true },
      selectedRustLibraryPaths: libraryPaths, optionalConfigs, providerCopies, providerSource: bytePin(capturePin),
      adapterLink: { path: adapter, canonicalTarget: await realpath(adapter) }, observations }));
    const checkoutPath = join(root, 'CHECKOUT_OBSERVATION.json');
    await write(checkoutPath, bytes({ format: 'private.exact-checkout-command-observations.v1', root: source,
      commit: selected.commit, tree: selected.tree, head: head.pin, treeOutput: tree.pin,
      status: status.pin, trackedFiles: tracked.pin }));
    const versionsPath = join(root, 'VERSION_OBSERVATION.json');
    await write(versionsPath, bytes({ nodeVersion: 'v22.22.1', rustRelease: '1.97.1', cargoRelease: '1.97.1',
      raw: { node: nodeVersion.pin, cargo: cargoVersion.pin, rustc: rustVersion.pin }, toolPins }));
    for (const pin of Object.values(toolPins)) await verifyPinned(pin, deadline);
    const actual = { format: 'private.root-selected-tooling-run-input.v1',
      authority: bytePin(await descriptor(authorityPath, deadline)),
      source: { root: source, commit: selected.commit, tree: selected.tree,
        checkoutObservation: bytePin(await descriptor(checkoutPath, deadline)) },
      tools: { node, cargo, ownerRuntime, versionObservation: bytePin(await descriptor(versionsPath, deadline)) },
      environment: env, externalClosure, outputParent: root };
    const destination = join(root, 'BINDING_PRODUCER_CAPSULE'); await mkdir(destination, { mode: 0o700 });
    for (const [relative, pin] of Object.entries(producerPins)) {
      if (relative === 'authenticated-run.mjs') continue;
      const observed = await observeAuthenticatedPin({ path: fileURLToPath(new URL('./binding-producer/' + relative, import.meta.url)), ...pin }, deadline);
      const data = (await verifyPinned(observed, deadline, true)).data;
      if (relative.startsWith('recorder/')) {
        try { await mkdir(join(destination, 'recorder'), { mode: 0o700 }); }
        catch (error) { if (error.code !== 'EEXIST') throw error; }
      }
      await write(join(destination, relative), data);
    }
    await write(join(destination, 'authenticated-run.mjs'), Buffer.from('export default ' + JSON.stringify(actual) + ';\n'));
  } catch (error) { failure = String(error.message).slice(0, 2048); process.exitCode = 1; }
  finally {
    await result.writeFile(bytes({ status: failure ? 'stopped-preserve-observations' : 'observations-recorded-not-qualified',
      failure, observations, elapsedMs: performance.now() - started, qualification: false, casePassCounts: null }));
    await result.sync(); await result.close();
  }
}
await main();
