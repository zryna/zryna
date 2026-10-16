// Fixed observation of the existing browser only; no process launcher or policy selector.
import { ChildProcess } from 'node:child_process';
import { emptyBrowserProcess, browserProcessExit, browserProcessLog, scanBrowserFatal, fatalCandidate, inspectBrowserFatal } from './wire.mjs';

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
