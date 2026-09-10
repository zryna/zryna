import assert from 'node:assert/strict';
import test from 'node:test';
import { canonical, sha256 } from '../scripts/distribution-release/canonical.mjs';
import {
  validatePreassemblyGates,
  validatePreassemblyGatesText,
} from '../scripts/distribution-release/validate-preassembly-gates.mjs';

const COMMIT = 'a'.repeat(40);
const jobs = [
  'adapter', 'm0', 'm2', 'm3', 'rust (ubuntu-latest)', 'rust (windows-latest)',
].map((name) => ({ name, conclusion: 'success', sourceCommit: COMMIT }));

function fixture() {
  const document = {
    format: 'zryna.preassembly-gates.v1',
    repository: 'https://github.com/zryna/zryna',
    workflow: '.github/workflows/ci.yml',
    runId: '123456789',
    runAttempt: 1,
    runUrl: 'https://github.com/zryna/zryna/actions/runs/123456789',
    sourceCommit: COMMIT,
    requiredJobs: structuredClone(jobs),
  };
  const text = `${canonical(document)}\n`;
  const input = {
    source: { repository: document.repository, commit: COMMIT },
    gateReceipt: {
      format: document.format,
      workflow: document.workflow,
      runId: document.runId,
      runAttempt: document.runAttempt,
      runUrl: document.runUrl,
      sourceCommit: COMMIT,
      requiredJobs: structuredClone(jobs),
      size: Buffer.byteLength(text),
      sha256: sha256(text),
    },
  };
  return { document, input, text };
}

test('accepts exact parsed and canonical gate evidence', () => {
  const { document, input, text } = fixture();
  assert.equal(validatePreassemblyGates(document, input), document);
  assert.deepEqual(validatePreassemblyGatesText(text, input), document);
});

test('rejects source, run, and required-job projection drift', () => {
  for (const [code, mutate] of [
    ['R406-GATES-SOURCE', ({ document }) => { document.sourceCommit = 'b'.repeat(40); }],
    ['R406-GATES-RUN', ({ document }) => { document.runAttempt = 2; }],
    ['R406-GATES-RUN', ({ document, input }) => {
      document.runUrl = 'https://github.com/zryna/zryna/actions/runs/987654321';
      input.gateReceipt.runUrl = document.runUrl;
    }],
    ['R406-GATES-JOBS', ({ document }) => { document.requiredJobs.reverse(); }],
  ]) {
    const value = fixture();
    mutate(value);
    assert.throws(() => validatePreassemblyGates(value.document, value.input),
      new RegExp(`${code}:`));
  }
});

test('rejects wrong descriptors before parsing gate bytes', () => {
  for (const mutate of [
    ({ input }) => { input.gateReceipt.size += 1; },
    ({ input }) => { input.gateReceipt.sha256 = 'b'.repeat(64); },
  ]) {
    const value = fixture();
    mutate(value);
    assert.throws(() => validatePreassemblyGatesText(value.text, value.input),
      /R406-GATES-DIGEST:/);
  }
});

test('schema rejects false success and hosted metadata extensions', () => {
  for (const mutate of [
    ({ document }) => { document.requiredJobs[0].conclusion = 'skipped'; },
    ({ document }) => { document.runnerPath = 'C:\\runner'; },
  ]) {
    const value = fixture();
    mutate(value);
    assert.throws(() => validatePreassemblyGates(value.document, value.input),
      /R406-GATES-SCHEMA:/);
  }
});
