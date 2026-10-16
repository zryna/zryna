// Synthetic fixture joins only; every frame still passes the original owning decoder.
import { completeFixture, domFixture, compilation, json } from './corpus-fixture.mjs';
import { qualificationCases, scalarCases } from './browser-oracles.mjs';
import { sha256 } from './wire.mjs';

const publication = (kind, caseIndex = null, scalarIndex = null) => ({ kind, caseIndex, scalarIndex });
export async function completionFixture() {
  const trace = await completeFixture(), audit = [], rows = [];
  function add(id, route, request, body, reference, finishId = null) {
    const input = request === null ? Buffer.alloc(0) : json(request);
    const method = route === 'session' ? 'GET' : 'POST';
    const record = { id, route, method, requestBytes: input.length, requestSha256: sha256(input),
      status: 200, responseBytes: body.length, responseSha256: sha256(body) };
    audit.push(record);rows.push({ ordinal: id + 30, id, route, method,
      requestBytes: record.requestBytes, requestSha256: record.requestSha256, browserStatus: 200,
      response: { status: 200, bytes: body.length, sha256: record.responseSha256 }, network: 'FINISHED',
      writeComplete: true, publication: reference, finishId });
  }
  add(trace.sessionRequestId, 'session', null, json({ version: 2, evaluationMode: 'supervised-host',
    authority: trace.authority, witSha256: trace.witSha256, template: Buffer.from('inert').toString('base64') }), publication('STARTUP'));
  const held = json({ version: 1, revision: 1, source: qualificationCases[0].source });
  audit.push({ id: 2, route: 'compile', aborted: true, dispatched: false, requestBytes: held.length, requestSha256: sha256(held) });
  rows.push({ ordinal: 32, id: 2, route: 'compile', method: 'POST', requestBytes: held.length,
    requestSha256: sha256(held), browserStatus: null, response: null, network: 'INITIAL_PROBE_ABORT',
    writeComplete: false, publication: publication('INITIAL_ABORT'), finishId: null });
  add(trace.cancelRequestId, 'cancel', { cancelled: true }, json({ finished: true }), publication('INITIAL_CANCEL'));
  let scalarIndex = 0;
  for (const [caseIndex, expected] of qualificationCases.entries()) {
    const compiled = trace.compilations[caseIndex], frame = compilation(expected, compiled.revision);
    const token = frame.headers.find(row => row.name === 'x-zryna-job').value;
    add(compiled.requestId, 'compile', { version: 1, revision: compiled.revision, source: expected.source },
      frame.body, publication('COMPILE', caseIndex), compiled.finishId);
    add(compiled.finishId, 'finish', { token, cancelled: false }, json({ finished: true }), publication('FINISH', caseIndex));
    for (const selected of scalarCases(expected)) {
      const evaluated = trace.evaluations[scalarIndex];
      const input = { revision: compiled.revision, sourceSha256: compiled.sourceSha256,
        componentSha256: compiled.identity.componentSha256, logical: selected.logical, args: selected.args };
      const ownToken = sha256(Buffer.from('synthetic-evaluation-' + input.revision + '-' + input.logical + '-' + input.args.join(',')));
      add(evaluated.requestId, 'evaluate', input, json({ token: ownToken, observation: {
        revision: compiled.revision, logical: selected.logical, componentSha256: compiled.identity.componentSha256, value: selected.value } }),
      publication('EVALUATE', caseIndex, scalarIndex), evaluated.finishId);
      add(evaluated.finishId, 'finish', { token: ownToken, cancelled: false }, json({ finished: true }), publication('FINISH', caseIndex, scalarIndex++));
    }
  }
  rows.sort((a, b) => a.ordinal - b.ordinal);audit.sort((a, b) => a.id - b.id);
  const ledger = { format: 'zryna.ui-api-completion-ledger.v1', sealed: true, rows, firstProvisional: null,
    unresolvedOrdinals: [], unmatchedLocalIds: [], failureEvidenceTruncated: false };
  return { ledger, evidence: { audit, trace, corpus: domFixture(trace),
    abort: { id: 2, route: 'compile', paused: true, acknowledged: true, event: 'requestfailed',
      httpResponseClosed: true, requestBytes: held.length, requestSha256: sha256(held) },
    initial: { hashRemoved: true, pathname: '/examples/playground/restricted/', status: 'Source edited. Compile this revision.' },
    state: { status: 'Cancelled. The previous operation has finished cleanup.', compileDisabled: false, runDisabled: true, diagnosticChildren: 0 } } };
}
