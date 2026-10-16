import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { constants } from 'node:fs';
import { access, mkdir, realpath, writeFile } from 'node:fs/promises';
import { delimiter, dirname, isAbsolute, join } from 'node:path';
import { performance } from 'node:perf_hooks';
import { fileURLToPath } from 'node:url';
import { capture, descriptor } from './files.mjs';

const LIMIT = 1048576;
const deadlineRemaining = deadline => {
  const value = Math.ceil(deadline - performance.now());
  assert(value > 0, 'Owned process absolute deadline exceeded.');
  return value;
};
async function executablePath(program, env) {
  const candidates = isAbsolute(program) ? [program] : (env.PATH ?? env.Path ?? '').split(delimiter)
    .filter(Boolean).flatMap(root => process.platform === 'win32'
      ? [join(root, program), join(root, `${program}.exe`)] : [join(root, program)]);
  for (const candidate of candidates) {
    try {
      await access(candidate, constants.X_OK);
      return await realpath(candidate);
    } catch {}
  }
  throw new Error('Required trusted executable is unavailable.');
}
export async function launchOwned(program, args, options) {
  const env = options.env ?? process.env;
  const runtime = await executablePath(process.platform === 'win32'
    ? join(env.ProgramFiles, 'PowerShell', '7', 'pwsh.exe') : 'python3', env);
  const helperSource = fileURLToPath(new URL(process.platform === 'win32'
    ? './owner-windows.ps1' : './owner-linux.py', import.meta.url));
  const target = await executablePath(program, env);
  const pins = { runtime: await descriptor(runtime, options.deadline),
    helper: await descriptor(helperSource, options.deadline), target: await descriptor(target, options.deadline) };
  const material = join(options.output, `${options.name}.owner-material`);
  await mkdir(material, { mode: 0o700 });
  const helper = join(material, process.platform === 'win32' ? 'owner.ps1' : 'owner.py');
  pins.helperCapture = await capture(helperSource, helper, pins.helper, options.deadline);
  const helperArgs = process.platform === 'win32'
    ? ['-NoLogo', '-NoProfile', '-NonInteractive', '-File', helper] : ['-I', '-B', helper];
  const child = spawn(runtime, helperArgs, { cwd: options.cwd, env, shell: false,
    windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'] });
  const events = [], parts = [[], []], sizes = [0, 0], raw = [], stderr = [];
  let pending = Buffer.alloc(0), protocolBytes = 0, stderrBytes = 0;
  let failure, ended = null, complete, rootPid, ready, started = false, ownerClosed = false;
  let resolveClosed;
  const closed = new Promise(resolve => { resolveClosed = resolve; });
  const requestStop = () => {
    if (!child.stdin.destroyed) child.stdin.end('{"op":"stop"}\n');
  };
  const bad = message => { failure ??= new Error(message); requestStop(); };
  const receive = line => {
    let row;
    try { row = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(line)); }
    catch { bad('Malformed owner protocol.'); return; }
    if (!row || typeof row !== 'object' || typeof row.kind !== 'string') {
      bad('Owner event shape invalid.'); return;
    }
    if (row.kind === 'data') {
      if (![0, 1].includes(row.stream) || typeof row.data !== 'string') {
        bad('Owner data shape invalid.'); return;
      }
      const bytes = Buffer.from(row.data, 'base64');
      if (bytes.toString('base64') !== row.data) { bad('Owner data encoding invalid.'); return; }
      sizes[row.stream] += bytes.length;
      if (sizes[row.stream] > LIMIT) bad('Owned process stream cap exceeded.');
      else parts[row.stream].push(bytes);
    } else {
      if (!['ready', 'identity', 'started', 'image', 'gone', 'root-exit', 'complete', 'unknown', 'observation-scope', 'parent'].includes(row.kind)
          || events.length >= 4096) { bad('Owner event bound or kind invalid.'); return; }
      events.push(row);
      if (row.kind === 'ready') { if (ready) bad('Duplicate owner readiness.'); else ready = row; }
      if (row.kind === 'started') { rootPid = row.pid; started = true; }
      if (row.kind === 'root-exit') ended = { status: row.status, signal: null };
      if (row.kind === 'unknown') bad('Owner could not prove process cleanup.');
      if (row.kind === 'complete') {
        if (complete) bad('Duplicate owner completion.');
        complete = row;
      }
    }
  };
  child.stdout.on('data', bytes => {
    protocolBytes += bytes.length;
    if (protocolBytes > 4 * LIMIT) { bad('Owner protocol cap exceeded.'); return; }
    raw.push(bytes); pending = Buffer.concat([pending, bytes]);
    for (;;) {
      const index = pending.indexOf(10);
      if (index < 0) break;
      receive(pending.subarray(0, index)); pending = pending.subarray(index + 1);
    }
    if (pending.length > 262144) bad('Owner event cap exceeded.');
  });
  child.stderr.on('data', bytes => {
    stderrBytes += bytes.length;
    if (stderrBytes > LIMIT) bad('Owner stderr cap exceeded.'); else stderr.push(bytes);
  });
  child.on('error', () => { bad('Trusted process owner failed to launch.'); resolveClosed(); });
  child.on('close', (status, signal) => { ownerClosed = true; resolveClosed({ status, signal }); });
  const config = { program: target, args, cwd: options.cwd, stage: options.stage,
    timeoutMs: Math.min(600000, deadlineRemaining(options.deadline)), cleanupMs: 15000 };
  child.stdin.on('error', () => { failure ??= new Error('Owner control pipe unavailable.'); });
  child.stdin.write(`${JSON.stringify(config)}\n`);
  const timer = setTimeout(() => bad('Owned process absolute deadline exceeded.'),
    deadlineRemaining(options.deadline));
  async function waitUntil(predicate, deadline) {
    while (!predicate() && !failure) {
      if (ownerClosed) throw new Error('Owner closed before required protocol event.');
      deadlineRemaining(deadline);
      await Promise.race([closed, new Promise(resolve => setTimeout(resolve, 25))]);
    }
    if (failure) throw failure;
  }
  async function finish({ stop = false } = {}) {
    if (stop) requestStop();
    const cleanupDeadline = stop || ended || failure
      ? Math.min(options.deadline + 15000, performance.now() + 15000) : options.deadline + 15000;
    let ownerExit;
    try {
      ownerExit = await Promise.race([closed, new Promise(resolve => {
        const timer = setTimeout(() => resolve(null), Math.max(1, cleanupDeadline - performance.now()));
        closed.finally(() => clearTimeout(timer));
      })]);
      if (!ownerExit || pending.length || !complete || complete.cleanupConfirmed !== true
          || complete.status !== 0 || !Array.isArray(complete.failures) || complete.failures.length
          || ownerExit.status !== 0) failure ??= new Error('Owned process completion/cleanup unconfirmed.');
    } finally {
      clearTimeout(timer);
      // Unknown ownership remains a failure in the retained record. Closing pipe references
      // bounds this Node driver; it does not assert that an unresponsive owner was killed.
      child.stdin.destroy(); child.stdout.destroy(); child.stderr.destroy(); child.unref();
      await mkdir(options.output, { recursive: true });
      const record = { requested: { program, args, cwd: options.cwd }, supervisorPid: child.pid,
        rootPid, pins, config, ownerExit, complete: complete ?? null, events,
        stdoutBytes: sizes[0], stderrBytes: sizes[1],
        failure: failure ? failure.message : null, cleanupConfirmed: !failure,
        scope: 'Trusted installed-editor harness ownership; no hostile containment claim.' };
      await writeFile(join(options.output, `${options.name}.stdout`), Buffer.concat(parts[0]), { flag: 'wx' });
      await writeFile(join(options.output, `${options.name}.stderr`), Buffer.concat(parts[1]), { flag: 'wx' });
      await writeFile(join(options.output, `${options.name}.owner.jsonl`), Buffer.concat(raw), { flag: 'wx' });
      await writeFile(join(options.output, `${options.name}.owner.stderr`), Buffer.concat(stderr), { flag: 'wx' });
      await writeFile(join(options.output, `${options.name}.json`), `${JSON.stringify(record, null, 2)}\n`,
        { flag: 'wx' });
    }
    if (failure) throw failure;
    return { stdout: Buffer.concat(parts[0]).toString('utf8'), ended, events };
  }
  try {
    await waitUntil(() => ready, Math.min(options.deadline, performance.now() + 15000));
    assert.equal(ready.pid, child.pid);
    assert.equal(await realpath(ready.runtime), runtime);
    assert(Array.isArray(ready.dependencies) && ready.dependencies.length > 0 && ready.dependencies.length <= 256);
    pins.dependencies = [];
    for (const dependency of [...new Set(ready.dependencies)].sort()) {
      pins.dependencies.push(await descriptor(await realpath(dependency), options.deadline));
    }
    await writeFile(join(options.output, `${options.name}.before-start.json`),
      `${JSON.stringify({ supervisorPid: child.pid, pins, ready }, null, 2)}\n`, { flag: 'wx' });
    child.stdin.write('{"op":"start"}\n');
    await waitUntil(() => started, Math.min(options.deadline, performance.now() + 15000));
  } catch (error) {
    failure ??= error;
    await finish({ stop: true });
    throw error;
  }
  return { pid: rootPid, failure: () => failure, ended: () => ended, finish, closed };
}
