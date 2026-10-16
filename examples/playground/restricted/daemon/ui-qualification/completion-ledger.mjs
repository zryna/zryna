// Injective two-sided matching. Bodies and Request identities never leave this process.
import { fail, sha256, bounds } from './wire.mjs';
import { routeFor, routes } from './routes.mjs';
import { completionRoutes, validateCompletionLedger, validateCompletionFailure } from './completion-record.mjs';
const check = value => { if (!value) fail('COMPLETION_UNPROVEN'); };
export function createCompletionLedger(selectedOrigin) {
  const requests = new WeakMap(), browser = new Map(), local = new Map(), browserKeys = new Set(), localKeys = new Set();
  const byOrdinal = new Map(), byId = new Map(), fences = [], handles = new WeakMap();let active = null, sequence = 0, ordinal = 0, sealed = false;
  let firstProvisional = null, probeProved = false;
  function tick() { check(!sealed && ++sequence <= 1024);return sequence; }
  function key(route, method, body) { check(Buffer.isBuffer(body) && body.length <= routes[route].input);
    return JSON.stringify([selectedOrigin, route, method, body.length, sha256(body)]); }
  function join(entry, other) {
    if (!other) return;
    check(entry.fence === other.fence && !entry.peer && !other.peer);entry.peer = other;other.peer = entry;
  }
  function summary(anchor) {
    if (!anchor) return null;
    const entry = byOrdinal.get(anchor.ordinal);
    return { ...anchor, id: entry?.peer?.id ?? null };
  }
  function rows() {
    return [...byOrdinal.values()].map(entry => ({ ordinal: entry.ordinal, id: entry.peer?.id ?? null, route: entry.route,
      method: entry.method, requestBytes: entry.bytes, requestSha256: entry.sha256, browserStatus: entry.status,
      response: entry.peer?.response ?? null, network: entry.held && probeProved ? 'INITIAL_PROBE_ABORT' : entry.network,
      writeComplete: entry.peer?.writeComplete ?? false, publication: entry.publication ?? null, finishId: entry.finishId ?? null }));
  }
  // Keep this predicate explicit: a held probe has its separate original two-signal proof.
  function incomplete(entry) {
    if (!entry.peer || !entry.publication || !entry.network) return true;
    return entry.held ? !probeProved : !entry.peer.writeComplete || !entry.peer.response || entry.status !== 200;
  }
  return Object.freeze({
    begin(kind, caseIndex = null, scalarIndex = null) {
      tick();check(!active && ['STARTUP', 'INITIAL_ABORT', 'INITIAL_CANCEL', 'COMPILE', 'EVALUATE'].includes(kind));
      if (kind === 'STARTUP') check(!fences.length);
      const fence = { kind, caseIndex, scalarIndex, browser: [], local: [], published: false };
      active = fence;fences.push(fence);const handle = Object.freeze({});handles.set(handle, fence);return handle;
    },
    publish(handle) {
      tick();const fence = handles.get(handle);check(fence && active === fence && !fence.published);
      const target = { STARTUP: 'session', INITIAL_ABORT: 'compile', INITIAL_CANCEL: 'cancel', COMPILE: 'compile', EVALUATE: 'evaluate' }[fence.kind];
      const paired = ['COMPILE', 'EVALUATE'].includes(fence.kind), expected = paired ? 2 : 1;
      check(fence.browser.length === expected && fence.local.length === expected);
      check(fence.browser[0].route === target && fence.local[0].route === target);
      if (paired) check(fence.browser[1].route === 'finish' && fence.local[1].route === 'finish' && fence.browser[1].peer === fence.local[1]);
      for (const entry of fence.browser) {
        check(entry.peer && entry.peer.fence === fence);
        entry.publication = Object.freeze({ kind: entry.route === 'finish' ? 'FINISH' : fence.kind, caseIndex: fence.caseIndex, scalarIndex: fence.scalarIndex });
      }
      if (paired) fence.browser[0].finishId = fence.browser[1].peer.id;
      fence.published = true;active = null;
    },
    browserStart(request) {
      tick();check(!requests.has(request));const route = routeFor(request.url(), request.method(), selectedOrigin);
      if (!completionRoutes.includes(route)) return null;
      check(request.resourceType() === 'fetch' && typeof request.redirectedFrom === 'function' && typeof request.redirectedTo === 'function' &&
        !request.redirectedFrom() && !request.redirectedTo() && active && ordinal < 256);
      let body = request.postDataBuffer();if (route === 'session' && body == null) body = Buffer.alloc(0);
      const tuple = key(route, request.method(), body);check(!browserKeys.has(tuple));browserKeys.add(tuple);
      const entry = { ordinal: ++ordinal, route, method: request.method(), bytes: body.length, sha256: sha256(body),
        fence: active, held: active.kind === 'INITIAL_ABORT', peer: null, status: null, network: null, publication: null };
      check(entry.held === (route === 'compile' && active.kind === 'INITIAL_ABORT'));
      requests.set(request, entry);byOrdinal.set(entry.ordinal, entry);browser.set(tuple, entry);active.browser.push(entry);join(entry, local.get(tuple));
      return Object.freeze({ bytes: entry.bytes, sha256: entry.sha256 });
    },
    localBody(job, method, body) {
      tick();if (!completionRoutes.includes(job.name)) return;
      check(active && !byId.has(job.id) && byId.size < 256 && Number.isSafeInteger(job.id) && job.id > 0 && job.id <= 256);
      const tuple = key(job.name, method, body);check(!localKeys.has(tuple));localKeys.add(tuple);
      const entry = { id: job.id, route: job.name, fence: active, peer: null, response: null, writeComplete: false };
      byId.set(job.id, entry);local.set(tuple, entry);active.local.push(entry);join(entry, browser.get(tuple));
    },
    browserResponse(response) {
      tick();const entry = requests.get(response.request());if (!entry) return;
      check(entry.status === null && !entry.network && response.status() === 200 && !entry.held);entry.status = response.status();
    },
    browserTerminal(request, network) {
      tick();const entry = requests.get(request);if (!entry) return false;
      check(!entry.network && ['FINISHED', 'ERR_ABORTED'].includes(network));
      if (entry.held) { check(network === 'ERR_ABORTED');entry.network = network;return true; }
      if (entry.status !== 200 && network === 'ERR_ABORTED') return false;
      check(entry.status === 200);entry.network = network;
      if (network === 'ERR_ABORTED') {
        const anchor = Object.freeze({ ordinal: entry.ordinal, id: entry.peer?.id ?? null, route: entry.route,
          sequence, network, status: 200 });entry.provisional = anchor;firstProvisional ??= anchor;
      }
      return true;
    },
    localReply(id, status, body) {
      tick();const entry = byId.get(id);if (!entry) return;
      check(!entry.response && status === 200 && Buffer.isBuffer(body) && body.length <= routes[entry.route].output && entry.fence.kind !== 'INITIAL_ABORT');
      entry.response = Object.freeze({ status, bytes: body.length, sha256: sha256(body) });
    },
    localWriteComplete(id) { tick();const entry = byId.get(id);if (!entry) return;check(entry.response && !entry.writeComplete);entry.writeComplete = true; },
    proveInitialAbort(proof) {
      tick();const held = [...byOrdinal.values()].filter(entry => entry.held);check(!probeProved && held.length === 1);
      const entry = held[0];check(entry.network === 'ERR_ABORTED' && entry.peer && entry.peer.id === proof.id && proof.paused === true &&
        proof.acknowledged === true && proof.httpResponseClosed === true && proof.event === 'requestfailed' &&
        proof.requestBytes === entry.bytes && proof.requestSha256 === entry.sha256);probeProved = true;
    },
    seal(idle) {
      tick();check(idle === true && !active && fences.every(fence => fence.published) && byOrdinal.size === 45 && byId.size === 45 &&
        ![...byOrdinal.values()].some(incomplete) && ![...byId.values()].some(entry => !entry.peer));
      const value = { format: 'zryna.ui-api-completion-ledger.v1', sealed: true, rows: rows(), firstProvisional: summary(firstProvisional),
        unresolvedOrdinals: [], unmatchedLocalIds: [], failureEvidenceTruncated: false };
      validateCompletionLedger(value);sealed = true;return structuredClone(value);
    },
    failure() {
      const pending = [...byOrdinal.values()].filter(incomplete), candidates = [...byOrdinal.values()].filter(entry => !entry.held && entry.network === 'ERR_ABORTED');
      const earliest = candidates.filter(entry => pending.includes(entry)).sort((a,b) => a.provisional.sequence - b.provisional.sequence)[0];
      const anchor = summary(earliest?.provisional ?? null);
      const full = Buffer.from(JSON.stringify({ rows: rows(), firstProvisional: summary(firstProvisional) }));check(full.length <= bounds.frame);
      const value = { format: 'zryna.ui-api-completion-failure.v1', sealed: false, firstProvisional: summary(firstProvisional), earliestUnresolved: anchor,
        unresolvedOrdinals: pending.map(entry => entry.ordinal).sort((a,b) => a-b),
        unmatchedLocalIds: [...byId.values()].filter(entry => !entry.peer).map(entry => entry.id).sort((a,b) => a-b), rowCount: byOrdinal.size,
        candidateCount: candidates.length, fullLedgerSha256: sha256(full), fullLedgerBytes: full.length, truncated: true };
      validateCompletionFailure(value);return value;
    },
  });
}
