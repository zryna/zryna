// Fixed pinned host executable; one framed operation and cancellation byte, never a command API.
import { spawn } from 'node:child_process';
import { closeSync } from 'node:fs';
import { capture, stable } from './bridge-material.mjs';
import { decodeSourceRequest } from '../protocol.mjs';
import { decodeJson } from '../json.mjs';
import { validateObservation } from '../evaluation.mjs';
import { exact, fail, limits } from '../limits.mjs';

export function decodeHostFailure(bytes) {
  const value = decodeJson(bytes, 4096);
  exact(value, ['format', 'version', 'reason', 'cleanupConfirmed', 'owner', 'primary', 'cleanup']);
  const code = input => typeof input === 'string' && /^PLAYGROUND-[A-Z0-9-]{1,100}$/.test(input);
  if (value.format !== 'zryna.playground-host-error.v1' || value.version !== 1 || !code(value.reason) ||
      typeof value.cleanupConfirmed !== 'boolean' ||
      value.owner !== null && (typeof value.owner !== 'string' || !/^[a-f0-9]{32}$/.test(value.owner)) ||
      value.primary !== null && !code(value.primary) || value.cleanup !== null && !code(value.cleanup) ||
      value.cleanupConfirmed && (value.owner !== null || value.cleanup !== null)) fail('HOST-TERMINAL');
  return Object.freeze(value);
}

function hostFailure(record, message = record.reason) {
  const error = new Error(message);
  error.hostFailure = record;
  error.owner = record.owner; error.primary = record.primary; error.cleanup = record.cleanup;
  error.cleanupConfirmed = record.cleanupConfirmed;
  return error;
}

export function createDaemonBridge(policy) {
  exact(policy, ['compiler', 'approval']);
  const compiler = capture(policy.compiler, true);
  let approval;
  try { approval = capture(policy.approval, false); }
  catch (error) { closeSync(compiler.fd); throw error; }
  let active = null;
  let poisoned = false;
  let closed = false;
  let recovery = null;
  const poison = error => {
    poisoned = true;
    if (!recovery || !recovery.owner && error.owner) recovery = error;
    return error;
  };

  function start(operation, bytes, signal, token) {
    if (closed || poisoned) fail('HOST-TEARDOWN');
    if (active) fail('BUSY');
    if (typeof token !== 'string' || !/^[a-f0-9]{64}$/.test(token) || !(bytes instanceof Uint8Array) ||
        bytes.length < 1 || bytes.length > (operation === 'compile' ? limits.request : 2097152)) fail('REQUEST');
    if (signal?.aborted) fail('CANCELLED');
    stable(compiler); stable(approval);
    const cap = operation === 'compile' ? limits.frame : 16384;
    const duration = operation === 'compile' ? limits.compileMs : limits.evaluateMs;
    const frame = Buffer.alloc(4 + bytes.length);
    frame.writeUInt32LE(bytes.length); frame.set(bytes, 4);
    const job = { token, child: null, cancelled: null, closed: false, promise: null, cancel: null };
    active = job;
    job.promise = new Promise((resolve, reject) => {
      const chunks = []; const errors = []; let stdoutBytes = 0; let stderrBytes = 0;
      let deadline; let cleanup; let reap;
      let settled = false;
      const done = error => {
        if (settled) return;
        settled = true;
        clearTimeout(deadline); clearTimeout(cleanup); clearTimeout(reap);
        signal?.removeEventListener('abort', aborted);
        job.child?.stdin.destroy(); job.child?.stdout.destroy(); job.child?.stderr.destroy();
        if (error) { if (poisoned) recovery ??= error; reject(error); }
        else resolve(new Uint8Array(Buffer.concat(chunks)));
      };
      const uncertain = () => {
        let error = new Error('PLAYGROUND-HOST-TEARDOWN');
        try {
          const record = decodeHostFailure(Buffer.concat(errors));
          if (!record.cleanupConfirmed) error = hostFailure(record, error.message);
        } catch { /* unframed bytes never acknowledge cleanup */ }
        return poison(error);
      };
      const force = () => {
        // Killing a host without its removal acknowledgement leaves unknown daemon ownership.
        poisoned = true;
        job.child?.kill('SIGKILL');
        reap = setTimeout(() => done(uncertain()), 100);
      };
      job.cancel = reason => {
        if (job.closed || job.cancelled) return;
        job.cancelled = reason;
        clearTimeout(deadline);
        try { job.child?.stdin.write(Buffer.from([1])); } catch { /* timeout still owns recovery */ }
        cleanup = setTimeout(force, limits.teardownMs);
      };
      const aborted = () => job.cancel('PLAYGROUND-CANCELLED');
      deadline = setTimeout(() => job.cancel('PLAYGROUND-HOST-TIMEOUT'), duration);
      signal?.addEventListener('abort', aborted, { once: true });
      try {
        // Inherited descriptor 3 executes the captured inode; no PATH, shell or ambient selectors.
        job.child = spawn('/proc/self/fd/3', ['--daemon-approval', approval.path,
          '--approval-sha256', approval.sha256, '--operation', operation], {
          shell: false, cwd: '/', env: { LANG: 'C.UTF-8', LC_ALL: 'C.UTF-8', TZ: 'UTC' },
          stdio: ['pipe', 'pipe', 'pipe', compiler.fd], windowsHide: true,
        });
      } catch { job.closed = true; done(new Error('PLAYGROUND-HOST-LAUNCH')); return; }
      job.child.stdin.on('error', () => job.cancel('PLAYGROUND-HOST-PIPE'));
      job.child.stdout.on('data', chunk => {
        stdoutBytes += chunk.length;
        if (stdoutBytes > cap) job.cancel('PLAYGROUND-HOST-OUTPUT-LIMIT');
        else chunks.push(chunk);
      });
      job.child.stderr.on('data', chunk => {
        stderrBytes += chunk.length;
        if (stderrBytes > 4096) job.cancel('PLAYGROUND-HOST-OUTPUT-LIMIT');
        else errors.push(chunk);
      });
      job.child.on('error', () => {
        // Spawn failure has no child authority. A post-spawn error cannot prove daemon removal.
        if (job.child.pid) { poisoned = true; job.cancel('PLAYGROUND-HOST-TEARDOWN'); }
        else { job.closed = true; done(new Error('PLAYGROUND-HOST-LAUNCH')); }
      });
      job.child.on('close', (code, killed) => {
        job.closed = true;
        try { stable(compiler); stable(approval); }
        catch { poisoned = true; done(new Error('PLAYGROUND-HOST-MATERIAL')); return; }
        if (poisoned || killed) { done(uncertain()); return; }
        if (code === 0 && stdoutBytes > 0 && stderrBytes === 0 && stdoutBytes <= cap) {
          if (job.cancelled) done(new Error(job.cancelled)); else done();
          return;
        }
        let record;
        try {
          if (code !== 1 || stdoutBytes !== 0 || stderrBytes > 4096) fail('HOST-TERMINAL');
          record = decodeHostFailure(Buffer.concat(errors));
        } catch { done(poison(new Error('PLAYGROUND-HOST-TEARDOWN'))); return; }
        // Cleanup acknowledgement wins over cancellation; never erase a retained owner.
        if (!record.cleanupConfirmed) {
          done(poison(hostFailure(record, 'PLAYGROUND-HOST-TEARDOWN'))); return;
        }
        done(hostFailure(record, job.cancelled ?? record.reason));
      });
      job.child.stdin.write(frame, error => { if (error) job.cancel('PLAYGROUND-HOST-PIPE'); });
      // Deliberately do not end stdin: EOF is a cancellation signal to the host.
      if (signal?.aborted) aborted();
    });
    return job.promise;
  }

  return Object.freeze({
    compile(bytes, signal, token) {
      decodeSourceRequest(bytes);
      return start('compile', bytes, signal, token);
    },
    evaluate(token, payload, signal) {
      exact(payload, ['sourceRequest', 'frameBytes', 'selection', 'receipt', 'authority']);
      if (!(payload.frameBytes instanceof Uint8Array) || payload.frameBytes.length > limits.frame) fail('FRAME');
      const wire = { version: 1, sourceRequest: payload.sourceRequest,
        frameBase64: Buffer.from(payload.frameBytes).toString('base64'), selection: payload.selection,
        receipt: payload.receipt, authority: payload.authority };
      return start('evaluate', Buffer.from(JSON.stringify(wire)), signal, token)
        .then(bytes => validateObservation(decodeJson(bytes, 16384), payload.selection));
    },
    async finish(token, cancelled) {
      if (closed || poisoned) fail('HOST-TEARDOWN');
      if (!active || active.token !== token || typeof cancelled !== 'boolean') fail('JOB');
      const job = active;
      if (cancelled) job.cancel('PLAYGROUND-CANCELLED');
      try { await job.promise; }
      catch (error) {
        if (!job.closed || poisoned) { poisoned = true; fail('HOST-TEARDOWN'); }
        if (!cancelled) throw error;
      }
      if (!job.closed || poisoned) { poisoned = true; fail('HOST-TEARDOWN'); }
      if (active === job) active = null;
    },
    recovery() { return recovery; },
    idle() { return !closed && !poisoned && active === null; },
    async close() {
      if (closed) return;
      let error = poisoned ? new Error('PLAYGROUND-HOST-TEARDOWN') : undefined;
      try {
        if (active) {
          active.cancel('PLAYGROUND-CANCELLED');
          try { await active.promise; } catch { /* removal status is checked below */ }
          if (!active.closed || poisoned) error = new Error('PLAYGROUND-HOST-TEARDOWN');
          active = null;
        }
      } finally {
        closed = true;
        try { closeSync(approval.fd); } catch { error = new Error('PLAYGROUND-HOST-TEARDOWN'); }
        try { closeSync(compiler.fd); } catch { error = new Error('PLAYGROUND-HOST-TEARDOWN'); }
      }
      if (error) throw error;
    },
  });
}
