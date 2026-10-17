// Joins browser evidence to authenticated host bytes; no request or token authority is created.
import { fail as reject, integer, bounds } from './wire.mjs';
import { validateCompletionLedger, completionProof } from './completion-record.mjs';
import { inspectCorpus } from './corpus-observation.mjs';
import { qualificationCases, scalarCases } from './browser-oracles.mjs';

const fail = () => reject('COMPLETION_UNPROVEN');
const api = ['session', 'compile', 'evaluate', 'finish', 'cancel'];
const same = (left, right) => {
  if (!left || typeof left !== 'object' || Array.isArray(left) !== Array.isArray(right) ||
      Object.keys(left).sort().join(',') !== Object.keys(right).sort().join(',') ||
      Object.keys(right).some(key => !Object.is(left[key], right[key]))) fail();
};
const publication = (kind, caseIndex = null, scalarIndex = null) => ({ kind, caseIndex, scalarIndex });

export function inspectCompletionLedger(value, { audit, trace, corpus, abort, initial, state }) {
  const ledger = validateCompletionLedger(value);
  // A publication reference alone supplies no DOM/report/scalar/AX authority.
  inspectCorpus(corpus, trace);
  if (!Array.isArray(audit) || audit.length > bounds.requests || !trace.cleanupAcknowledged) fail();
  const auditIds = new Set(), actual = new Map(), keys = new Set();
  for (const row of audit) {
    integer(row.id, bounds.requests);if (auditIds.has(row.id)) fail();auditIds.add(row.id);
    if (!api.includes(row.route)) continue;
    const method = row.route === 'session' ? 'GET' : 'POST';
    const key = JSON.stringify([row.route, method, row.requestBytes, row.requestSha256]);
    if (keys.has(key)) fail();keys.add(key);actual.set(row.id, row);
  }
  if (actual.size !== 45 || ledger.rows.length !== actual.size) fail();
  const rows = new Map();
  for (const row of ledger.rows) {
    if (rows.has(row.id)) fail();rows.set(row.id, row);
    const host = actual.get(row.id);
    if (!host || host.route !== row.route || host.requestBytes !== row.requestBytes ||
        host.requestSha256 !== row.requestSha256) fail();
    if (host.aborted === true) {
      if (host.dispatched !== false || row.id !== abort.id || row.route !== 'compile' ||
          row.network !== 'INITIAL_PROBE_ABORT' || row.response !== null || row.writeComplete !== false ||
          row.requestBytes !== abort.requestBytes || row.requestSha256 !== abort.requestSha256) fail();
    } else {
      if (host.method !== row.method || host.status !== 200 || row.browserStatus !== 200 ||
          row.writeComplete !== true || !['FINISHED', 'ERR_ABORTED'].includes(row.network)) fail();
      same(row.response, { status: host.status, bytes: host.responseBytes, sha256: host.responseSha256 });
    }
  }
  const used = new Set();let previousId = 0, previousOrdinal = 0;
  function take(id, route, expected) {
    integer(id, bounds.requests);const row = rows.get(id);
    if (!row || row.route !== route || used.has(id) || id <= previousId || row.ordinal <= previousOrdinal) fail();
    used.add(id);previousId = id;previousOrdinal = row.ordinal;
    same(row.publication, expected);return row;
  }
  const startup = take(trace.sessionRequestId, 'session', publication('STARTUP'));
  if (startup.finishId !== null || startup.requestBytes !== 0 || !initial.hashRemoved ||
      initial.pathname !== '/examples/playground/restricted/' ||
      initial.status !== 'Source edited. Compile this revision.') fail();
  const held = take(abort.id, 'compile', publication('INITIAL_ABORT'));
  if (held.finishId !== null || abort.route !== 'compile' || !abort.paused || !abort.acknowledged ||
      abort.event !== 'requestfailed' || !abort.httpResponseClosed) fail();
  const cancelled = take(trace.cancelRequestId, 'cancel', publication('INITIAL_CANCEL'));
  if (cancelled.finishId !== null || state.status !== 'Cancelled. The previous operation has finished cleanup.' ||
      state.compileDisabled !== false || state.runDisabled !== true || state.diagnosticChildren !== 0) fail();
  let scalarIndex = 0;
  function operation(record, route, reference) {
    const row = take(record.requestId, route, reference);
    if (row.finishId !== record.finishId || row.id >= row.finishId) fail();
    const finish = take(record.finishId, 'finish', publication('FINISH', reference.caseIndex, reference.scalarIndex));
    if (finish.finishId !== null || row.ordinal >= finish.ordinal) fail();
  }
  if (trace.compilations.length !== qualificationCases.length || trace.evaluations.length !== 10) fail();
  for (const [caseIndex, expected] of qualificationCases.entries()) {
    const compiled = trace.compilations[caseIndex];
    if (compiled.id !== expected.id) fail();
    operation(compiled, 'compile', publication('COMPILE', caseIndex));
    for (const selected of scalarCases(expected)) {
      const evaluated = trace.evaluations[scalarIndex];
      if (!evaluated || evaluated.id !== expected.id || evaluated.revision !== compiled.revision ||
          evaluated.componentSha256 !== compiled.identity?.componentSha256 ||
          evaluated.logical !== selected.logical) fail();
      same(evaluated.args, selected.args);
      operation(evaluated, 'evaluate', publication('EVALUATE', caseIndex, scalarIndex++));
    }
  }
  if (scalarIndex !== 10 || used.size !== 45 || used.size !== rows.size) fail();
  const observedApiAborts = ledger.rows.filter(row => row.network === 'ERR_ABORTED').length;
  if ((ledger.firstProvisional === null) !== (observedApiAborts === 0)) fail();
  if (ledger.firstProvisional !== null) {
    const anchor = ledger.firstProvisional, row = rows.get(anchor.id);
    if (!row || row.ordinal !== anchor.ordinal || row.route !== anchor.route || row.network !== 'ERR_ABORTED') fail();
  }
  return Object.freeze({ proof: completionProof, apiRows: used.size, pairedFinishes: 21,
    observedApiAborts });
}
