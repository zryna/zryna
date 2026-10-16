// Startup boundary checks only. This test must never count a stub as a real Chrome result.
import assert from 'node:assert/strict';
import { PassThrough } from 'node:stream';
import { spawn } from 'node:child_process';
import test from 'node:test';
import { createFrameWriter } from './framing.mjs';
import { runUiDriver } from './inner.mjs';
import { initialPlan, planBytes } from './plan.mjs';
import { sha256 } from './wire.mjs';

test('fixed image plan mismatch rejects before loading browser tools and stdout remains empty', async () => {
  const input = new PassThrough(); const output = new PassThrough(); const observed = [];
  output.on('data', bytes => observed.push(bytes));
  let loads = 0;
  const run = runUiDriver(input, output, async () => { loads++; throw new Error('must not load'); }, sha256(planBytes()));
  const writer = createFrameWriter(input);
  await writer.write({ version: 1, type: 'start', runId: 'a'.repeat(32), origin: 'http://127.0.0.1:12345',
    launchPath: '/examples/playground/restricted/', capability: 'b'.repeat(64), planSha256: 'c'.repeat(64) });
  await assert.rejects(run, /UI-TRANSPORT/);
  assert.equal(loads, 0); assert.equal(Buffer.concat(observed).length, 0);
  assert.equal(initialPlan.phase, 'initial-abort-static-ax');
});
test('partial startup EOF cannot start Chromium or produce a passing result', async () => {
  const input = new PassThrough(); const output = new PassThrough(); let loads = 0;
  const run = runUiDriver(input, output, async () => { loads++; }, sha256(planBytes()));
  input.end(Buffer.from([1, 0]));
  await assert.rejects(run, /UI-TRANSPORT/); assert.equal(loads, 0);
});
test('real inert driver failure exits with parent stdin kept open and emits no result', { timeout: 4000 }, async () => {
  const moduleUrl = new URL('./inner.mjs', import.meta.url).href;
  const code = `import { runUiDriver } from ${JSON.stringify(moduleUrl)};
    runUiDriver(process.stdin,process.stdout,async()=>{throw new Error('inert no-browser startup');},
      ${JSON.stringify(sha256(planBytes()))}).catch(()=>{process.exitCode=7;});`;
  const child = spawn(process.execPath, ['--input-type=module', '-e', code], {
    cwd: '/', env: { LANG: 'C.UTF-8', LC_ALL: 'C.UTF-8', TZ: 'UTC' }, stdio: ['pipe', 'pipe', 'pipe'],
  });
  const output = []; let stderrBytes = 0;
  child.stdout.on('data', bytes => output.push(bytes));
  child.stderr.on('data', bytes => { stderrBytes += bytes.length; if (stderrBytes > 4096) child.kill('SIGKILL'); });
  const closed = new Promise((resolve, reject) => {
    child.once('error', reject); child.once('close', (status, signal) => resolve({ status, signal }));
  });
  const timer = setTimeout(() => child.kill('SIGKILL'), 1500);
  try {
    const writer = createFrameWriter(child.stdin);
    await writer.write({ version: 1, type: 'start', runId: 'a'.repeat(32), origin: 'http://127.0.0.1:12345',
      launchPath: '/examples/playground/restricted/', capability: 'b'.repeat(64), planSha256: sha256(planBytes()) });
    // Deliberately retain the outer stdin pipe: the driver must stop its own read handle.
    assert.deepEqual(await closed, { status: 7, signal: null });
    assert.equal(Buffer.concat(output).length, 0); assert.equal(stderrBytes, 0);
  } finally {
    clearTimeout(timer); child.stdin.destroy();
    if (child.exitCode === null && child.signalCode === null) { child.kill('SIGKILL'); await closed; }
  }
});
