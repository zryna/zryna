// Synthetic metadata only. These tests do not authenticate a run or execute any compiler/guest.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import test from 'node:test';
import { canonical } from '../scripts/stability-gates/input.mjs';
import { parseHostedPerformanceExpectations, validateHostedPerformanceExpectations }
  from '../scripts/stability-gates/cli-performance-hosted-expectations.mjs';

import { platformFixture } from './cli-performance-platform-fixture.mjs';

const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const encode = value => Buffer.from(canonical(value));
const fixture = () => platformFixture().value;
function rejects(change) {
  const value = fixture(); validateHostedPerformanceExpectations(value);
  change(value); assert.throws(() => validateHostedPerformanceExpectations(value));
}
test('fixed four-job six-artifact expectations require an external byte digest and preserve input', () => {
  const value = fixture(), before = structuredClone(value), bytes = encode(value);
  assert.deepEqual(parseHostedPerformanceExpectations(bytes, digest(bytes)), value);
  assert.deepEqual(value, before);
  assert.equal(Object.hasOwn(value, 'sourceAuthenticated'), false);
  assert.equal(Object.hasOwn(value, 'thresholds'), false);
  assert.throws(() => parseHostedPerformanceExpectations(bytes));
  assert.throws(() => parseHostedPerformanceExpectations(bytes, '0'.repeat(64)));
});
test('schema rejects authority claims and wrong repository identity even if caller recomputes a hash', () => {
  for (const change of [v => { v.sourceAuthenticated = true; }, v => { v.thresholds = {}; },
    v => { v.repository = 'other/zryna'; }, v => { v.repositoryId = 1350402317; },
    v => { v.repositoryId = '1350402316'; }, v => { v.source.role = 'approved'; }]) rejects(change);
  const value = fixture(); value.reviewedSha256 = 'a'.repeat(64); const bytes = encode(value);
  assert.throws(() => parseHostedPerformanceExpectations(bytes, digest(bytes)));
});
test('missing duplicate swapped or partial job roles cannot provide complete evidence', () => {
  for (const change of [v => { v.jobs.pop(); }, v => { v.jobs.push(structuredClone(v.jobs[0])); },
    v => { v.jobs[1] = structuredClone(v.jobs[0]); }, v => { v.jobs[1].jobId = v.jobs[0].jobId; },
    v => { v.jobs[1].checkRunId = v.jobs[0].checkRunId; },
    v => { v.jobs[0].round = 'held-out-candidate'; }, v => { v.jobs[0].host = 'linux-arm64'; }]) rejects(change);
});
test('primary/provider IDs names run attempts and size bounds cannot be substituted', () => {
  for (const change of [v => { v.jobs[0].provider = null; },
    v => { v.jobs[2].provider = structuredClone(v.jobs[0].provider); },
    v => { v.jobs[1].provider.id = v.jobs[0].primary.id; },
    v => { v.jobs[1].primary.name = v.jobs[0].primary.name; },
    v => { v.jobs[0].primary.name = 'cli-performance-ubuntu-24.04-baseline-round-1-100-2'; },
    v => { v.jobs[0].primary.bytes = 128 * 1024 * 1024 + 1; },
    v => { v.jobs[0].provider.bytes = 1024 * 1024 + 1; },
    v => { v.jobs[0].primary.id = true; }, v => { v.jobs[0].primary.sha256 = 'bad'; },
    v => { v.jobs[0].primary.expiresAt = '2026-02-30T00:00:00Z'; }]) rejects(change);
});
test('retry selection and invalid source or execution identities reject', () => {
  for (const change of [v => { v.run.attempt = 2; }, v => { v.run.id = 0; },
    v => { v.run.id = Number.MAX_SAFE_INTEGER + 1; }, v => { v.run.event = 'workflow_run'; },
    v => { v.source.commit = 'f'.repeat(39); }, v => { v.execution.tree = '9'.repeat(40); },
    v => { v.execution.parents = [v.execution.commit]; }]) rejects(change);
  for (const ref of ['main', 'refs/tags/v1', 'refs/heads/fix//bad', 'refs/heads/fix/../bad',
    'refs/heads/fix/.hidden', 'refs/heads/branch.lock', 'refs/heads/end.', 'refs/heads/fix/$bad'])
    rejects(v => { v.source.ref = ref; });
});
test('PR checkout and platform execution retain distinct explicitly parented identities', () => {
  const value = fixture(); value.run.event = 'pull_request';
  value.run.pullRequest = 575;
  value.execution = { commit: '4'.repeat(40), tree: '5'.repeat(40), parents: ['3'.repeat(40), value.source.commit] };
  validateHostedPerformanceExpectations(value);
  for (const parents of [[], ['3'.repeat(40)], [value.source.commit, '3'.repeat(40)],
    [value.source.commit, value.source.commit]]) {
    assert.throws(() => validateHostedPerformanceExpectations({ ...value, execution: { ...value.execution, parents } }));
  }
  assert.throws(() => validateHostedPerformanceExpectations({ ...value,
    execution: { ...value.execution, commit: value.source.commit } }));
});
test('both reviewed workflow sources must be exact distinct bounded files', () => {
  for (const change of [v => { v.workflows.pop(); }, v => { v.workflows[1] = structuredClone(v.workflows[0]); },
    v => { v.workflows[0].path = '.github/workflows/other.yml'; },
    v => { v.workflows[0].bytes = 0; }, v => { v.workflows[0].bytes = 65537; },
    v => { v.workflows[0].sha256 = 'G'.repeat(64); }, v => { v.workflows[0].approved = true; }]) rejects(change);
});
test('external identity does not allow duplicate keys noncanonical JSON invalid UTF8 or oversized bytes', () => {
  const canonicalBytes = encode(fixture());
  const duplicate = Buffer.from(canonicalBytes.toString().replace('{', '{"format":"not-authority",'));
  for (const bytes of [duplicate, Buffer.from(JSON.stringify(fixture(), null, 2)),
    Buffer.from([0xff]), Buffer.alloc(256 * 1024 + 1)])
    assert.throws(() => parseHostedPerformanceExpectations(bytes, digest(bytes)));
});

test('exact declared archive limits and role reordering remain valid without changing selected evidence', () => {
  const value = fixture();
  value.jobs[0].primary.bytes = 128 * 1024 * 1024;
  value.jobs[0].provider.bytes = 1024 * 1024;
  value.jobs.reverse();
  validateHostedPerformanceExpectations(value);
  const bytes = encode(value);
  assert.deepEqual(parseHostedPerformanceExpectations(bytes, digest(bytes)), value);
});
