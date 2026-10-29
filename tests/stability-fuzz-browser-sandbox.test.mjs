// Inert controls: no browser, sudo command, native payload, or kernel profile runs.
import assert from 'node:assert/strict';
import { existsSync, mkdtempSync, readFileSync, rmSync, unlinkSync } from 'node:fs';
import { resolve } from 'node:path';
import { tmpdir } from 'node:os';
import test from 'node:test';
import { profileDefinition, assertRestrictedPolicy, runSandbox } from '../scripts/scalar-host/fuzz-browser-sandbox.mjs';

const executable = '/home/runner/work/zryna/zryna/.zryna/cache/pinned/browser/chrome-linux64/chrome';
const runId = '123456789';
const restricted = { apparmorEnabled: 'Y', unprivilegedUserns: '1', restrictedUserns: '1' };
function fixture(t) {
  const parent = mkdtempSync(resolve(tmpdir(), 'zryna-browser-policy-'));
  t.after(() => rmSync(parent, { recursive: true, force: true }));
  const output = resolve(parent, 'original'), calls = [];
  const state = { installed: null, failLoad: false, failUnload: false, interruptCreate: false, policy: () => ({ ...restricted }) };
  const options = { runId, attempt: '1', platform: 'linux', browserRoot: '/fixture/browser',
    realpath: path => path,
    verify: async root => { assert.equal(root, '/fixture/browser'); return executable; },
    policy: () => state.policy(),
    exists: path => path.startsWith('/etc/apparmor.d/') ? state.installed !== null : existsSync(path),
    installed: () => state.installed,
    spawn: (program, args, command) => {
      calls.push({ program, args, command });
      assert.equal(program, '/usr/bin/sudo'); assert.equal(args[0], '-n');
      assert.equal(command.shell, false); assert.equal(command.timeout, 10000);
      assert.equal(command.maxBuffer, 65536);
      if (args[1] === '/usr/bin/python3') {
        assert.deepEqual(args.slice(2, 4), ['-I', '-c']);
        assert(args[4].includes('os.O_EXCL|os.O_NOFOLLOW'));
        assert.equal(args[5], `/etc/apparmor.d/zryna-fuzz-browser-${runId}`);
        state.installed = Buffer.from(args[6]);
        if (state.interruptCreate) return { status: null, signal: 'SIGKILL', stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) };
      }
      if (args[1] === '/usr/bin/rm') state.installed = null;
      return { status: (state.failLoad && args.includes('-a')) || (state.failUnload && args.includes('-R')) ? 1 : 0, signal: null,
        stdout: Buffer.from('synthetic command stream\n'), stderr: Buffer.alloc(0) };
    } };
  return { output, state, calls, options };
}

test('profile grants userns to one literal executable and rejects path or profile injection', () => {
  const text = profileDefinition(executable, runId);
  assert(text.includes(`"${executable}"`)); assert.equal(text.match(/userns,/g).length, 1);
  for (const path of [executable+'*', executable.replace('pinned','**'), executable.replace('pinned','a/../b'),
    executable.replace('pinned','a" {}'), executable.replace('pinned','a\nb'), 'relative/chrome-linux64/chrome']) {
    assert.throws(() => profileDefinition(path, runId));
  }
  for (const id of ['0', '1\nother', '1;rm', '1'.repeat(21)]) assert.throws(() => profileDefinition(executable, id));
});

test('host preparation rejects weakened or unknown global user namespace policy', () => {
  assertRestrictedPolicy(restricted);
  for (const change of [{ apparmorEnabled: 'N' }, { unprivilegedUserns: '0' }, { restrictedUserns: '0' }, { extra: true }]) {
    assert.throws(() => assertRestrictedPolicy({ ...restricted, ...change }));
  }
});

test('inert preparation records bounded direct commands and exact owned-profile cleanup', async t => {
  const value = fixture(t);
  await runSandbox('prepare', value.output, value.options);
  assert.equal(JSON.parse(readFileSync(resolve(value.output,'prepared.json'))).globalM7, false);
  assert.deepEqual(value.state.installed, Buffer.from(profileDefinition(executable, runId)));
  await runSandbox('cleanup', value.output, value.options);
  assert.equal(value.calls.length, 4); assert.equal(value.state.installed, null);
  assert(value.calls[2].args.includes('-R'));
  assert.equal(JSON.parse(readFileSync(resolve(value.output,'cleanup.json'))).ownedFileAbsent, true);
});

test('unverified browser and pre-existing profiles never receive privileged writes or cleanup', async t => {
  for (const reason of ['browser', 'preexisting']) {
    const value = fixture(t);
    if (reason === 'browser') value.options.verify = async () => { throw Error('inventory mismatch'); };
    else value.state.installed = Buffer.from('pre-existing host profile');
    await assert.rejects(runSandbox('prepare', value.output, value.options));
    await runSandbox('cleanup', value.output, value.options);
    assert.equal(value.calls.length, 0);
    assert.equal(JSON.parse(readFileSync(resolve(value.output,'cleanup.json'))).owned, false);
    assert(!existsSync(resolve(value.output,'prepared.json')));
  }
});

test('failed profile loading retains command streams and still cleans the owned file', async t => {
  const value = fixture(t); value.state.failLoad = true;
  await assert.rejects(runSandbox('prepare', value.output, value.options));
  assert.equal(JSON.parse(readFileSync(resolve(value.output,'load-profile.json'))).status, 1);
  assert(existsSync(resolve(value.output,'load-profile.stdout')));
  assert(existsSync(resolve(value.output,'prepare-failure.json')));
  assert(!existsSync(resolve(value.output,'prepared.json')));
  await assert.rejects(runSandbox('cleanup', value.output, value.options));
  assert.equal(value.state.installed, null);
  assert(!value.calls.some(call => call.args.includes('-R')), 'failed add never unloads a pre-existing kernel profile');
  assert(!existsSync(resolve(value.output,'cleanup.json')));
});

test('global policy mutation and altered owned-profile bytes fail without a false receipt', async t => {
  const value = fixture(t); let observations = 0;
  value.state.policy = () => ({ ...restricted, restrictedUserns: ++observations === 2 ? '0' : '1' });
  await assert.rejects(runSandbox('prepare', value.output, value.options));
  assert(!existsSync(resolve(value.output,'prepared.json')));
  const calls = value.calls.length; value.state.installed = Buffer.from('other root-owned profile');
  await assert.rejects(runSandbox('cleanup', value.output, value.options));
  assert.equal(value.calls.length, calls); assert(existsSync(resolve(value.output,'cleanup-failure.json')));
});

test('failed kernel unload still removes only the owned file and never reports cleanup success', async t => {
  const value = fixture(t);
  await runSandbox('prepare', value.output, value.options); value.state.failUnload = true;
  await assert.rejects(runSandbox('cleanup', value.output, value.options));
  assert.equal(value.state.installed, null);
  assert.equal(JSON.parse(readFileSync(resolve(value.output,'unload-profile.json'))).status, 1);
  assert(existsSync(resolve(value.output,'remove-profile.json')));
  assert(!existsSync(resolve(value.output,'cleanup.json')));
});

test('missing owned file cannot claim kernel cleanup', async t => {
  const value = fixture(t);
  await runSandbox('prepare', value.output, value.options);
  value.state.installed = null;
  await assert.rejects(runSandbox('cleanup', value.output, value.options));
  assert.equal(value.calls.length, 2);
  assert(!existsSync(resolve(value.output,'cleanup.json')));
});

test('unknown interrupted creation preserves the file and cannot mint ownership or cleanup', async t => {
  const value = fixture(t); value.state.interruptCreate = true;
  await assert.rejects(runSandbox('prepare', value.output, value.options));
  assert(!existsSync(resolve(value.output,'owned.json')));
  await assert.rejects(runSandbox('cleanup', value.output, value.options));
  assert(value.state.installed !== null); assert.equal(value.calls.length, 1);
  assert(!existsSync(resolve(value.output,'cleanup.json')));
});

test('missing terminal add evidence never becomes successful kernel cleanup', async t => {
  const value = fixture(t);
  await runSandbox('prepare', value.output, value.options);
  unlinkSync(resolve(value.output,'load-profile.json'));
  await assert.rejects(runSandbox('cleanup', value.output, value.options));
  assert.equal(value.state.installed, null);
  assert(!value.calls.some(call => call.args.includes('-R')));
  assert(!existsSync(resolve(value.output,'cleanup.json')));
});
