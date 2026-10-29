// Ephemeral GitHub Linux prerequisite; never disables Chromium's own sandbox.
import assert from 'node:assert/strict';
import { existsSync, lstatSync, readFileSync, realpathSync } from 'node:fs';
import { resolve, posix } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { performance } from 'node:perf_hooks';
import { verifyBrowser } from '../../tests/scalar-host/browser-fixture.mjs';
import { readStableFile } from '../m4/registry-files.mjs';
import { childEnvironment, document, prepare, retain } from '../stability-gates/fuzz-files.mjs';
import { sha256, readSafe } from '../stability-gates/input.mjs';

const ROOT = resolve(import.meta.dirname, '../..');
const policyFiles = {
  apparmorEnabled: '/sys/module/apparmor/parameters/enabled',
  unprivilegedUserns: '/proc/sys/kernel/unprivileged_userns_clone',
  restrictedUserns: '/proc/sys/kernel/apparmor_restrict_unprivileged_userns',
};
// Isolated system Python creates exactly one root-owned file without replacement.
const CREATE = "import os,sys\np=sys.argv[1]; fd=None; owned=None; complete=False\ntry:\n fd=os.open(p,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o644)\n owned=os.fstat(fd); os.fchmod(fd,0o644)\n with os.fdopen(fd,\"wb\") as stream:\n  fd=None\n  stream.write(sys.argv[2].encode(\"ascii\")); stream.flush(); os.fsync(stream.fileno())\n complete=True\nfinally:\n if fd is not None: os.close(fd)\n if not complete and owned is not None:\n  try: current=os.lstat(p)\n  except FileNotFoundError: pass\n  else:\n   if (current.st_dev,current.st_ino)==(owned.st_dev,owned.st_ino): os.unlink(p)\n";

export function profileDefinition(executable, runId) {
  assert.match(runId, /^[1-9][0-9]{0,19}$/);
  assert.equal(typeof executable, 'string');
  assert(executable.length <= 512 && /^\/[A-Za-z0-9_./-]+\/chrome-linux64\/chrome$/.test(executable));
  assert.equal(posix.resolve(executable), executable, 'literal canonical executable path; no patterns');
  return `abi <abi/4.0>,\ninclude <tunables/global>\nprofile zryna-fuzz-browser-${runId} "${executable}" flags=(unconfined) {\n  userns,\n}\n`;
}

export function assertRestrictedPolicy(policy) {
  assert.deepEqual(policy, { apparmorEnabled: 'Y', unprivilegedUserns: '1', restrictedUserns: '1' },
    'observe the existing restricted Ubuntu user-namespace policy; never change it');
}

function kernelPolicy() {
  return Object.fromEntries(Object.entries(policyFiles).map(([key, path]) => [key, readFileSync(path, 'utf8').trim()]));
}

function installedProfile(path) {
  const stat = lstatSync(path);
  assert(stat.isFile() && !stat.isSymbolicLink() && stat.uid === 0 && stat.gid === 0 &&
    (stat.mode & 0o777) === 0o644 && stat.nlink === 1, 'single root-owned profile');
  return readStableFile(path, 4096, 'exact installed browser profile');
}

function command(output, id, argv, spawn) {
  const started = performance.now();
  const result = spawn('/usr/bin/sudo', ['-n', ...argv], { shell: false, windowsHide: true,
    timeout: 10000, maxBuffer: 65536, env: childEnvironment() });
  const stdout = result.stdout ?? Buffer.alloc(0), stderr = result.stderr ?? Buffer.alloc(0);
  document(output, `${id}.json`, { executable: '/usr/bin/sudo', args: ['-n', ...argv],
    status: result.status, signal: result.signal, error: result.error?.code ?? null,
    elapsedMs: performance.now() - started,
    stdout: retain(output, `${id}.stdout`, stdout), stderr: retain(output, `${id}.stderr`, stderr) });
  assert.equal(result.error, undefined, 'bounded host setup command');
  assert.equal(result.signal, null);
  assert.equal(result.status, 0, 'host prerequisite/cleanup command must succeed');
  return result;
}

async function setupProfile(output, context) {
  const { runId, target, exists, policy, installed, spawn } = context;
  const before = policy(); document(output, 'kernel-policy-before.json', before); assertRestrictedPolicy(before);
  const verifiedExecutable = await context.verify(context.browserRoot);
  const executable = context.realpath(verifiedExecutable);
  const definition = profileDefinition(executable, runId), bytes = Buffer.from(definition);
  retain(output, 'profile.apparmor', bytes);
  if (exists(target)) {
    document(output, 'preexisting.json', { runId, target });
    throw Error('never replace a pre-existing profile file');
  }
  command(output, 'create-profile', ['/usr/bin/python3', '-I', '-c', CREATE, target, definition], spawn);
  document(output, 'owned.json', { runId, target, executable, profileSha256: sha256(bytes) });
  assert.deepEqual(installed(target), bytes, 'root-owned profile matches the verified exact executable');
  // Add is exclusive in the kernel as well; never replace an existing loaded profile.
  command(output, 'load-profile', ['/usr/sbin/apparmor_parser', '-a', target], spawn);
  const after = policy(); document(output, 'kernel-policy-after-load.json', after); assert.deepEqual(after, before);
  document(output, 'prepared.json', { format: 'zryna.fuzz-browser-sandbox-prepared.v1', runId,
    verifiedExecutable, executable, target, profileSha256: sha256(bytes), globalPolicyChanged: false, globalM7: false });
}

function cleanupProfile(output, context) {
  const { runId, target, exists, policy, installed, spawn } = context;
  if (!exists(resolve(output, 'owned.json'))) {
    assert(!exists(target) || exists(resolve(output, 'preexisting.json')),
      'incomplete creation has unknown ownership; preserve evidence and fail closed');
    document(output, 'cleanup.json', { runId, target, owned: false, untouchedByCleanup: true, globalM7: false });
    return;
  }
  const ownership = JSON.parse(readSafe(output, 'owned.json'));
  assert.equal(ownership.runId, runId); assert.equal(ownership.target, target);
  assert(exists(target), 'owned file is missing; kernel unload is unproven');
  const bytes = installed(target);
  assert.equal(sha256(bytes), ownership.profileSha256);
  assert.equal(bytes.toString('utf8'), profileDefinition(ownership.executable, runId));
  retain(output, 'installed-before-cleanup.apparmor', bytes);
  const loadPath = resolve(output, 'load-profile.json');
  const load = exists(loadPath) ? JSON.parse(readSafe(output, 'load-profile.json')) : null;
  if (load) {
    assert.equal(load.executable, '/usr/bin/sudo');
    assert.deepEqual(load.args, ['-n', '/usr/sbin/apparmor_parser', '-a', target]);
  }
  const loaded = load?.status === 0 && load.signal === null && load.error === null;
  let unloadError;
  if (loaded) {
    try { command(output, 'unload-profile', ['/usr/sbin/apparmor_parser', '-R', target], spawn); }
    catch (error) { unloadError = error; }
  }
  command(output, 'remove-profile', ['/usr/bin/rm', '--', target], spawn);
  assert(!exists(target), 'owned host profile file removed');
  if (unloadError) throw unloadError;
  assert(load && loaded, 'failed/missing/uncertain kernel add result: no unowned kernel profile is unloaded; cleanup unproven');
  const after = policy(); document(output, 'kernel-policy-after-cleanup.json', after);
  const before = JSON.parse(readSafe(output, 'kernel-policy-before.json'));
  assert.deepEqual(after, before, 'global host policy stays unchanged');
  document(output, 'cleanup.json', { format: 'zryna.fuzz-browser-sandbox-cleanup.v1', runId,
    target, ownedFileAbsent: true, ownedKernelProfileUnloaded: loaded, globalPolicyChanged: false, globalM7: false });
}

export async function runSandbox(mode, output, options = {}) {
  assert(['prepare', 'cleanup'].includes(mode));
  const runId = options.runId ?? process.env.GITHUB_RUN_ID;
  assert.match(runId, /^[1-9][0-9]{0,19}$/);
  assert.equal(options.attempt ?? process.env.GITHUB_RUN_ATTEMPT, '1');
  assert.equal(options.platform ?? process.platform, 'linux');
  const context = { runId, target: `/etc/apparmor.d/zryna-fuzz-browser-${runId}`,
    exists: options.exists ?? existsSync, policy: options.policy ?? kernelPolicy,
    installed: options.installed ?? installedProfile, spawn: options.spawn ?? spawnSync,
    verify: options.verify ?? verifyBrowser, realpath: options.realpath ?? realpathSync,
    browserRoot: options.browserRoot ?? process.env.ZRYNA_TEST_BROWSER_ROOT };
  let ownedOutput = mode === 'cleanup';
  try {
    if (mode === 'prepare') { prepare(ROOT, output); ownedOutput = true; await setupProfile(output, context); }
    else cleanupProfile(output, context);
  } catch (error) {
    if (ownedOutput) {
      let observedPolicy = null, policyReadError = null;
      try { observedPolicy = context.policy(); } catch (failure) { policyReadError = String(failure.message).slice(0, 4096); }
      document(output, `${mode}-failure.json`, { error: String(error.message).slice(0, 4096),
        observedPolicy, policyReadError, globalM7: false });
    }
    throw error;
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    assert.equal(process.argv.length, 4, 'usage: fuzz-browser-sandbox.mjs prepare|cleanup /absolute/private-output');
    await runSandbox(process.argv[2], process.argv[3]);
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
