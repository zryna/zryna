// Trusted operator bootstrap only: fixed system Python, fixed reviewed owner, fixed19 CLI operations.
import childProcess from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { decodeJson } from '../../json.mjs';
import { canonical, exact, fail, freeze, sha256 } from './records.mjs';
import { findOperation, parityPlanSha256 } from './plan.mjs';

const main = fileURLToPath(new URL('./cli_main.py', import.meta.url));
const failure = reason => new Error(`PLAYGROUND-CLI-PARITY-OWNER-${reason}`);
export function createContainedCliAdapter(approval) {
  // Root must independently review this approval against the complete immutable360 image proof.
  exact(approval, ['format', 'version', 'image', 'layer_sha256', 'image_plan_sha256',
    'seccomp', 'seccomp_sha256', 'command_plan_sha256']);
  const initial = Buffer.from(JSON.stringify({ approval, operationId: 'x' }) + '\n');
  if (initial.length > 16384) fail();
  const trusted = JSON.parse(JSON.stringify(approval));
  if (trusted.command_plan_sha256 !== parityPlanSha256()) fail();
  const evidence = []; let active = null; let poisoned = false;
  const cancel = job => { if (!job.cancelled) { job.cancelled = true; job.child.stdio[3].end(); } };
  function decode(job) {
    if (job.overflow || job.stderr.length || !job.stdout.length) throw failure('STREAM');
    let value;
    try { value = decodeJson(Buffer.concat(job.stdout), 1048576); }
    catch { throw failure('RECEIPT'); }
    if (value.format !== 'zryna.playground-cli-owner-observation.v1' || value.version !== 1 ||
        value.operationId !== job.id || value.qualificationOnly !== true || value.cleanupConfirmed !== true ||
        value.ownershipUnconfirmed !== false || value.cleanupFailure !== null || value.removed !== true ||
        !Array.isArray(value.ownedVolumes) || value.ownedVolumes.length) throw failure('CLEANUP');
    job.record = value;
    return value;
  }
  const adapter = Object.freeze({
    async run(input, context) {
      if (active || poisoned || evidence.length >= 19 || canonical(input) !== canonical(findOperation(input?.id))) fail();
      exact(context, ['kind', 'id', 'signal', 'activeMs', 'cleanupMs']);
      if (context.kind !== 'cli' || context.id !== input.id || context.activeMs !== 10000 ||
          context.cleanupMs !== 2000 || !(context.signal instanceof AbortSignal)) fail();
      if (context.signal.aborted) throw failure('ABORTED');
      const body = Buffer.from(JSON.stringify({ approval: trusted, operationId: input.id }) + '\n');
      if (body.length > 16384) fail();
      const child = childProcess.spawn('/usr/bin/python3', ['-I', '-S', main], { cwd: path.dirname(main),
        env: { PATH: '/nonexistent', LC_ALL: 'C.UTF-8', HOME: '/nonexistent' },
        stdio: ['pipe', 'pipe', 'pipe', 'pipe'], windowsHide: true });
      const job = { child, id: input.id, stdout: [], stderr: [], stdoutBytes: 0, stderrBytes: 0,
        cancelled: false, overflow: false, closed: false, code: null, record: null };
      active = job;
      for (const stream of [child.stdin, child.stdout, child.stderr, child.stdio[3]]) {
        stream.on('error', () => { job.overflow = true; cancel(job); });
      }
      // Extra stdio pipes are socketpairs: consume EOF so original-child close is observable.
      child.stdio[3].on('data', bytes => { if (bytes.length) { job.overflow = true; cancel(job); } });
      const collect = (kind, cap) => bytes => {
        job[kind + 'Bytes'] += bytes.length;
        if (job[kind + 'Bytes'] > cap || job[kind].length >= (kind === 'stdout' ? 1024 : 64)) {
          job.overflow = true; cancel(job);
        }
        else job[kind].push(Buffer.from(bytes));
      };
      child.stdout.on('data', collect('stdout', 1048576)); child.stderr.on('data', collect('stderr', 4096));
      job.done = new Promise(resolve => {
        child.once('error', () => { job.overflow = true; cancel(job); });
        child.once('close', code => { job.closed = true; job.code = code; resolve(); });
      });
      const abort = () => cancel(job);
      context.signal.addEventListener('abort', abort, { once: true });
      if (context.signal.aborted) cancel(job);
      try {
        child.stdin.end(body);
        await job.done;
        const value = decode(job);
        if (context.signal.aborted || job.cancelled) throw failure('ABORTED');
        if (job.code !== 0 || value.failure !== null || value.operationStatus !== 'captured-source-outcome') throw failure('EXECUTION');
        const captured = value.exec?.capture;
        exact(captured, ['exitCode', 'stdoutBase64', 'stderrBase64', 'stdoutBytes', 'stderrBytes', 'stdoutSha256', 'stderrSha256']);
        if (typeof captured.stdoutBase64 !== 'string' || captured.stdoutBase64.length > 87384 ||
            typeof captured.stderrBase64 !== 'string' || captured.stderrBase64.length) throw failure('CAPTURE');
        const stdout = Buffer.from(captured.stdoutBase64, 'base64'); const stderr = Buffer.from(captured.stderrBase64, 'base64');
        if (![0, 3].includes(captured.exitCode) || stdout.length < 1 || stdout.length > 65536 || stderr.length ||
            stdout.toString('base64') !== captured.stdoutBase64 || stderr.toString('base64') !== captured.stderrBase64 ||
            stdout.length !== captured.stdoutBytes || captured.stderrBytes !== 0 ||
            sha256(stdout) !== captured.stdoutSha256 || sha256(stderr) !== captured.stderrSha256) throw failure('CAPTURE');
        return { exitCode: captured.exitCode, stdout, stderr };
      } finally { context.signal.removeEventListener('abort', abort); }
    },
    async cleanup(context) {
      const job = active;
      if (!job) return { confirmed: !poisoned };
      if (context.cancelled) cancel(job);
      let timer;
      let rejectAbort;
      const aborted = new Promise((_, reject) => { rejectAbort = reject; });
      const abort = () => { cancel(job); rejectAbort(failure('CLEANUP-DEADLINE')); };
      context.signal.addEventListener('abort', abort, { once: true });
      try {
        if (context.signal.aborted) abort();
        await Promise.race([job.done, aborted, new Promise((_, reject) => {
          timer = setTimeout(() => reject(failure('CLEANUP-DEADLINE')), 2000);
        })]);
        decode(job);
        if (!job.closed) throw failure('CLEANUP');
      } catch {
        poisoned = true;
        if (!job.closed) job.child.kill('SIGKILL');
      } finally {
        clearTimeout(timer);
        context.signal.removeEventListener('abort', abort);
        evidence.push(freeze({ operationId: job.id, ownerExitCode: job.code, ownerClosed: job.closed,
          cleanupConfirmed: !poisoned, stdoutBytes: job.stdoutBytes, stderrBytes: job.stderrBytes,
          stdoutBase64: Buffer.concat(job.stdout).toString('base64'), stderrBase64: Buffer.concat(job.stderr).toString('base64'),
          receipt: job.record }));
        active = null;
      }
      return { confirmed: !poisoned };
    },
  });
  return Object.freeze({ adapter, evidence: () => freeze([...evidence]), poisoned: () => poisoned });
}
