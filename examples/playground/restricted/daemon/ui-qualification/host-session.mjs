// Owns only the private test HTTP channel; outer reviewed owner owns launcher and container teardown.
import { createHttpBroker } from './http-broker.mjs';
import { bounds, envelope, exact, fail, hash, integer, start } from './wire.mjs';
import { routes } from './routes.mjs';

function pinnedResources(selected) {
  if (!(selected instanceof Map)) fail();
  const copied = new Map();
  for (const [name, route] of Object.entries(routes)) {
    if (!name.startsWith('asset:') && name !== 'index') continue;
    const pin = selected.get(route.path);
    exact(pin, ['bytes', 'sha256']); integer(pin.bytes, route.output);
    if (!hash(pin.sha256)) fail();
    copied.set(route.path, Object.freeze({ ...pin }));
  }
  if (selected.size !== copied.size) fail();
  return copied;
}
function inspectResult(value, configuration, audit, resources) {
  envelope(value, 'result', ['runId', 'phase', 'transport', 'abort', 'initial', 'state', 'scripts',
    'axBefore', 'axAfter', 'axUpdates', 'corpus', 'audibleAssistiveTechnology', 'browserClosed']);
  if (value.runId !== configuration.runId || value.phase !== 'initial-abort-static-ax' ||
      value.transport !== 'operator-stdio-http-interception-v1' || value.corpus !== 'unrun' ||
      value.audibleAssistiveTechnology !== 'unrun' || value.browserClosed !== true ||
      Buffer.byteLength(JSON.stringify(value)) > 1048576) fail();
  exact(value.abort, ['id', 'route', 'paused', 'event', 'acknowledged']);
  integer(value.abort.id, bounds.requests);
  if (value.abort.route !== 'compile' || value.abort.paused !== true ||
      value.abort.event !== 'requestfailed' || value.abort.acknowledged !== true) fail();
  const actualAbort = audit.find(entry => entry.id === value.abort.id);
  if (!actualAbort?.aborted || actualAbort.dispatched !== false || actualAbort.route !== 'compile') fail();
  if (audit.some(entry => entry.route === 'evaluate' || entry.route === 'compile' && !entry.aborted)) fail();
  exact(value.initial, ['pathname', 'hashRemoved', 'title', 'status', 'role', 'live']);
  if (value.initial.pathname !== configuration.launchPath || value.initial.hashRemoved !== true ||
      value.initial.title !== 'Zryna source playground' || value.initial.role !== 'status' || value.initial.live !== 'polite') fail();
  exact(value.state, ['focusedId', 'status', 'compileDisabled', 'runDisabled', 'diagnosticChildren']);
  if (value.state.status !== 'Cancelled. The previous operation has finished cleanup.' ||
      value.state.compileDisabled !== false || value.state.runDisabled !== true || value.state.diagnosticChildren !== 0) fail();
  for (const entries of [value.axBefore, value.axAfter, ...value.axUpdates]) {
    if (!Array.isArray(entries) || !entries.length || entries.length > 512) fail();
    for (const node of entries) {
      exact(node, ['role', 'name', 'ignored', 'properties']);
      if (typeof node.role !== 'string' || typeof node.name !== 'string' || node.name.length > 4096 ||
          typeof node.ignored !== 'boolean' || !Array.isArray(node.properties) || node.properties.length > 4) fail();
      for (const property of node.properties) {
        exact(property, ['name', 'value']);
        if (!['focused', 'live', 'invalid', 'disabled'].includes(property.name) ||
            !['string', 'boolean'].includes(typeof property.value) && property.value !== null) fail();
      }
    }
  }
  if (!Array.isArray(value.axUpdates) || value.axUpdates.length > 32 || !Array.isArray(value.scripts) ||
      value.scripts.length > 32 || new Set(value.scripts).size !== value.scripts.length) fail();
  const accessible = (nodes, role, name) => nodes.some(node => !node.ignored && node.role === role && node.name === name);
  if (!accessible(value.axBefore, 'textbox', 'Source') || !accessible(value.axBefore, 'combobox', 'Examples') ||
      !accessible(value.axBefore, 'button', 'Compile source') ||
      !value.axBefore.some(node => node.role === 'textbox' && node.name === 'Source' &&
        node.properties.some(property => property.name === 'focused' && property.value === true)) ||
      !value.axAfter.some(node => node.role === 'status' &&
        node.properties.some(property => property.name === 'live' && property.value === 'polite'))) fail();
  for (const path of value.scripts) {
    if (!resources.has(path) || !path.endsWith('.mjs') || path.endsWith('/worker.mjs')) fail();
  }
  const required = Object.entries(routes).filter(([name]) => name.startsWith('asset:') && name !== 'asset:worker.mjs');
  for (const [name, route] of required) {
    const rows = audit.filter(entry => entry.route === name || name === 'asset:index.html' && entry.route === 'index');
    const pin = resources.get(route.path);
    if (!rows.length || rows.some(row => row.status !== 200 || row.responseBytes !== pin.bytes || row.responseSha256 !== pin.sha256)) fail();
  }
  if (!audit.some(entry => entry.route === 'session' && entry.status === 200) ||
      !audit.some(entry => entry.route === 'cancel' && entry.status === 200)) fail();
  return structuredClone(value);
}
export function createHostSession({ configuration, send, resourcePins }) {
  const selected = start(configuration);
  const resources = pinnedResources(resourcePins);
  let begun = false; let terminal = false; let stopped = false; let error = null; let resolveResult; let rejectResult;
  let stopping = null; let cleanupError = null;
  const operations = new Set(); const transmissions = new Set();
  const result = new Promise((resolve, reject) => { resolveResult = resolve; rejectResult = reject; });
  result.catch(() => {});
  const broken = () => {
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
      if (begun || stopped || terminal) fail();
      begun = true;
      return transmit(selected).catch(() => { broken(); throw error; });
    },
    accept(record) {
      if (stopped || terminal) fail();
      try {
        if (!begun) fail();
        if (record?.type === 'request') {
          const operation = broker.request(record).then(transmit).catch(broken);
          operations.add(operation); operation.finally(() => operations.delete(operation));
        } else if (record?.type === 'abort') {
          envelope(record, 'abort', ['id']);
          const reply = broker.abort(record.id);
          if (reply) transmit(reply).catch(broken);
        } else if (record?.type === 'result') {
          if (broker.activeIds().length) fail();
          const inspected = inspectResult(record, selected, broker.audit(), resources);
          terminal = true; resolveResult(Object.freeze({ observation: inspected,
            transportAudit: broker.audit(), fullQualification: false, containerCleanupConfirmed: false }));
        } else fail();
      } catch { broken(); throw error; }
    },
    eof() { if (!terminal) broken(); },
    recovery() {
      if (cleanupError) return new AggregateError([...(error ? [error] : []), cleanupError], 'PLAYGROUND-UI-CLEANUP');
      return broker.recovery() ?? error;
    },
    audit() { return broker.audit(); },
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
