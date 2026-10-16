// Private operator test protocol, never a guest capability or a general HTTP API.
import { createHash } from 'node:crypto';

export const bounds = Object.freeze({ frame: 2097152, aggregate: 67108864, frames: 1024,
  requests: 256, live: 4, queued: 32, headers: 4096, stderr: 16384, lifetimeMs: 300000 });
// Closed validator sites only. Error text, URLs and protocol values never become branch labels.
export const transportBranches = Object.freeze(["UNSPECIFIED", "HTTP_ERROR", "INTERCEPT_EVENT", "INTERCEPT_HEADERS", "INTERCEPT_SEND", "INTERCEPT_WAIT", "INTERCEPT_FULFILL", "WIRE_001", "WIRE_002", "WIRE_003", "WIRE_004", "WIRE_005", "WIRE_006", "WIRE_007", "WIRE_008", "WIRE_009", "WIRE_010", "WIRE_011", "WIRE_012", "WIRE_013", "WIRE_014", "WIRE_015", "WIRE_016", "ROUTES_001", "ROUTES_002", "ROUTES_003", "ROUTES_004", "ROUTES_005", "ROUTES_006", "ROUTES_007", "ROUTES_008", "ROUTES_009", "ROUTES_010", "ROUTES_011", "ROUTES_012", "ROUTES_013", "FRAMING_001", "FRAMING_002", "FRAMING_003", "FRAMING_004", "FRAMING_005", "ENGINE_001", "ENGINE_002", "ENGINE_003", "ENGINE_004", "BROKER_001", "BROKER_002", "BROKER_003", "BROKER_004", "BROKER_005", "BROKER_006", "SESSION_001", "SESSION_002", "SESSION_003", "SESSION_004", "SESSION_005", "SESSION_006", "SESSION_007", "SESSION_008", "SESSION_009", "SESSION_010", "SESSION_011", "SESSION_012", "SESSION_013", "SESSION_014", "SESSION_015", "SESSION_016", "SESSION_017", "SESSION_018", "SESSION_019", "SESSION_020", "SESSION_021", "SESSION_022", "INTERCEPT_001", "INTERCEPT_002", "INTERCEPT_003", "INTERCEPT_004", "INTERCEPT_005", "INTERCEPT_006", "INTERCEPT_007", "INTERCEPT_008", "INTERCEPT_009", "INTERCEPT_010"]);
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
  'PAGE', 'CDP_SETUP', 'NAVIGATE', 'SOURCE_READY', 'INITIAL_STATE', 'AX_BEFORE', 'SOURCE_EDIT',
  'COMPILE_PAUSED', 'CANCEL_CLICK', 'ABORT_PROOF', 'CANCELLED_STATE', 'AX_AFTER', 'FINAL_STATE', 'RESULT']);
export const innerCauses = Object.freeze(['UNKNOWN', 'TIMEOUT', 'TRANSPORT', 'STOPPED', 'ABORT_UNPROVED',
  'INITIAL_HASH', 'INITIAL_PATH', 'INITIAL_TITLE', 'INITIAL_ROLE', 'INITIAL_LIVE', 'ABORT_EVENT',
  'FINAL_CONTROLS', 'AX_NODES', 'AX_NODE_TEXT', 'SCRIPT_SOURCE', 'RESULT_SIZE',
  'NET_ERR_FAILED', 'NET_ERR_ABORTED', 'TARGET_CLOSED', 'PAGE_CRASHED', 'FRAME_DETACHED',
  'NAVIGATION_INTERRUPTED', 'STRICT_MODE', 'PROTOCOL_ERROR']);
export function innerFailure(value, runId) {
  envelope(value, 'failure', ['runId', 'phase', 'cause', 'cleanupFailed', 'cleanup', 'branch']);
  exact(value.cleanup, ['intercept', 'context', 'browser']);
  if (value.runId !== runId || !innerPhases.includes(value.phase) || !innerCauses.includes(value.cause) ||
      !transportBranches.includes(value.branch) || typeof value.cleanupFailed !== 'boolean' ||
      Object.values(value.cleanup).some(bit => typeof bit !== 'boolean') ||
      value.cleanupFailed !== Object.values(value.cleanup).some(Boolean)) fail('WIRE_001');
  return Object.freeze({ ...value, cleanup: Object.freeze({ ...value.cleanup }) });
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
