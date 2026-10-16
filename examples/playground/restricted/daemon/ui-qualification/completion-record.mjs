// Closed, token-free evidence. Publication is a source implication, not a sampled reader state.
import { exact, fail, hash, bounds } from './wire.mjs';
export const completionProof = 'ORIGINAL_SOURCE_COMPLETION_IMPLICATION';
export const completionRoutes = Object.freeze(['session', 'compile', 'evaluate', 'finish', 'cancel']);
const check = value => { if (!value) fail('COMPLETION_UNPROVEN'); };
const number = (value, low, high) => Number.isSafeInteger(value) && !Object.is(value, -0) && value >= low && value <= high;
const id = value => number(value, 1, 256);
const input = { session: 0, compile: 32768, evaluate: 8192, finish: 1024, cancel: 1024 };
const output = { session: 98304, compile: 1277956, evaluate: 2048, finish: 1024, cancel: 1024 };
export function validatePublication(value) {
  exact(value, ['kind', 'caseIndex', 'scalarIndex']);
  check(['STARTUP', 'INITIAL_ABORT', 'INITIAL_CANCEL', 'COMPILE', 'EVALUATE', 'FINISH'].includes(value.kind));
  if (['STARTUP', 'INITIAL_ABORT', 'INITIAL_CANCEL'].includes(value.kind)) check(value.caseIndex === null && value.scalarIndex === null);
  else check(number(value.caseIndex, 0, 10) && (value.kind === 'COMPILE' ? value.scalarIndex === null :
    value.kind === 'EVALUATE' ? number(value.scalarIndex, 0, 9) : value.scalarIndex === null || number(value.scalarIndex, 0, 9)));
}
export function validateCompletionSummary(value) {
  if (value === null) return;
  exact(value, ['ordinal', 'id', 'route', 'sequence', 'network', 'status']);
  check(id(value.ordinal) && (value.id === null || id(value.id)) && completionRoutes.includes(value.route) &&
    number(value.sequence, 1, 1024) && value.network === 'ERR_ABORTED' && value.status === 200);
}
function ids(value) {
  check(Array.isArray(value) && value.length <= 256 && value.every(id) &&
    value.every((entry, index) => index === 0 || entry > value[index - 1]));
}
function row(value) {
  exact(value, ['ordinal', 'id', 'route', 'method', 'requestBytes', 'requestSha256', 'browserStatus', 'response',
    'network', 'writeComplete', 'publication', 'finishId']);
  check(id(value.ordinal) && id(value.id) && completionRoutes.includes(value.route) &&
    value.method === (value.route === 'session' ? 'GET' : 'POST') && number(value.requestBytes, 0, input[value.route]) && hash(value.requestSha256));
  validatePublication(value.publication);
  const held = value.publication.kind === 'INITIAL_ABORT';
  check(held ? value.route === 'compile' && value.browserStatus === null && value.response === null &&
    value.network === 'INITIAL_PROBE_ABORT' && value.writeComplete === false :
    value.browserStatus === 200 && ['FINISHED', 'ERR_ABORTED'].includes(value.network) && value.writeComplete === true);
  if (!held) { exact(value.response, ['status', 'bytes', 'sha256']);check(value.response.status === 200 &&
    number(value.response.bytes, 0, output[value.route]) && hash(value.response.sha256)); }
  const kinds = { session: ['STARTUP'], compile: ['COMPILE', 'INITIAL_ABORT'], evaluate: ['EVALUATE'], finish: ['FINISH'], cancel: ['INITIAL_CANCEL'] };
  check(kinds[value.route].includes(value.publication.kind));
  check(['COMPILE', 'EVALUATE'].includes(value.publication.kind) ? id(value.finishId) : value.finishId === null);
}
export function validateCompletionLedger(value) {
  exact(value, ['format', 'sealed', 'rows', 'firstProvisional', 'unresolvedOrdinals', 'unmatchedLocalIds', 'failureEvidenceTruncated']);
  check(value.format === 'zryna.ui-api-completion-ledger.v1' && value.sealed === true && value.failureEvidenceTruncated === false &&
    Array.isArray(value.rows) && value.rows.length === 45);
  ids(value.unresolvedOrdinals);ids(value.unmatchedLocalIds);check(!value.unresolvedOrdinals.length && !value.unmatchedLocalIds.length);
  validateCompletionSummary(value.firstProvisional);
  const ordinals = new Set(), localIds = new Set(), keys = new Set(), counts = {};
  for (const [index, entry] of value.rows.entries()) {
    row(entry);check(!ordinals.has(entry.ordinal) && !localIds.has(entry.id) && (index === 0 || entry.ordinal > value.rows[index - 1].ordinal));
    const key = JSON.stringify([entry.route, entry.method, entry.requestBytes, entry.requestSha256]);check(!keys.has(key));
    keys.add(key);ordinals.add(entry.ordinal);localIds.add(entry.id);counts[entry.route] = (counts[entry.route] ?? 0) + 1;
  }
  check(counts.session === 1 && counts.compile === 12 && counts.evaluate === 10 && counts.finish === 21 && counts.cancel === 1);
  const finishes = new Set(), compiles = new Set(), scalars = new Set();
  for (const entry of value.rows.filter(entry => entry.finishId !== null)) {
    const finish = value.rows.find(other => other.id === entry.finishId);
    check(finish && finish.route === 'finish' && !finishes.has(finish.id) && finish.ordinal > entry.ordinal && finish.id > entry.id &&
      finish.publication.caseIndex === entry.publication.caseIndex && finish.publication.scalarIndex === entry.publication.scalarIndex);
    finishes.add(finish.id);
    const selected = entry.route === 'compile' ? compiles : scalars, position = entry.route === 'compile' ? entry.publication.caseIndex : entry.publication.scalarIndex;
    check(!selected.has(position));selected.add(position);
  }
  check(compiles.size === 11 && scalars.size === 10 && finishes.size === 21);
  const candidates = value.rows.filter(entry => entry.network === 'ERR_ABORTED');
  check(Boolean(value.firstProvisional) === Boolean(candidates.length));
  if (value.firstProvisional) { const first = value.rows.find(entry => entry.ordinal === value.firstProvisional.ordinal);
    check(first && first.id === value.firstProvisional.id && first.route === value.firstProvisional.route && first.network === 'ERR_ABORTED'); }
  check(Buffer.byteLength(JSON.stringify(value)) <= 49152);return value;
}
export function validateCompletionFailure(value) {
  if (value === null) return;
  exact(value, ['format', 'sealed', 'firstProvisional', 'earliestUnresolved', 'unresolvedOrdinals', 'unmatchedLocalIds',
    'rowCount', 'candidateCount', 'fullLedgerSha256', 'fullLedgerBytes', 'truncated']);
  check(value.format === 'zryna.ui-api-completion-failure.v1' && value.sealed === false && typeof value.truncated === 'boolean');
  validateCompletionSummary(value.firstProvisional);validateCompletionSummary(value.earliestUnresolved);
  ids(value.unresolvedOrdinals);ids(value.unmatchedLocalIds);
  check(number(value.rowCount, 0, 256) && number(value.candidateCount, 0, value.rowCount) && hash(value.fullLedgerSha256) &&
    number(value.fullLedgerBytes, 1, bounds.frame) && Buffer.byteLength(JSON.stringify(value)) <= 4096);
  check(Boolean(value.firstProvisional) === Boolean(value.candidateCount));
  check(value.unresolvedOrdinals.every(ordinal => ordinal <= value.rowCount));
  for (const summary of [value.firstProvisional, value.earliestUnresolved]) if (summary) check(summary.ordinal <= value.rowCount);
  if (value.earliestUnresolved) check(value.unresolvedOrdinals.includes(value.earliestUnresolved.ordinal));
  return value;
}

const observedRoutes = ['index', 'session', 'compile', 'evaluate', 'finish', 'cancel', ...['client.mjs', 'controller.mjs', 'corpus.mjs', 'evaluation.mjs', 'executable.mjs', 'index.html', 'json.mjs', 'limits.mjs', 'main.mjs', 'presentation.mjs', 'protocol.mjs', 'report.mjs', 'style.css'].map(name => 'asset:' + name)];
export function validateRequestObservation(value) {
  if (value === null) return;
  exact(value, ['observed', 'ordinal', 'route', 'method', 'resourceType', 'responseSeen', 'responseStatus', 'requestFinished']);
  if (['observed', 'responseSeen', 'requestFinished'].some(key => typeof value[key] !== 'boolean') ||
    value.responseStatus !== null && (!Number.isInteger(value.responseStatus) || value.responseStatus < 100 || value.responseStatus > 599) ||
    !value.responseSeen && value.responseStatus !== null) fail('WIRE_001');
  if (!value.observed) {
    if (['ordinal', 'route', 'method', 'resourceType', 'responseStatus'].some(key => value[key] !== null) || value.responseSeen || value.requestFinished) fail('WIRE_001');
    return;
  }
  if (!Number.isSafeInteger(value.ordinal) || value.ordinal < 1 || value.ordinal > bounds.requests || !observedRoutes.includes(value.route) ||
    !['document', 'script', 'stylesheet', 'fetch'].includes(value.resourceType) ||
    value.method !== (value.route === 'index' || value.route === 'session' || value.route.startsWith('asset:') ? 'GET' : 'POST')) fail('WIRE_001');
}
