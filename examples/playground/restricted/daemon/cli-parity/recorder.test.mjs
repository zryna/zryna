import assert from 'node:assert/strict';
import test from 'node:test';
import { createParityRecorder } from './recorder.mjs';
import { createParityPlan, fixedCorpus } from './plan.mjs';
import { authority, witSha256, cliCapture, compilation, observation } from './fixture.test-support.mjs';
const recorder = () => createParityRecorder({ authority, witSha256 });

test('fixed request revision, frame limit and closed hashed receipt reject before recorder admission', async () => {
  const r = recorder();
  await assert.rejects(r.compilation('addition', await compilation('addition', 2)), /CLI-PARITY/);
  const value = await compilation('addition'); value.receipt.revision = 2;
  await assert.rejects(r.compilation('addition', value), /CLI-PARITY/);
  value.receipt.revision = 1; value.receipt.policySha256 = 'wrong';
  await assert.rejects(r.compilation('addition', value), /CLI-PARITY/);
  value.frameBytes = new Uint8Array(1277957);
  await assert.rejects(r.compilation('addition', value), /CLI-PARITY/);
  await assert.rejects(r.compilation('unknown', {}), /CLI-PARITY/);
  assert.throws(() => r.finish(), /CLI-PARITY/);
});
test('duplicates, early/stale/wrong scalar observations, concurrent compilations and abort cannot finish', async () => {
  const r = recorder(); const capture = await compilation('addition');
  const pending = r.compilation('addition', capture);
  await assert.rejects(r.compilation('addition', capture), /CLI-PARITY/); await pending;
  r.cli('build-addition', cliCapture(createParityPlan().operations[0]));
  assert.throws(() => r.cli('build-addition', cliCapture(createParityPlan().operations[0])), /CLI-PARITY/);
  assert.throws(() => r.observation('run-edited-literal-0', {}), /CLI-PARITY/);
  const obs = observation('run-addition-0', '0'.repeat(64));
  assert.throws(() => r.observation('run-addition-0', obs), /PLAYGROUND-/);
  r.abort(); assert.throws(() => r.finish(), /CLI-PARITY/);
  await assert.rejects(r.compilation('edited-literal', await compilation('edited-literal')), /CLI-PARITY/);
});
async function fill(r, wrongDiagnostic = false) {
  const plan = createParityPlan();
  for (const entry of fixedCorpus) {
    const value = await compilation(entry.id); await r.compilation(entry.id, value);
    const header = JSON.parse(Buffer.from(value.frameBytes).subarray(4, 4 + new DataView(value.frameBytes.buffer,
      value.frameBytes.byteOffset).getUint32(0, true)).toString());
    for (const operation of plan.operations.filter(item => item.corpusId === entry.id)) {
      const captured = cliCapture(operation);
      if (wrongDiagnostic && entry.id === 'any-rejected') {
        const json = JSON.parse(captured.stdout);
        if (wrongDiagnostic === 'duplicate') json.diagnostics.push(structuredClone(json.diagnostics[0]));
        else if (wrongDiagnostic === 'span') json.diagnostics[0].primary = { kind: 'source', span: { file: 0, start: 0, end: 2 } };
        else json.diagnostics[0].message = 'Mismatched authoritative diagnostic';
        captured.stdout = Buffer.from(JSON.stringify(json));
      }
      r.cli(operation.id, captured);
      if (operation.kind === 'run') r.observation(operation.id, observation(operation.id, header.identity.componentSha256));
    }
  }
}
test('all 19 synthetic rows compare exact diagnostic text and scalar identity while denying execution acceptance', async () => {
  const r = recorder(); await fill(r); const result = r.finish();
  assert.equal(result.rows.length, 19); assert.equal(result.comparedScalarObservations, 10);
  assert.equal(result.acceptance, false); assert.equal(result.executionEvidenceRequired, true);
  assert.equal(result.scope, 'comparison-only-no-runtime-or-publication-attestation');
  assert.throws(() => r.finish(), /CLI-PARITY/);
  const mismatch = recorder(); await fill(mismatch, true);
  assert.throws(() => mismatch.finish(), /CLI-PARITY/);
});
test('structured projection preserves duplicate counts and valid changed spans for strict recorder rejection', async () => {
  for (const mutation of ['duplicate', 'span']) {
    const r = recorder(); await fill(r, mutation);
    assert.throws(() => r.finish(), /CLI-PARITY/);
  }
});
