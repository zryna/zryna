import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { registry, requiredGates } from '../scripts/m6/registry.mjs';
import { requiredSourceGates, sourceRegistry } from '../scripts/m6/source-registry.mjs';

const digest = bytes => createHash('sha256').update(bytes).digest('hex');

test('source route preserves all 23 platform gates with an explicit publication successor', () => {
  assert.equal(requiredSourceGates.length, 23);
  assert.deepEqual(requiredSourceGates, requiredGates);
  assert.equal(sourceRegistry.version, 2);
  assert.equal(sourceRegistry.route, 'reviewed-source-v1');
  assert.deepEqual(sourceRegistry.supersession.map(({ gate, previousCase, case: id }) =>
    [gate, previousCase, id]), [['publication-linux', 'outer-signature', 'published-compiler-signature']]);
  for (const original of registry.gates) {
    const current = sourceRegistry.gates.find(gate => gate.id === original.id);
    assert.deepEqual(current.platforms, original.platforms);
    assert.deepEqual(current.cases, original.cases.map(id => original.id === 'publication' &&
      id === 'outer-signature' ? 'published-compiler-signature' : id));
  }
  assert.ok(sourceRegistry.gates.find(gate => gate.id === 'publication')
    .artifactRoles.includes('playground-compiler'));
  assert.equal(Object.isFrozen(sourceRegistry.gates[0].cases), true);
  assert.ok(!sourceRegistry.requiredArtifacts.includes('playground-toolkit'));
  assert.ok(registry.requiredArtifacts.includes('playground-toolkit'));
});

test('reviewed role policies deny kernel authority and compiler network/namespace creation', async () => {
  const pins = { compiler: 'bf4a72a0da350e14260d8b1534967de60bb40dff11f088c0eeae3e455c74c33b',
    evaluator: '05b2215822405c5d346b678b9a8d37e9d8642083138c5649420cc46c0ccf8641' };
  for (const [role, expected] of Object.entries(pins)) {
    const bytes = await readFile(new URL(`../examples/playground/restricted/daemon/policies/${role}-amd64.json`, import.meta.url));
    assert.equal(digest(bytes), expected);
    const profile = JSON.parse(bytes);
    assert.equal(profile.defaultAction, 'SCMP_ACT_ERRNO');
    assert.equal(profile.defaultErrnoRet, 1);
    assert.deepEqual(profile.architectures, ['SCMP_ARCH_X86_64']);
    const allowed = profile.syscalls.filter(rule => rule.action === 'SCMP_ACT_ALLOW');
    for (const name of ['ptrace', 'bpf', 'process_vm_readv', 'process_vm_writev', 'keyctl',
      'mount', 'setns', 'reboot', 'open_by_handle_at', 'name_to_handle_at']) {
      assert.equal(allowed.some(rule => rule.names.includes(name)), false, `${role}: ${name}`);
    }
    const pair = allowed.filter(rule => rule.names.includes('socketpair'));
    assert.deepEqual(pair, [{ action: 'SCMP_ACT_ALLOW', args: [{ index: 0,
      op: 'SCMP_CMP_EQ', value: 1 }], names: ['socketpair'] }]);
    if (role === 'compiler') {
      for (const name of ['socket', 'connect', 'bind', 'listen', 'accept', 'accept4', 'unshare', 'chroot']) {
        assert.equal(allowed.some(rule => rule.names.includes(name)), false, name);
      }
      assert.equal(allowed.filter(rule => rule.names.includes('clone')).length, 1);
    } else {
      assert.deepEqual(allowed.filter(rule => rule.names.includes('socket'))
        .map(rule => rule.args[0].value), [1, 2, 10, 16]);
      assert.deepEqual(allowed.filter(rule => rule.names.includes('unshare'))
        .map(rule => rule.args[0]), [{ index: 0, op: 'SCMP_CMP_EQ', value: 268435456 }]);
    }
  }
});
