// SOURCE ONLY. selection=null refuses before reads/writes/importing the native owner.
import assert from 'node:assert/strict';
import { mkdir, open, readFile } from 'node:fs/promises';
import { constants } from 'node:fs';
import { join, dirname } from 'node:path';
import { pathToFileURL } from 'node:url';
import { performance } from 'node:perf_hooks';
import { selection, requireSelection } from './preparation.mjs';
import { verifyPinned, observeAuthenticatedPin, ordinaryDirectory, writeMetadata, resolveOwnerRuntime, originalOwnerSha256 } from './capture.mjs';

assert.equal(process.argv.length, 2);
const selected = structuredClone(requireSelection(selection));
assert.equal(selected.format, 'private.root-reviewed-byte-pinned-no-run-preparation.v1');
assert.equal(selected.platform, process.platform);
assert(['linux', 'win32'].includes(process.platform) && process.arch === 'x64');
assert(/^[a-f0-9]{40}$/.test(selected.commit) && /^[a-f0-9]{40}$/.test(selected.tree));
const deadline = performance.now() + 3000000; // Five fixed commands; original owner max remains600000 each.
const allowed = new Set(['PATH', 'Path', 'LANG', 'LC_ALL', 'TZ', 'HOME', 'USERPROFILE', 'TMPDIR',
  'TEMP', 'TMP', 'SystemRoot', 'WINDIR', 'ProgramFiles', 'COMSPEC', 'PATHEXT', 'ZRYNA_NODE',
  'CARGO_HOME', 'RUSTUP_HOME', 'RUSTUP_TOOLCHAIN', 'CARGO_TARGET_DIR', 'CARGO_PROFILE_DEV_DEBUG', 'CARGO_PROFILE_TEST_DEBUG', 'CARGO_BUILD_JOBS', 'RUST_TEST_THREADS',
  ...(process.platform === 'win32' ? ['ProgramFiles(x86)', 'ProgramW6432', 'SystemDrive'] : [])]);
assert(Object.keys(selected.environment).every(key => allowed.has(key)
  && typeof selected.environment[key] === 'string' && !selected.environment[key].includes('\0')));
await ordinaryDirectory(selected.sourceRoot);
const stable = [];
for (const key of ['node', 'cargo', 'ownerRuntime', 'cleanSourceProof', 'independentMainAndPublicationReview']) {
  selected[key] = await observeAuthenticatedPin(selected[key], deadline); stable.push(selected[key]);
}
assert(Array.isArray(selected.inputPins) && selected.inputPins.length > 0 && selected.inputPins.length <= 4096);
const observedInputs = [];
for (const pin of selected.inputPins) observedInputs.push(await observeAuthenticatedPin(pin, deadline));
selected.inputPins = observedInputs; stable.push(...observedInputs);
assert.equal(process.execPath, selected.node.path); assert.equal(process.version, 'v22.22.1');
assert(!(selected.environment.PATH && selected.environment.Path));
assert.equal(selected.environment.RUSTUP_TOOLCHAIN, '1.97.1');
  for (const [key, value] of [['CARGO_PROFILE_DEV_DEBUG', '0'], ['CARGO_PROFILE_TEST_DEBUG', '0'],
    ['CARGO_BUILD_JOBS', '2'], ['RUST_TEST_THREADS', '2']]) {
    if (selected.environment[key] !== undefined) assert.equal(selected.environment[key], value);
  }
assert.equal(selected.environment.ZRYNA_NODE, selected.node.path);
if (process.platform === 'win32') assert.equal(selected.environment.SystemRoot, process.env.SystemRoot);
assert.equal(await resolveOwnerRuntime(selected.environment), selected.ownerRuntime.path);
assert.deepEqual(Object.keys(selected.modules).sort(), Object.keys(originalOwnerSha256).sort());
for (const [name, sha256] of Object.entries(originalOwnerSha256)) {
  const pin = selected.modules[name]; assert.equal(pin.sha256, sha256);
  assert.equal(pin.path, join(selected.sourceRoot, 'scripts/m6/installed-editor', name));
  selected.modules[name] = await observeAuthenticatedPin(pin, deadline); stable.push(selected.modules[name]);
}
await ordinaryDirectory(selected.outputParent);
const root = join(selected.outputParent, 'OWNED_NO_RUN_PREBUILD');
assert.equal(selected.environment.CARGO_TARGET_DIR, join(dirname(selected.outputParent), 'native-tooling-warm-target'));
await mkdir(root, { mode: 0o700 });
const result = await open(join(root, 'RESULT.json'), constants.O_WRONLY | constants.O_CREAT
  | constants.O_EXCL | (constants.O_NOFOLLOW ?? 0), 0o400);
const recipes = [['driver', ['-p', 'zryna-driver', '--lib']], ['cli', ['-p', 'zryna', '--test', 'cli']],
  ['main', ['-p', 'zryna', '--bin', 'zryna']], ['protocol', ['-p', 'zryna-language-server', '--test', 'protocol']],
  ['process', ['-p', 'zryna-language-server', '--test', 'process']]];
const records = []; let failure = null;
try {
  await result.sync(); await writeMetadata(join(root, 'INTENT.json'), { commit: selected.commit,
    tree: selected.tree, recipes, initialLocalIdentities: stable, scope: 'Explicit owned no-run prebuild only; no test or case pass.' });
  const { command } = await import(pathToFileURL(selected.modules['process.mjs'].path).href);
  for (const [name, target] of recipes) {
    for (const pin of stable) await verifyPinned(pin, deadline);
    await command(selected.cargo.path, ['test', '--locked', '--offline', ...target,
      '--no-run', '--message-format=json'], { cwd: selected.sourceRoot, env: selected.environment,
      output: root, stage: root, name, deadline });
    const record = JSON.parse(await readFile(join(root, `${name}.json`), 'utf8'));
    records.push({ name, record: `${name}.json`, stdout: `${name}.stdout`, stderr: `${name}.stderr` });
    assert.equal(record.cleanupConfirmed, true);
    assert.deepEqual(record.pins.target, selected.cargo);
    assert.deepEqual(record.pins.runtime, selected.ownerRuntime);
    assert.deepEqual(record.pins.helper, selected.modules[process.platform === 'win32'
      ? 'owner-windows.ps1' : 'owner-linux.py']);
    if (process.platform === 'win32') assert.deepEqual(record.pins.jobHelper, selected.modules['owner-windows-job.ps1']);
    const exits = record.events.filter(row => row.kind === 'root-exit');
    assert(exits.length === 1 && exits[0].status === 0);
  }
  for (const pin of stable) await verifyPinned(pin, deadline);
} catch (error) { failure = String(error.message).slice(0, 2048); process.exitCode = 1; }
finally {
  let finalIdentityFailure = null;
  try { for (const pin of stable) await verifyPinned(pin, deadline + 15000); }
  catch (error) { finalIdentityFailure = String(error.message).slice(0, 2048);
    failure ??= finalIdentityFailure; process.exitCode = 1; }
  await result.writeFile(JSON.stringify({ status: failure ? 'stopped-preserve-prebuild' : 'no-run-recorded-not-approved',
    commit: selected.commit, tree: selected.tree, records, failure, finalIdentityFailure, testsExecuted: false, casePassCounts: null }));
  await result.sync(); await result.close();
}
