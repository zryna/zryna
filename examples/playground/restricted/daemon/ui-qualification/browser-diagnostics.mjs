// Fixed observation of the existing browser only; no process launcher or policy selector.
import { ChildProcess } from 'node:child_process';
import { sha256, tagTransport, emptyBrowserProcess, browserProcessExit, browserProcessLog, scanBrowserFatal, fatalCandidate, inspectBrowserFatal, bounds } from './wire.mjs';
import { routeFor } from './routes.mjs';

// Read-only diagnostic of the exact child from the pinned in-process Playwright mapping.
export function observeBrowserProcess(browser) {
  const state = emptyBrowserProcess();let implementation, child, pid, active = true, bytes = 0, examined = 0, candidates = 0, frozen;
  const exited = (code, signal) => { try { if (active) browserProcessExit(state, code, signal); } catch { state.diagnosticFailed = true; } };
  try {
    implementation = browser._connection.toImpl(browser);child = implementation.options.browserProcess.process;pid = child.pid;
    if (!(child instanceof ChildProcess) || child.spawnfile !== '/materials/browser/chrome-linux64/chrome' ||
        !Number.isSafeInteger(pid) || pid < 1 || typeof implementation.options.browserLogsCollector?.onMessage !== 'function' ||
        typeof implementation.options.browserLogsCollector.recentLogs !== 'function') throw new Error();
    state.attached = true;state.identity = 'STABLE';state.fatalLive = 'COMPLETE';child.on('exit', exited);
    implementation.options.browserLogsCollector.onMessage(message => { try {
      if (!active || typeof message !== 'string' || !message.startsWith(`[pid=${pid}][err] `)) return;
      state.stderrSeen = true;const start = `[pid=${pid}][err] `.length;
      if (examined < 1024) {
        examined++;
        if (fatalCandidate(message.slice(start, start + 256))) {
          if (candidates < 64) { candidates++;inspectBrowserFatal(state, message.slice(start, start + 1024), message.length - start > 1024 ? message.slice(-1024) : ''); }
          else state.fatalLive = 'TRUNCATED';
        }
      } else state.fatalLive = 'TRUNCATED';
      if (bytes >= 16384 || message.length > 4096) { state.stderrTruncated = true;return; }
      bytes += Buffer.byteLength(message);if (bytes > 16384) { state.stderrTruncated = true;return; }
      browserProcessLog(state, message);
    } catch { state.diagnosticFailed = true; } });
  } catch { state.diagnosticFailed = true; }
  return () => {
    if (frozen) return structuredClone(frozen);
    if (state.attached) { try {
      try { scanBrowserFatal(state, implementation.options.browserLogsCollector.recentLogs(), `[pid=${pid}][err] `); } catch { state.fatalScan = 'UNAVAILABLE'; }
      let mapped;try { mapped = browser._connection.toImpl(browser); } catch {}
      state.identity = child.pid !== pid || implementation.options.browserProcess.process !== child || mapped && mapped !== implementation ? 'CHANGED' : mapped ? 'STABLE' : 'DETACHED';
      if (child.exitCode !== null || child.signalCode !== null) exited(child.exitCode, child.signalCode);
      child.off('exit', exited);
    } catch { state.identity = 'UNKNOWN';state.diagnosticFailed = true; } }
    active = false;frozen = structuredClone(state);return structuredClone(frozen);
  };
}

// Exact closed category only; the raw failure text is never retained.
export function requestFailureBranch(request) {
  let text = null;
  try { text = request.failure()?.errorText; } catch {}
  const codes = ["ERR_ABORTED", "ERR_FAILED", "ERR_EMPTY_RESPONSE", "ERR_CONNECTION_CLOSED", "ERR_CONNECTION_RESET", "ERR_CONNECTION_REFUSED", "ERR_TIMED_OUT", "ERR_INVALID_RESPONSE", "ERR_CONTENT_LENGTH_MISMATCH", "ERR_INCOMPLETE_CHUNKED_ENCODING", "ERR_BLOCKED_BY_RESPONSE", "ERR_BLOCKED_BY_CLIENT", "ERR_BLOCKED_BY_CSP", "ERR_NETWORK_CHANGED", "ERR_HTTP_RESPONSE_CODE_FAILURE"];
  const code = typeof text === 'string' && text.length <= 256 ? codes.find(value => text === 'net::' + value) : null;
  return 'INTERCEPT_REQUEST_FAILED_' + (code ?? 'UNKNOWN');
}

const requestErrors = new WeakMap();
const unknownRequest = () => ({ observed: false, ordinal: null, route: null, method: null, resourceType: null,
  responseSeen: false, responseStatus: null, requestFinished: false });
export function requestFailureObservation(error) {
  return structuredClone(requestErrors.get(error) ?? null);
}
// Diagnostic identity only: ordinal is not a host HTTP id or proof of body consumption.
export function observeBrowserRequests(context, selectedOrigin) {
  const requests = new WeakMap();let ordinal = 0;let stopped = false;
  const started = request => { try {
    if (stopped || requests.has(request) || ordinal >= bounds.requests) return;
    const method = request.method(), route = routeFor(request.url(), method, selectedOrigin), resourceType = request.resourceType();
    if (!['document', 'script', 'stylesheet', 'fetch'].includes(resourceType)) return;
    requests.set(request, { observed: true, ordinal: ++ordinal, route, method, resourceType,
      responseSeen: false, responseStatus: null, requestFinished: false });
  } catch { /* Missing diagnostic identity never changes the admission guard. */ } };
  const responded = response => { try {
    if (stopped) return;const value = requests.get(response.request());if (!value) return;
    value.responseSeen = true;const status = response.status();
    if (Number.isInteger(status) && status >= 100 && status <= 599) value.responseStatus = status;
  } catch { /* Status remains explicitly unknown. */ } };
  const finished = request => { if (!stopped) { const value = requests.get(request);if (value) value.requestFinished = true; } };
  context.on('request', started);context.on('response', responded);context.on('requestfinished', finished);
  return Object.freeze({
    failed(request, error) {
      if (!requestErrors.has(error)) requestErrors.set(error, Object.freeze({ ...(requests.get(request) ?? unknownRequest()) }));
      return error;
    },
    stop() { stopped = true;context.off('request', started);context.off('response', responded);context.off('requestfinished', finished); },
  });
}

// Same original fatal page/resource guards, shared to keep the transport module bounded.
export function guardBrowserPages(context, broken) {
  const pages = new Set();
  const events = Object.fromEntries(['worker', 'websocket', 'download', 'popup'].map(name => [name, () => broken(null, 'INTERCEPT_PAGE_' + name.toUpperCase())]));
  const serviceWorker = () => broken(null, 'INTERCEPT_SERVICE_WORKER');
  const started = page => {
    if (pages.size) { broken(null, 'INTERCEPT_SECOND_PAGE');return; }pages.add(page);
    for (const [name, handler] of Object.entries(events)) page.on(name, handler);
  };
  context.on('serviceworker', serviceWorker);context.on('page', started);
  return () => { context.off('serviceworker', serviceWorker);context.off('page', started);
    for (const page of pages) for (const [name, handler] of Object.entries(events)) page.off(name, handler); };
}

// A provisional event remains bound to the same actual Request and its observed status.
export function classifyRequestFailure(request, observations, completion) {
  const branch = requestFailureBranch(request);
  const error = observations.failed(request, tagTransport(new Error('PLAYGROUND-UI-TRANSPORT'), branch));
  if (completion && branch === 'INTERCEPT_REQUEST_FAILED_ERR_ABORTED' && completion.browserTerminal(request, 'ERR_ABORTED')) return null;
  return error;
}

// Legacy transport-only fixtures also retain identity without retaining the body.
export function requestBodyIdentity(request) {
  const body = request.postDataBuffer() ?? Buffer.alloc(0);
  return { bytes: body.length, sha256: sha256(body) };
}

// Fatal closed completion observations use the same actual response/Request identity.
export function observeCompletionResponses(context, current, stopped, broken) {
  const responded = response => { if (!stopped()) { try { current()?.browserResponse(response); } catch (error) { broken(error); } } };
  const finished = request => { if (!stopped()) { try { current()?.browserTerminal(request, 'FINISHED'); } catch (error) { broken(error); } } };
  context.on('response', responded);context.on('requestfinished', finished);
  return () => { context.off('response', responded);context.off('requestfinished', finished); };
}
