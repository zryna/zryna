'use strict';

const { spawn } = require('node:child_process');
const { isAbsolute, join } = require('node:path');
const pending = new Set();

function stopRuns() {
  for (const abort of [...pending]) abort();
}

function terminate(child) {
  if (!child) return;
  if (process.platform === 'win32' && child.pid && isAbsolute(process.env.SystemRoot ?? '')) {
    const killer = spawn(join(process.env.SystemRoot, 'System32', 'taskkill.exe'), ['/PID', String(child.pid), '/T', '/F'],
      { shell: false, windowsHide: true, stdio: 'ignore' });
    killer.on('error', () => child.kill());
    killer.on('exit', code => { if (code !== 0) child.kill(); });
  } else if (process.platform !== 'win32' && child.pid) {
    try { process.kill(-child.pid, 'SIGKILL'); } catch { child.kill(); }
  } else child.kill();
}

function cancelled(token) {
  if (token?.isCancellationRequested) throw new Error('Run cancelled.');
}

function compilerProcess(executable, args, cwd, token, launch = spawn) {
  cancelled(token);
  if (typeof executable !== 'string' || !isAbsolute(executable) || executable.includes('\0')
    || /\.(cmd|bat)$/i.test(executable)) throw new Error('Set zryna.compilerPath to an absolute installed compiler executable in user settings.');
  return new Promise((resolve, reject) => {
    let child;
    let timer;
    let cancellation;
    let settled = false;
    let size = 0;
    const stdout = [];
    const stderr = [];
    const abort = () => finish(new Error('Run cancelled.'));
    const finish = (error, result, kill = true) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      cancellation?.dispose();
      pending.delete(abort);
      if (error) { if (kill) terminate(child); reject(error); } else resolve(result);
    };
    try {
      child = launch(executable, args, { cwd, shell: false, windowsHide: true,
        detached: process.platform !== 'win32', stdio: ['ignore', 'pipe', 'pipe'] });
      pending.add(abort);
      for (const [stream, parts] of [[child.stdout, stdout], [child.stderr, stderr]]) {
        stream.on('data', bytes => {
          if (settled) return;
          size += bytes.length;
          if (size > 1024 * 1024) finish(new Error('Compiler output exceeded 1 MiB.'));
          else parts.push(Buffer.from(bytes));
        });
      }
      child.on('error', error => finish(new Error(`Could not start compiler: ${error.message}`)));
      child.on('close', code => {
        const out = Buffer.concat(stdout).toString('utf8');
        const err = Buffer.concat(stderr).toString('utf8');
        finish(code === 0 ? null : new Error(`Compiler exited ${code}.\n${err || out}`), { stdout: out, stderr: err }, false);
      });
      timer = setTimeout(() => finish(new Error('Compiler timed out after 120 seconds.')), 120000);
      cancellation = token?.onCancellationRequested(() => finish(new Error('Run cancelled.')));
      if (settled) cancellation?.dispose();
      if (token?.isCancellationRequested) finish(new Error('Run cancelled.'));
    } catch (error) { finish(error); }
  });
}

module.exports = { compilerProcess, cancelled, stopRuns };
