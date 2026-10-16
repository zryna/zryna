// Owns only the private test HTTP channel; outer reviewed owner owns launcher and container teardown.
import { createHttpBroker } from './http-broker.mjs';
import { bounds, envelope, exact, fail, hash, integer, start, innerFailure, transportBranch, transportBranches } from './wire.mjs';
import { routes } from './routes.mjs';

// Failure evidence is a projection of owned observations, never raw protocol or headers.
export function validateFailureEvidence(value) {
  exact(value, ['format', 'version', 'firstBranch', 'inner', 'audit']);
  if (value.format !== 'zryna.ui-failure-transport.v1' || value.version !== 1 ||
      !transportBranches.includes(value.firstBranch) || !Array.isArray(value.audit) ||
      value.audit.length > bounds.requests) fail();
  if (value.inner !== null) {
    if (typeof value.inner?.runId !== 'string' || !/^[a-f0-9]{32}$/.test(value.inner.runId)) fail();
    innerFailure(value.inner, value.inner.runId);
  }
  const seen = new Set();
  for (const row of value.audit) {
    integer(row.id, bounds.requests);
    if (seen.has(row.id) || !Object.hasOwn(routes, row.route) || row.route === 'asset:worker.mjs') fail();
    seen.add(row.id);const route = routes[row.route];
    if (Object.hasOwn(row, 'aborted')) {
      exact(row, ['id', 'route', 'aborted', 'dispatched', 'requestBytes', 'requestSha256']);
      if (!Number.isSafeInteger(row.requestBytes) || row.requestBytes < 0 || row.requestBytes > route.input || !hash(row.requestSha256)) fail();
      if (row.aborted !== true || typeof row.dispatched !== 'boolean') fail();
    } else {
      exact(row, ['id', 'route', 'method', 'requestBytes', 'requestSha256', 'status', 'responseBytes', 'responseSha256']);
      if (row.method !== route.method || !Number.isSafeInteger(row.requestBytes) || row.requestBytes < 0 ||
          row.requestBytes > route.input || !Number.isSafeInteger(row.responseBytes) || row.responseBytes < 0 ||
          row.responseBytes > route.output || !hash(row.requestSha256) || !hash(row.responseSha256) ||
          !Number.isSafeInteger(row.status) || row.status < 200 || row.status > 599 || row.status >= 300 && row.status < 400) fail();
    }
  }
  if (Buffer.byteLength(JSON.stringify(value)) > 131072) fail();
  return Object.freeze(structuredClone(value));
}

function pinnedResources(selected) {
  if (!(selected instanceof Map)) fail('SESSION_001');
  const copied = new Map();
  for (const [name, route] of Object.entries(routes)) {
    if (!name.startsWith('asset:') && name !== 'index') continue;
    const pin = selected.get(route.path);
    exact(pin, ['bytes', 'sha256']); integer(pin.bytes, route.output);
    if (!hash(pin.sha256)) fail('SESSION_002');
    copied.set(route.path, Object.freeze({ ...pin }));
  }
  if (selected.size !== copied.size) fail('SESSION_003');
  return copied;
}
function inspectResult(value, configuration, audit, resources) {
  envelope(value, 'result', ['runId', 'phase', 'transport', 'abort', 'initial', 'state', 'scripts',
    'axBefore', 'axAfter', 'axUpdates', 'corpus', 'audibleAssistiveTechnology', 'browserClosed']);
  if (value.runId !== configuration.runId || value.phase !== 'initial-abort-static-ax' ||
      value.transport !== 'operator-stdio-browser-loopback-http-v1' || value.corpus !== 'unrun' ||
      value.audibleAssistiveTechnology !== 'unrun' || value.browserClosed !== true ||
      Buffer.byteLength(JSON.stringify(value)) > 1048576) fail('SESSION_004');
  exact(value.abort, ['id', 'route', 'paused', 'event', 'acknowledged', 'httpResponseClosed', 'requestBytes', 'requestSha256']);
  integer(value.abort.id, bounds.requests);
  if (value.abort.route !== 'compile' || value.abort.paused !== true ||
      value.abort.event !== 'requestfailed' || value.abort.acknowledged !== true || value.abort.httpResponseClosed !== true ||
      !Number.isSafeInteger(value.abort.requestBytes) || value.abort.requestBytes < 0 || value.abort.requestBytes > routes.compile.input || !hash(value.abort.requestSha256)) fail('SESSION_005');
  const actualAbort = audit.find(entry => entry.id === value.abort.id);
  if (!actualAbort?.aborted || actualAbort.dispatched !== false || actualAbort.route !== 'compile' || actualAbort.requestBytes !== value.abort.requestBytes ||
      actualAbort.requestSha256 !== value.abort.requestSha256) fail('SESSION_006');
  if (audit.some(entry => entry.route === 'evaluate' || entry.route === 'compile' && !entry.aborted)) fail('SESSION_007');
  exact(value.initial, ['pathname', 'hashRemoved', 'title', 'status', 'role', 'live']);
  if (value.initial.pathname !== configuration.launchPath || value.initial.hashRemoved !== true ||
      value.initial.title !== 'Zryna source playground' || value.initial.role !== 'status' || value.initial.live !== 'polite') fail('SESSION_008');
  exact(value.state, ['focusedId', 'status', 'compileDisabled', 'runDisabled', 'diagnosticChildren']);
  if (value.state.status !== 'Cancelled. The previous operation has finished cleanup.' ||
      value.state.compileDisabled !== false || value.state.runDisabled !== true || value.state.diagnosticChildren !== 0) fail('SESSION_009');
  for (const entries of [value.axBefore, value.axAfter, ...value.axUpdates]) {
    if (!Array.isArray(entries) || !entries.length || entries.length > 512) fail('SESSION_010');
    for (const node of entries) {
      exact(node, ['role', 'name', 'ignored', 'properties']);
      if (typeof node.role !== 'string' || typeof node.name !== 'string' || node.name.length > 4096 ||
          typeof node.ignored !== 'boolean' || !Array.isArray(node.properties) || node.properties.length > 4) fail('SESSION_011');
      for (const property of node.properties) {
        exact(property, ['name', 'value']);
        if (!['focused', 'live', 'invalid', 'disabled'].includes(property.name) ||
            !['string', 'boolean'].includes(typeof property.value) && property.value !== null) fail('SESSION_012');
      }
    }
  }
  if (!Array.isArray(value.axUpdates) || value.axUpdates.length > 32 || !Array.isArray(value.scripts) ||
      value.scripts.length > 32 || new Set(value.scripts).size !== value.scripts.length) fail('SESSION_013');
  const accessible = (nodes, role, name) => nodes.some(node => !node.ignored && node.role === role && node.name === name);
  if (!accessible(value.axBefore, 'textbox', 'Source') || !accessible(value.axBefore, 'combobox', 'Examples') ||
      !accessible(value.axBefore, 'button', 'Compile source') ||
      !value.axBefore.some(node => node.role === 'textbox' && node.name === 'Source' &&
        node.properties.some(property => property.name === 'focused' && property.value === true)) ||
      !value.axAfter.some(node => node.role === 'status' &&
        node.properties.some(property => property.name === 'live' && property.value === 'polite'))) fail('SESSION_014');
  for (const path of value.scripts) {
    if (!resources.has(path) || !path.endsWith('.mjs') || path.endsWith('/worker.mjs')) fail('SESSION_015');
  }
  const required = Object.entries(routes).filter(([name]) => name.startsWith('asset:') && name !== 'asset:worker.mjs');
  for (const [name, route] of required) {
    const rows = audit.filter(entry => entry.route === name || name === 'asset:index.html' && entry.route === 'index');
    const pin = resources.get(route.path);
    if (!rows.length || rows.some(row => row.status !== 200 || row.responseBytes !== pin.bytes || row.responseSha256 !== pin.sha256)) fail('SESSION_016');
  }
  if (!audit.some(entry => entry.route === 'session' && entry.status === 200) ||
      !audit.some(entry => entry.route === 'cancel' && entry.status === 200)) fail('SESSION_017');
  return structuredClone(value);
}
export function createHostSession({ configuration, send, resourcePins }) {
  const selected = start(configuration);
  const resources = pinnedResources(resourcePins);
  let begun = false; let terminal = false; let stopped = false; let error = null; let resolveResult; let rejectResult;
  let stopping = null; let cleanupError = null;
  let firstBranch = null;let inner = null;
  const operations = new Set(); const transmissions = new Set();
  const result = new Promise((resolve, reject) => { resolveResult = resolve; rejectResult = reject; });
  result.catch(() => {});
  const broken = failure => {
    firstBranch ??= transportBranch(failure);
    error ??= new Error('PLAYGROUND-UI-TRANSPORT'); stopped = true; rejectResult(error);
  };
  const transmit = record => {
    const pending = Promise.resolve().then(() => { if (!stopped) return send(record); });
    transmissions.add(pending);
    const release = () => transmissions.delete(pending);
    pending.then(release, release);
    return pending;
  };
  const broker = createHttpBroker({ origin: selected.origin, capability: selected.capability,
    holdCompileProbe: true, onEvent: record => { transmit(record).catch(broken); } });
  return Object.freeze({
    result,
    begin() {
      if (begun || stopped || terminal) fail('SESSION_018');
      begun = true;
      return transmit(selected).catch(failure => { broken(failure);throw error; });
    },
    accept(record) {
      if (stopped || terminal) fail('SESSION_019');
      try {
        if (!begun) fail('SESSION_020');
        if (record?.type === 'request') {
          const operation = broker.request(record).then(transmit).catch(broken);
          operations.add(operation); operation.finally(() => operations.delete(operation));
        } else if (record?.type === 'abort') {
          envelope(record, 'abort', ['id']);
          const reply = broker.abort(record.id);
          if (reply) transmit(reply).catch(broken);
        } else if (record?.type === 'failure') {
          const observed = innerFailure(record, selected.runId);
          inner = observed;firstBranch ??= observed.branch;
          terminal = true;stopped = true;
          error = new Error(`PLAYGROUND-UI-INNER-${observed.phase.replaceAll('_', '-')}-${observed.cause.replaceAll('_', '-')}${observed.cleanupFailed ? '-CLEANUP' : ''}`);
          rejectResult(error);
        } else if (record?.type === 'result') {
          if (broker.activeIds().length) fail('SESSION_021');
          const inspected = inspectResult(record, selected, broker.audit(), resources);
          terminal = true; resolveResult(Object.freeze({ observation: inspected,
            transportAudit: broker.audit(), fullQualification: false, containerCleanupConfirmed: false }));
        } else fail('SESSION_022');
      } catch (failure) { broken(failure);throw error; }
    },
    eof() { if (!terminal) broken(); },
    recovery() {
      if (cleanupError) return new AggregateError([...(error ? [error] : []), cleanupError], 'PLAYGROUND-UI-CLEANUP');
      return broker.recovery() ?? error;
    },
    audit() { return broker.audit(); },
    failureEvidence(failure) {
      if (failure) firstBranch ??= transportBranch(failure);
      const audit = broker.audit().map(row => {
        if (row.aborted) return { id: row.id, route: row.route, aborted: true, dispatched: row.dispatched, requestBytes: row.requestBytes, requestSha256: row.requestSha256 };
        const { id, route, method, requestBytes, requestSha256, status, responseBytes, responseSha256 } = row;
        return { id, route, method, requestBytes, requestSha256, status, responseBytes, responseSha256 };
      });
      return validateFailureEvidence({ format: 'zryna.ui-failure-transport.v1', version: 1,
        firstBranch: firstBranch ?? 'UNSPECIFIED', inner, audit });
    },
    stop() {
      if (stopping) return stopping;
      stopped = true;
      if (!terminal) { error ??= new Error('PLAYGROUND-UI-STOPPED'); rejectResult(error); }
      stopping = (async () => {
        let timer;
        try {
          // Start HTTP cleanup independently of a potentially stalled attached-stdin writer.
          await Promise.race([Promise.all([broker.close(), ...operations, ...transmissions]), new Promise((_, reject) => {
            timer = setTimeout(() => reject(new Error('PLAYGROUND-UI-CLEANUP')), 2000);
          })]);
        } catch (failure) { cleanupError ??= failure; throw failure; }
        finally { clearTimeout(timer); }
      })();
      return stopping;
    },
  });
}
