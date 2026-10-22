// Trusted callbacks must yield to the event loop; inline JS cannot be forcibly preempted here.
// Only an independently reviewed creation-owned adapter can attest actual containment/cleanup.
import { exact, fail } from './records.mjs';

const error = reason => new Error(`PLAYGROUND-CLI-PARITY-${reason}`);
const caughtError = value => value instanceof Error ? value : error('ADAPTER-FAILED');
export function validateAdapter(adapter) {
  exact(adapter, ['run', 'cleanup']);
  if (typeof adapter.run !== 'function' || typeof adapter.cleanup !== 'function') fail();
}
export async function boundedAdapterCall(adapter, input, { kind, id, activeMs, signal }) {
  validateAdapter(adapter);
  if (!['cli', 'compile', 'evaluate'].includes(kind) || typeof id !== 'string' ||
      activeMs !== ({ cli: 10000, compile: 10000, evaluate: 5000 })[kind] ||
      signal !== undefined && !(signal instanceof AbortSignal)) fail();
  if (signal?.aborted) throw error('ABORTED');
  const active = new AbortController(); const cleanup = new AbortController();
  let activeTimer; let cleanupTimer; let primary; let result; let rejectAbort;
  const cancelled = new Promise((_, reject) => { rejectAbort = reject; });
  void cancelled.catch(() => {});
  const cancel = () => { active.abort(); rejectAbort(error('ABORTED')); };
  signal?.addEventListener('abort', cancel, { once: true });
  const started = performance.now();
  try {
    activeTimer = setTimeout(() => { active.abort(); rejectAbort(error('DEADLINE')); }, activeMs);
    result = await Promise.race([Promise.resolve().then(() => {
      if (signal?.aborted || active.signal.aborted) throw error('ABORTED');
      return adapter.run(input, Object.freeze({ kind, id, signal: active.signal, activeMs, cleanupMs: 2000 }));
    }), cancelled]);
    if (performance.now() - started >= activeMs) throw error('DEADLINE');
    if (signal?.aborted) throw error('ABORTED');
  } catch (caught) { primary = caughtError(caught); }
  finally { clearTimeout(activeTimer); active.abort(); signal?.removeEventListener('abort', cancel); }
  const cleanupStarted = performance.now();
  try {
    const deadline = new Promise((_, reject) => {
      cleanupTimer = setTimeout(() => { cleanup.abort(); reject(error('CLEANUP-DEADLINE')); }, 2000);
    });
    const proof = await Promise.race([Promise.resolve().then(() => adapter.cleanup(Object.freeze({
      kind, id, cancelled: Boolean(primary), signal: cleanup.signal, cleanupMs: 2000,
    }))), deadline]);
    if (performance.now() - cleanupStarted >= 2000) throw error('CLEANUP-DEADLINE');
    exact(proof, ['confirmed']); if (proof.confirmed !== true) throw error('CLEANUP-UNKNOWN');
  } catch (caught) {
    const failure = error('CLEANUP-UNKNOWN');
    failure.primary = primary ?? null; failure.cleanup = caughtError(caught); failure.cleanupConfirmed = false;
    throw failure;
  } finally { clearTimeout(cleanupTimer); cleanup.abort(); }
  if (primary) {
    const failure = error('ADAPTER-FAILED'); failure.primary = primary; failure.cleanupConfirmed = true; throw failure;
  }
  if (signal?.aborted) { const failure = error('ABORTED'); failure.cleanupConfirmed = true; throw failure; }
  return result;
}
