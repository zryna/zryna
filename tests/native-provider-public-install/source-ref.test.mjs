import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, realpathSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { selectNativeSourceRef } from '../../scripts/distribution/native-source-ref-v2.mjs';

const head = 'a'.repeat(40), ref = 'refs/heads/codex/native-install';
function withEvent(name, event, action) {
  const root = realpathSync.native(mkdtempSync(join(tmpdir(), 'zryna-native-source-ref-')));
  const path = join(root, 'event.json');
  writeFileSync(path, JSON.stringify({ repository: { full_name: 'zryna/zryna' }, ...event }));
  try {
    return action({ GITHUB_ACTIONS: 'true', GITHUB_REPOSITORY: 'zryna/zryna',
      GITHUB_EVENT_NAME: name, GITHUB_EVENT_PATH: path, GITHUB_SHA: head,
      GITHUB_REF: ref, GITHUB_HEAD_REF: 'codex/native-install' });
  } finally { rmSync(root, { recursive: true }); }
}
const pr = () => ({ pull_request: { head: { sha: head, ref: 'codex/native-install',
  repo: { full_name: 'zryna/zryna' } } } });

test('attached source observation records actual branch and cannot be overridden by CI labels', () => {
  assert.equal(selectNativeSourceRef({ head, attachedRef: ref, env: {} }).ref, ref);
  assert.throws(() => selectNativeSourceRef({ head, attachedRef: 'refs/heads/feature/x',
    env: { GITHUB_REF: 'refs/heads/main' } }), /actual attached source branch/);
  assert.throws(() => selectNativeSourceRef({ head, attachedRef: null, env: {} }), /actual GitHub event/);
});

test('detached PR source observation binds actual event head repository commit and branch', () => {
  withEvent('pull_request', pr(), env => {
    const observed = selectNativeSourceRef({ head, attachedRef: null, env });
    assert.equal(observed.ref, ref); assert.equal(observed.commit, head);
    assert.equal(observed.observation, 'actual-github-event-head');
    assert.equal(observed.event.name, 'pull_request'); assert.match(observed.event.sha256, /^[a-f0-9]{64}$/);
    for (const changed of [{ GITHUB_HEAD_REF: 'main' }, { GITHUB_REPOSITORY: 'foreign/repo' }]) {
      assert.throws(() => selectNativeSourceRef({ head, attachedRef: null, env: { ...env, ...changed } }));
    }
  });
  for (const change of [event => { event.pull_request.head.sha = 'b'.repeat(40); },
    event => { event.pull_request.head.repo.full_name = 'foreign/repo'; },
    event => { event.pull_request.head.ref = 'main'; },
    event => { event.repository = { full_name: 'foreign/repo' }; }]) {
    const event = pr(); change(event);
    withEvent('pull_request', event, env =>
      assert.throws(() => selectNativeSourceRef({ head, attachedRef: null, env })));
  }
});

test('detached push and dispatch source observations bind their exact selected commit/ref', () => {
  for (const [name, event] of [['push', { after: head, ref }], ['workflow_dispatch', { ref }]]) {
    withEvent(name, event, env => {
      assert.equal(selectNativeSourceRef({ head, attachedRef: null, env }).ref, ref);
      assert.throws(() => selectNativeSourceRef({ head: 'b'.repeat(40), attachedRef: null, env }));
      assert.throws(() => selectNativeSourceRef({ head, attachedRef: null,
        env: { ...env, GITHUB_REF: 'refs/pull/414/merge' } }));
    });
  }
  withEvent('schedule', {}, env =>
    assert.throws(() => selectNativeSourceRef({ head, attachedRef: null, env }), /event kind unsupported/));
  withEvent('push', { after: head, ref: 'refs/tags/v0.2.3' }, env =>
    assert.throws(() => selectNativeSourceRef({ head, attachedRef: null,
      env: { ...env, GITHUB_REF: 'refs/tags/v0.2.3' } }), /actual detached source branch/));
});

test('actual fix branch observations preserve detached source proof and cannot relabel another head', () => {
  const fixed = 'refs/heads/fix/bounded-generics-profile';
  const branch = fixed.slice('refs/heads/'.length);
  const attached = selectNativeSourceRef({ head, attachedRef: fixed,
    env: { GITHUB_REF: 'refs/heads/main' } });
  assert.equal(attached.ref, fixed); assert.equal(attached.commit, head);
  assert.equal(attached.observation, 'actual-git-symbolic-ref');
  const event = pr(); event.pull_request.head.ref = branch;
  withEvent('pull_request', event, env => {
    const actual = { ...env, GITHUB_REF: 'refs/pull/572/merge', GITHUB_HEAD_REF: branch };
    const observed = selectNativeSourceRef({ head, attachedRef: null, env: actual });
    assert.equal(observed.ref, fixed); assert.equal(observed.commit, head);
    assert.equal(observed.observation, 'actual-github-event-head');
    assert.equal(observed.event.name, 'pull_request');
    for (const changed of [
      { GITHUB_HEAD_REF: 'main' }, { GITHUB_HEAD_REF: 'codex/native-install' },
      { GITHUB_REPOSITORY: 'foreign/repo' }, { GITHUB_ACTIONS: 'false' },
    ]) assert.throws(() => selectNativeSourceRef({ head, attachedRef: null,
      env: { ...actual, ...changed } }));
    assert.throws(() => selectNativeSourceRef({ head: 'b'.repeat(40), attachedRef: null, env: actual }));
  });
  for (const [name, event] of [['push', { after: head, ref: fixed }],
    ['workflow_dispatch', { ref: fixed }]]) {
    withEvent(name, event, env => {
      const actual = { ...env, GITHUB_REF: fixed };
      assert.equal(selectNativeSourceRef({ head, attachedRef: null, env: actual }).ref, fixed);
      assert.throws(() => selectNativeSourceRef({ head: 'b'.repeat(40), attachedRef: null, env: actual }));
      assert.throws(() => selectNativeSourceRef({ head, attachedRef: null,
        env: { ...actual, GITHUB_REF: 'refs/heads/main' } }));
    });
  }
  for (const unsafe of ['refs/heads/feature/native', 'refs/heads/fix/../main',
    'refs/heads/fix/a.lock', 'refs/tags/v0.2.3']) {
    assert.throws(() => selectNativeSourceRef({ head, attachedRef: unsafe,
      env: { GITHUB_REF: fixed } }), /actual attached source branch/);
  }
});
