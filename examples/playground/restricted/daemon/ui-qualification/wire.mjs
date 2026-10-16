// Private operator test protocol, never a guest capability or a general HTTP API.
import { createHash } from 'node:crypto';

export const bounds = Object.freeze({ frame: 2097152, aggregate: 67108864, frames: 1024,
  requests: 256, live: 4, queued: 32, headers: 4096, stderr: 16384, lifetimeMs: 300000 });
// Closed validator sites only. Error text, URLs and protocol values never become branch labels.
export const transportBranches = Object.freeze(["UNSPECIFIED", "CORPUS_SOURCE", "CORPUS_RECEIPT", "CORPUS_STATUS", "CORPUS_SCALAR", "CORPUS_DOM", "CORPUS_AX", "CORPUS_UNICODE", "HTTP_ERROR", "INTERCEPT_EVENT", "INTERCEPT_HEADERS", "INTERCEPT_SEND", "INTERCEPT_WAIT", "INTERCEPT_FULFILL", "WIRE_001", "WIRE_002", "WIRE_003", "WIRE_004", "WIRE_005", "WIRE_006", "WIRE_007", "WIRE_008", "WIRE_009", "WIRE_010", "WIRE_011", "WIRE_012", "WIRE_013", "WIRE_014", "WIRE_015", "WIRE_016", "ROUTES_001", "ROUTES_002", "ROUTES_003", "ROUTES_004", "ROUTES_005", "ROUTES_006", "ROUTES_007", "ROUTES_008", "ROUTES_009", "ROUTES_010", "ROUTES_011", "ROUTES_012", "ROUTES_013", "FRAMING_001", "FRAMING_002", "FRAMING_003", "FRAMING_004", "FRAMING_005", "ENGINE_001", "ENGINE_002", "ENGINE_003", "ENGINE_004", "BROKER_001", "BROKER_002", "BROKER_003", "BROKER_004", "BROKER_005", "BROKER_006", "SESSION_001", "SESSION_002", "SESSION_003", "SESSION_004", "SESSION_005", "SESSION_006", "SESSION_007", "SESSION_008", "SESSION_009", "SESSION_010", "SESSION_011", "SESSION_012", "SESSION_013", "SESSION_014", "SESSION_015", "SESSION_016", "SESSION_017", "SESSION_018", "SESSION_019", "SESSION_020", "SESSION_021", "SESSION_022", "INTERCEPT_001", "INTERCEPT_002", "INTERCEPT_003", "INTERCEPT_004", "INTERCEPT_005", "INTERCEPT_006", "INTERCEPT_007", "INTERCEPT_008", "INTERCEPT_009", "INTERCEPT_010", "INTERCEPT_REQUEST_EVENT", "INTERCEPT_SECOND_PAGE", "INTERCEPT_RESPONSE_CLOSE", "INTERCEPT_RESPONSE_ERROR", "INTERCEPT_RESPONSE_INCOMPLETE", "INTERCEPT_SERVE", "INTERCEPT_SOCKET_LIMIT", "INTERCEPT_SOCKET_TIMEOUT", "INTERCEPT_DROP", "INTERCEPT_CLIENT_ERROR", "INTERCEPT_SERVER_ERROR", "INTERCEPT_SERVICE_WORKER", "INTERCEPT_INVALIDATE", "INTERCEPT_PAGE_WORKER", "INTERCEPT_PAGE_WEBSOCKET", "INTERCEPT_PAGE_DOWNLOAD", "INTERCEPT_PAGE_POPUP", "INTERCEPT_REQUEST_FAILED_UNKNOWN", "INTERCEPT_REQUEST_FAILED_ERR_ABORTED", "INTERCEPT_REQUEST_FAILED_ERR_FAILED", "INTERCEPT_REQUEST_FAILED_ERR_EMPTY_RESPONSE", "INTERCEPT_REQUEST_FAILED_ERR_CONNECTION_CLOSED", "INTERCEPT_REQUEST_FAILED_ERR_CONNECTION_RESET", "INTERCEPT_REQUEST_FAILED_ERR_CONNECTION_REFUSED", "INTERCEPT_REQUEST_FAILED_ERR_TIMED_OUT", "INTERCEPT_REQUEST_FAILED_ERR_INVALID_RESPONSE", "INTERCEPT_REQUEST_FAILED_ERR_CONTENT_LENGTH_MISMATCH", "INTERCEPT_REQUEST_FAILED_ERR_INCOMPLETE_CHUNKED_ENCODING", "INTERCEPT_REQUEST_FAILED_ERR_BLOCKED_BY_RESPONSE", "INTERCEPT_REQUEST_FAILED_ERR_BLOCKED_BY_CLIENT", "INTERCEPT_REQUEST_FAILED_ERR_BLOCKED_BY_CSP", "INTERCEPT_REQUEST_FAILED_ERR_NETWORK_CHANGED", "INTERCEPT_REQUEST_FAILED_ERR_HTTP_RESPONSE_CODE_FAILURE"]);
const errorBranches = new WeakMap();
export function tagTransport(error, branch) {
  if (error instanceof Error && !errorBranches.has(error)) errorBranches.set(error,
    transportBranches.includes(branch) ? branch : 'UNSPECIFIED');
  return error;
}
export function transportBranch(error) { return errorBranches.get(error) ?? 'UNSPECIFIED'; }
export const fail = (branch = 'UNSPECIFIED') => { throw tagTransport(new Error('PLAYGROUND-UI-TRANSPORT'), branch); };
export const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
export const hash = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
export const innerPhases = Object.freeze(['LOAD_CHROMIUM', 'BROWSER_LAUNCH', 'CONTEXT', 'INTERCEPT',
  'PAGE', 'CDP_ATTACH', 'AX_ENABLE', 'DEBUGGER_ENABLE', 'NAVIGATE', 'SOURCE_READY', 'INITIAL_STATE', 'AX_BEFORE', 'SOURCE_EDIT',
  'COMPILE_PAUSED', 'CANCEL_CLICK', 'ABORT_PROOF', 'CANCELLED_STATE', 'AX_AFTER', 'FINAL_STATE',
  'CORPUS_EDIT', 'CORPUS_COMPILE', 'CORPUS_EVALUATE', 'UNICODE_NAVIGATION', 'SOURCE_BUDGET', 'RESULT']);
export const innerCauses = Object.freeze(['UNKNOWN', 'TIMEOUT', 'TRANSPORT', 'STOPPED', 'ABORT_UNPROVED',
  'INITIAL_HASH', 'INITIAL_PATH', 'INITIAL_TITLE', 'INITIAL_ROLE', 'INITIAL_LIVE', 'ABORT_EVENT',
  'FINAL_CONTROLS', 'AX_NODES', 'AX_NODE_TEXT', 'SCRIPT_SOURCE', 'RESULT_SIZE', 'CORPUS_CHECK',
  'NET_ERR_FAILED', 'NET_ERR_ABORTED', 'TARGET_CLOSED', 'PAGE_CRASHED', 'FRAME_DETACHED',
  'NAVIGATION_INTERRUPTED', 'STRICT_MODE', 'PROTOCOL_ERROR']);
export function classifyBrowserError(error) {
  let cause = error?.name === 'TimeoutError' ? 'TIMEOUT' :
    error?.message === 'PLAYGROUND-UI-TRANSPORT' ? 'TRANSPORT' :
    error?.message === 'PLAYGROUND-UI-STOPPED' ? 'STOPPED' :
    error?.message === 'PLAYGROUND-UI-ABORT-UNPROVED' ? 'ABORT_UNPROVED' :
    error?.message === 'PLAYGROUND-UI-CORPUS-CHECK' ? 'CORPUS_CHECK' : 'UNKNOWN';
  // Recognize only fixed Playwright API prefixes and literal failure categories.
  // The remaining message may contain a launch URL/capability; it is never retained.
  const prefixes = ['page.goto', 'page.evaluate', 'page.waitForFunction', 'locator.waitFor',
    'locator.click', 'locator.fill', 'locator.press', 'browserType.launch', 'browser.newContext',
    'browserContext.newPage', 'browserContext.newCDPSession', 'browserContext.unroute',
    'browserContext.close', 'browser.close', 'route.fulfill', 'route.abort', 'cdpSession.send'];
  const text = typeof error?.message === 'string' && error.message.length <= 65536 ? error.message : '';
  const prefix = prefixes.find(value => text.startsWith(value + ': '));
  const message = prefix ? text.slice(prefix.length + 2) : '';
  if (cause === 'UNKNOWN' && prefix) {
    if (/^net::ERR_FAILED(?: at |;|$)/.test(message)) cause = 'NET_ERR_FAILED';
    else if (/^net::ERR_ABORTED(?: at |;|$)/.test(message)) cause = 'NET_ERR_ABORTED';
    else if (/^(?:Target page, context or browser has been closed|Target closed\.?)(?:\n|$)/.test(message) ||
      /^Protocol error \((?:Target\.disposeBrowserContext|Target\.closeTarget|Target\.createTarget|Browser\.close)\): Target closed\.?/.test(message)) cause = 'TARGET_CLOSED';
    else if (/^Page crashed(?:\n|$)/.test(message)) cause = 'PAGE_CRASHED';
    else if (/^(?:Frame was detached|Navigating frame was detached)(?:\n|$)/.test(message)) cause = 'FRAME_DETACHED';
    else if (message.startsWith('Navigation to ') && message.includes(' is interrupted by another navigation')) cause = 'NAVIGATION_INTERRUPTED';
    else if (message.startsWith('Error: strict mode violation:') || message.startsWith('strict mode violation:')) cause = 'STRICT_MODE';
    else if (message.startsWith('Protocol error (')) cause = 'PROTOCOL_ERROR';
  }
  return cause;
}
export const browserSignals = Object.freeze(['NONE', 'UNKNOWN', 'SIGHUP', 'SIGINT', 'SIGQUIT', 'SIGILL',
  'SIGTRAP', 'SIGABRT', 'SIGBUS', 'SIGFPE', 'SIGKILL', 'SIGSEGV', 'SIGPIPE', 'SIGALRM', 'SIGTERM', 'SIGSYS']);
export const browserStderr = Object.freeze(['SANDBOX', 'THREAD_CREATE', 'MEMORY_ALLOCATION',
  'SHARED_MEMORY', 'SECCOMP', 'CRASH_SIGNAL', 'FATAL_CHECK', 'CXX_UNCAUGHT',
  'PERMISSION_DENIED', 'RESOURCE_UNAVAILABLE', 'READ_ONLY_FILESYSTEM', 'NO_SPACE',
  'C_ASSERT', 'STD_TERMINATE', 'BAD_ALLOC']);
export const emptyBrowserProcess = () => ({ attached: false, identity: 'UNAVAILABLE', exitBeforeCleanup: false,
  exitCode: null, signal: 'NONE', stderrSeen: false, stderrUnknown: false, stderrTruncated: false,
  stderrCategories: [], fatalScan: 'UNRUN', fatalLive: 'UNRUN', fatalHeader: 'UNSEEN', fatalLocations: [], diagnosticFailed: false });
export function browserProcessExit(value, code, signal) {
  value.exitBeforeCleanup = true;
  value.exitCode = Number.isInteger(code) && code >= 0 && code <= 255 ? code : null;
  value.signal = signal == null ? 'NONE' : browserSignals.includes(signal) ? signal : 'UNKNOWN';
  if (value.exitCode === null && value.signal === 'NONE') value.signal = 'UNKNOWN';
}
export function browserProcessLog(value, text) {
  value.stderrSeen = true;
  const categories = [];
  if (/No usable sandbox!|Failed to move to new namespace|FATAL:.*(?:sandbox|zygote)/i.test(text)) categories.push('SANDBOX');
  if (/pthread_create: Resource temporarily unavailable|Failed to create.*thread|thread constructor failed/i.test(text)) categories.push('THREAD_CREATE');
  if (/out of memory|Cannot allocate memory|OOM killer/i.test(text)) categories.push('MEMORY_ALLOCATION');
  if (text.includes('/dev/shm') && /No space left on device|Failed to create/i.test(text)) categories.push('SHARED_MEMORY');
  if (/Bad system call|SIGSYS|FATAL:.*seccomp/i.test(text)) categories.push('SECCOMP');
  if (/Received signal [0-9]+|Fatal signal|SIG(?:SEGV|ABRT|TRAP|BUS)/.test(text)) categories.push('CRASH_SIGNAL');
  if (/FATAL:|Check failed:/.test(text)) categories.push('FATAL_CHECK');
  if (!categories.length) value.stderrUnknown = true;
  value.stderrCategories = [...new Set([...value.stderrCategories, ...categories])].sort();
}
// Bounded candidate peeks and strict source-header projection; no assertion text is retained.
export const fatalCandidate = text => /FATAL:|Check failed:|Received signal [0-9]+|Fatal signal|SIG(?:ABRT|SEGV|TRAP|BUS)|libc\+\+abi: terminating|[Aa]ssertion|terminate called|bad_alloc|bad allocation/.test(text);
export function inspectBrowserFatal(value, first, last) {
  const pieces = [first, last];if (!pieces.some(fatalCandidate)) return;
  for (const text of pieces) {
    if (!text) continue;
    browserProcessLog(value, text);const names = [];
    if (/libc\+\+abi: terminating (?:due to |with )uncaught exception/.test(text)) names.push('CXX_UNCAUGHT');
    if (/\b[Aa]ssertion\b[^\n]{0,512}\bfailed\b/.test(text)) names.push('C_ASSERT');
    if (/terminate called (?:after throwing|without an active exception)/.test(text)) names.push('STD_TERMINATE');
    if (/\b(?:std::(?:__1::)?)?bad_alloc\b|bad allocation/.test(text)) names.push('BAD_ALLOC');
    if (/Operation not permitted|Permission denied/.test(text)) names.push('PERMISSION_DENIED');
    if (text.includes('Resource temporarily unavailable')) names.push('RESOURCE_UNAVAILABLE');
    if (text.includes('Read-only file system')) names.push('READ_ONLY_FILESYSTEM');
    if (text.includes('No space left on device')) names.push('NO_SPACE');
    value.stderrCategories = [...new Set([...value.stderrCategories, ...names])].sort();
  }
  const location = /^\[[0-9:./-]{0,64}FATAL:(?:[A-Za-z0-9_+.-]+\/){0,16}([A-Za-z_][A-Za-z0-9_]{0,63}\.(?:cc|h|c|cpp))(?::([1-9][0-9]{0,5})|\(([1-9][0-9]{0,5})\))\]/.exec(first);
  if (location) {
    value.fatalHeader = 'PARSED';const item = { source: location[1], line: Number(location[2] ?? location[3]) };
    if (value.fatalLocations.length < 4 && !value.fatalLocations.some(old => old.source === item.source && old.line === item.line)) value.fatalLocations.push(item);
  } else if (pieces.some(text => text.includes('FATAL:')) && value.fatalHeader !== 'PARSED') value.fatalHeader = 'UNPARSED';
}
export function scanBrowserFatal(value, logs, prefix) {
  if (!Array.isArray(logs) || logs.length > 300) { value.fatalScan = 'UNAVAILABLE';return; }
  value.fatalScan = logs.length > 64 ? 'TRUNCATED' : 'COMPLETE';let selected = 0;
  for (let index = logs.length - 1; index >= 0 && selected < 64; index--) {
    const message = logs[index];if (typeof message !== 'string' || !message.startsWith(prefix)) continue;
    selected++;value.stderrSeen = true;const start = prefix.length;
    if (message.length - start > 2048) value.fatalScan = 'TRUNCATED';
    inspectBrowserFatal(value, message.slice(start, start + 1024), message.length - start > 1024 ? message.slice(-1024) : '');
  }
}
export function validateBrowserProcess(value) {
  exact(value, Object.keys(emptyBrowserProcess()));
  if (!['UNRUN', 'COMPLETE', 'TRUNCATED', 'UNAVAILABLE'].includes(value.fatalScan) ||
      !['UNRUN', 'COMPLETE', 'TRUNCATED'].includes(value.fatalLive) ||
      !['UNSEEN', 'PARSED', 'UNPARSED'].includes(value.fatalHeader) ||
      (value.fatalHeader === 'PARSED') !== Boolean(value.fatalLocations?.length) ||
      !Array.isArray(value.fatalLocations) || value.fatalLocations.length > 4 ||
      !value.attached && (value.fatalScan !== 'UNRUN' || value.fatalLive !== 'UNRUN' || value.fatalHeader !== 'UNSEEN' || value.fatalLocations.length)) fail('WIRE_001');
  const locations = new Set();
  for (const item of value.fatalLocations) {
    exact(item, ['source', 'line']);
    if (typeof item.source !== 'string' || !/^[A-Za-z_][A-Za-z0-9_]{0,63}\.(?:cc|h|c|cpp)$/.test(item.source) ||
        !Number.isInteger(item.line) || item.line < 1 || item.line > 999999 ||
        locations.has(item.source + ':' + item.line) || !value.stderrSeen) fail('WIRE_001');
    locations.add(item.source + ':' + item.line);
  }
  if (['attached', 'exitBeforeCleanup', 'stderrSeen', 'stderrUnknown', 'stderrTruncated', 'diagnosticFailed'].some(key => typeof value[key] !== 'boolean') ||
      !['UNAVAILABLE', 'STABLE', 'DETACHED', 'CHANGED', 'UNKNOWN'].includes(value.identity) ||
      value.attached === (value.identity === 'UNAVAILABLE') || !browserSignals.includes(value.signal) ||
      value.exitCode !== null && (!Number.isInteger(value.exitCode) || value.exitCode < 0 || value.exitCode > 255) ||
      value.exitCode !== null && value.signal !== 'NONE' ||
      !value.exitBeforeCleanup && (value.exitCode !== null || value.signal !== 'NONE') ||
      value.exitBeforeCleanup && (value.exitCode === null && value.signal === 'NONE' || !value.attached) ||
      !Array.isArray(value.stderrCategories) ||
      !value.stderrSeen && (value.stderrUnknown || value.stderrTruncated || value.stderrCategories.length > 0) || value.stderrCategories.some(name => !browserStderr.includes(name)) ||
      JSON.stringify(value.stderrCategories) !== JSON.stringify([...new Set(value.stderrCategories)].sort())) fail('WIRE_001');
  return Object.freeze(structuredClone(value));
}
export function innerFailure(value, runId) {
  envelope(value, 'failure', ['runId', 'phase', 'cause', 'cleanupFailed', 'cleanup', 'lifecycle', 'process', 'branch']);
  exact(value.cleanup, ['intercept', 'context', 'browser']);
  validateBrowserProcess(value.process);
  exact(value.lifecycle, ['browserDisconnected', 'pageClosed', 'pageCrashed', 'contextClosed']);
  if (value.runId !== runId || !innerPhases.includes(value.phase) || !innerCauses.includes(value.cause) ||
      !transportBranches.includes(value.branch) || typeof value.cleanupFailed !== 'boolean' ||
      Object.values(value.lifecycle).some(bit => typeof bit !== 'boolean') ||
      Object.values(value.cleanup).some(bit => typeof bit !== 'boolean') ||
      value.cleanupFailed !== Object.values(value.cleanup).some(Boolean)) fail('WIRE_001');
  return Object.freeze({ ...value, cleanup: Object.freeze({ ...value.cleanup }), lifecycle: Object.freeze({ ...value.lifecycle }) });
}
export function exact(value, keys) {
  if (!value || typeof value !== 'object' || Array.isArray(value) ||
      JSON.stringify(Object.keys(value).sort()) !== JSON.stringify([...keys].sort())) fail('WIRE_002');
}
export function integer(value, maximum = Number.MAX_SAFE_INTEGER) {
  if (!Number.isSafeInteger(value) || value < 1 || value > maximum) fail('WIRE_003');
  return value;
}
export function decode(bytes) {
  if (!(bytes instanceof Uint8Array) || !bytes.length || bytes.length > bounds.frame) fail('WIRE_004');
  let text; let value;
  try { text = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true }).decode(bytes); value = JSON.parse(text); }
  catch { fail('WIRE_005'); }
  const pending = [[value, 0]];
  while (pending.length) {
    const [entry, depth] = pending.pop();
    if (depth > 8) fail('WIRE_006');
    if (entry && typeof entry === 'object') {
      const entries = Object.values(entry);
      if (entries.length > 512) fail('WIRE_007');
      for (const child of entries) pending.push([child, depth + 1]);
    } else if (typeof entry === 'number' && (!Number.isSafeInteger(entry) || Object.is(entry, -0))) fail('WIRE_008');
  }
  // Exact canonical serialization also rejects duplicate keys and alternate token spellings.
  if (JSON.stringify(value) !== text) fail('WIRE_009');
  return value;
}
export function binary(value, maximum) {
  if (typeof value !== 'string' || value.length > Math.ceil(maximum / 3) * 4 ||
      !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(value)) fail('WIRE_010');
  const bytes = Buffer.from(value, 'base64');
  if (bytes.length > maximum || bytes.toString('base64') !== value) fail('WIRE_011');
  return bytes;
}
export function envelope(value, type, keys) {
  exact(value, ['version', 'type', ...keys]);
  if (value.version !== 1 || value.type !== type) fail('WIRE_012');
}
export function origin(value) {
  if (typeof value !== 'string' || !/^http:\/\/127\.0\.0\.1:[1-9][0-9]{0,4}$/.test(value)) fail('WIRE_013');
  const parsed = new URL(value);
  if (!parsed.port || Number(parsed.port) > 65535 || parsed.origin !== value) fail('WIRE_014');
  return value;
}
export function start(value) {
  envelope(value, 'start', ['runId', 'origin', 'launchPath', 'capability', 'planSha256']);
  if (typeof value.runId !== 'string' || !/^[a-f0-9]{32}$/.test(value.runId) ||
      value.launchPath !== '/examples/playground/restricted/' || !hash(value.capability) ||
      !hash(value.planSha256)) fail('WIRE_015');
  origin(value.origin);
  if (Buffer.byteLength(JSON.stringify(value)) > 4096) fail('WIRE_016');
  return Object.freeze({ ...value });
}

// Read-only Playwright failure metadata; exact closed categories only, never raw error text.
export function requestFailureBranch(request) {
  let text = null;
  try { text = request.failure()?.errorText; } catch {}
  const codes = ["ERR_ABORTED", "ERR_FAILED", "ERR_EMPTY_RESPONSE", "ERR_CONNECTION_CLOSED", "ERR_CONNECTION_RESET", "ERR_CONNECTION_REFUSED", "ERR_TIMED_OUT", "ERR_INVALID_RESPONSE", "ERR_CONTENT_LENGTH_MISMATCH", "ERR_INCOMPLETE_CHUNKED_ENCODING", "ERR_BLOCKED_BY_RESPONSE", "ERR_BLOCKED_BY_CLIENT", "ERR_BLOCKED_BY_CSP", "ERR_NETWORK_CHANGED", "ERR_HTTP_RESPONSE_CODE_FAILURE"];
  const code = typeof text === 'string' && text.length <= 256 ? codes.find(value => text === 'net::' + value) : null;
  return 'INTERCEPT_REQUEST_FAILED_' + (code ?? 'UNKNOWN');
}
