// Pure joins of synthetic owning-decoder fixtures, not browser or native acceptance.
import assert from 'node:assert/strict';
import test from 'node:test';
import { inspectCompletionLedger } from './completion-host.mjs';
import { completionFixture } from './completion-host-fixture.mjs';
import { completionProof } from './completion-record.mjs';

test('all45 actual-id fixture rows join original decoded corpus and21 own finish links without ordinal equality', async () => {
  const { ledger, evidence } = await completionFixture();
  assert.ok(ledger.rows.every(row => row.ordinal !== row.id));
  assert.deepEqual(inspectCompletionLedger(ledger, evidence), { proof: completionProof, apiRows: 45, pairedFinishes: 21, observedApiAborts: 0 });
  const copy = structuredClone(ledger);const row = copy.rows.find(row => row.route === 'finish');
  row.network = 'ERR_ABORTED';copy.firstProvisional = { ordinal: row.ordinal, id: row.id, route: row.route,
    sequence: 100, network: 'ERR_ABORTED', status: 200 };
  assert.equal(inspectCompletionLedger(copy, evidence).observedApiAborts, 1);
});
test('a coherent rekey cannot move the undispatched probe before authenticated session startup', async () => {
  const { ledger, evidence } = await completionFixture();
  for (const row of [...ledger.rows, ...evidence.audit]) {
    if (row.id === 1) row.id = 2;else if (row.id === 2) row.id = 1;
  }
  evidence.trace.sessionRequestId = 2;evidence.abort.id = 1;
  assert.throws(() => inspectCompletionLedger(ledger, evidence));
});
test('original preset export preference survives declaration-first export differing from selected export', async () => {
  const { ledger, evidence } = await completionFixture();
  const index = evidence.trace.compilations.findIndex(row => row.id === 'multiple-exports');
  assert.equal(evidence.trace.compilations[index].exports[0].logical, 'zero');
  assert.equal(evidence.corpus.rows[index].controls.selectedExport, 'constant');
  assert.doesNotThrow(() => inspectCompletionLedger(ledger, evidence));
});

const mutations = {
  'unknown broker id': f => { f.ledger.rows[3].id = 200; },
  'browser ordinal borrowed as host id': f => { f.ledger.rows[0].id = f.ledger.rows[0].ordinal; },
  'missing actual API response': f => { f.evidence.audit.pop(); },
  'extra replayed API response': f => { f.evidence.audit.push({ ...f.evidence.audit[0], id: 200 }); },
  'same actual id appears twice': f => { f.evidence.audit.push({ ...f.evidence.audit[0] }); },
  'host request hash differs': f => { f.evidence.audit[3].requestSha256 = 'f'.repeat(64); },
  'host request count differs': f => { f.evidence.audit[3].requestBytes++; },
  'host response hash differs': f => { f.evidence.audit[3].responseSha256 = 'f'.repeat(64); },
  'host response count differs': f => { f.evidence.audit[3].responseBytes++; },
  'host status not200': f => { f.evidence.audit[3].status = 400; },
  'host method differs': f => { f.evidence.audit[3].method = 'GET'; },
  'held compile actually dispatched': f => { f.evidence.audit[1].dispatched = true; },
  'held compile abort id borrowed': f => { f.evidence.abort.id = 4; },
  'held compile lacks HTTP close': f => { f.evidence.abort.httpResponseClosed = false; },
  'initial cancel missing trace': f => { f.evidence.trace.cancelRequestId = null; },
  'cancelled DOM used without acknowledgement state': f => { f.evidence.state.compileDisabled = true; },
  'session mounting not reached': f => { f.evidence.initial.status = 'Connecting to the supervised compiler…'; },
  'session matched to another API': f => { f.evidence.trace.sessionRequestId = 4; },
  'two operations borrow same finish': f => { f.evidence.trace.evaluations[0].finishId = f.evidence.trace.compilations[0].finishId; },
  'finish link differs from actual trace': f => { f.ledger.rows[3].finishId = 7; },
  'source revision differs': f => { f.evidence.trace.evaluations[0].revision++; },
  'component differs': f => { f.evidence.trace.evaluations[0].componentSha256 = 'f'.repeat(64); },
  'selected arguments differ': f => { f.evidence.trace.evaluations[0].args = [0, 0]; },
  'DOM diagnostic differs': f => { f.evidence.corpus.rows[4].diagnostics[0][0].text = 'wrong'; },
  'DOM scalar differs': f => { f.evidence.corpus.rows[0].observations[0].text = 'wrong'; },
  'DOM source replaced': f => { f.evidence.corpus.rows[0].source = 'foreign'; },
  'actual selected export differs from original preset preference': f => { f.evidence.corpus.rows[3].controls.selectedExport = 'zero'; },
  'compile remains disabled at publication': f => { f.evidence.corpus.rows[0].controls.compileDisabled = true; },
  'cancel remains enabled at publication': f => { f.evidence.corpus.rows[0].controls.cancelDisabled = false; },
  'observed selection source is stale': f => { f.evidence.corpus.rows[0].observations[0].selection.source = 'stale'; },
  'observed selected logical export is wrong': f => { f.evidence.corpus.rows[0].observations[0].selection.logical = 'zero'; },
  'observed argument input strings are noncanonical': f => { f.evidence.corpus.rows[0].observations[0].selection.args[0] = '020'; },
  'evaluation publication status is stale': f => { f.evidence.corpus.rows[0].observations[0].selection.status = 'Running export…'; },
  'evaluation run control remains disabled': f => { f.evidence.corpus.rows[0].observations[0].selection.runDisabled = true; },
  'source publication case substituted': f => { f.ledger.rows[3].publication.caseIndex = 1; },
  'scalar publication index substituted': f => { f.ledger.rows[5].publication.scalarIndex = 9; },
  'paired finish publication substituted': f => { f.ledger.rows[4].publication.caseIndex = 1; },
  'complete response without own local write': f => { f.ledger.rows[3].writeComplete = false; },
  'request not terminal': f => { f.ledger.rows[3].network = 'UNSEEN'; },
  'candidate without actual same-object200': f => { f.ledger.rows[3].browserStatus = null; },
  'unresolved candidate': f => { f.ledger.unresolvedOrdinals = [f.ledger.rows[3].ordinal]; },
  'unmatched local job': f => { f.ledger.unmatchedLocalIds = [4]; },
  'partial ledger marked truncated': f => { f.ledger.failureEvidenceTruncated = true; },
  'unconfirmed host cleanup': f => { f.evidence.trace.cleanupAcknowledged = false; },
  'unknown primary metadata smuggled': f => { f.ledger.rows[3].raw = 'private'; },
};
for (const [name, mutate] of Object.entries(mutations)) test(name + ' cannot establish completion', async () => {
  const fixture = await completionFixture();
  assert.doesNotThrow(() => inspectCompletionLedger(fixture.ledger, fixture.evidence), 'unmodified baseline must be valid before mutation');
  mutate(fixture);
  assert.throws(() => inspectCompletionLedger(fixture.ledger, fixture.evidence));
});
