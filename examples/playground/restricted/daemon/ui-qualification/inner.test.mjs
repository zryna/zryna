// Startup boundary checks only. This test must never count a stub as a real Chrome result.
import assert from 'node:assert/strict';
import { PassThrough } from 'node:stream';
import { spawn } from 'node:child_process';
import test from 'node:test';
import { createFrameWriter, createFrameDecoder } from './framing.mjs';
import { runUiDriver } from './inner.mjs';
import { initialPlan, planBytes } from './plan.mjs';
import { sha256 } from './wire.mjs';
import { browserFailure } from './browser-probe.mjs';

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
test('real inert driver failure exits with parent stdin kept open and emits only a closed failure', { timeout: 4000 }, async () => {
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
    const records = [];const decoder = createFrameDecoder(value => records.push(value));
    decoder.push(Buffer.concat(output));decoder.end();
    assert.deepEqual(records, [{ version: 1, type: 'failure', runId: 'a'.repeat(32),
      phase: 'LOAD_CHROMIUM', cause: 'UNKNOWN', cleanupFailed: false, cleanup: { intercept: false, context: false, browser: false }, branch: 'UNSPECIFIED' }]);
    assert.equal(stderrBytes, 0);
  } finally {
    clearTimeout(timer); child.stdin.destroy();
    if (child.exitCode === null && child.signalCode === null) { child.kill('SIGKILL'); await closed; }
  }
});

test('browser startup failure retains only fixed phase/cause after cleanup', async () => {
  for (const cleanupFails of [false, true]) {
    const input = new PassThrough();const output = new PassThrough();const records = [];
    const decoder = createFrameDecoder(value => records.push(value));output.on('data', value => decoder.push(value));
    const chromium = cleanupFails ? { async launch() { return {
      async newContext() { throw new Error('private URL/#capability Bearer secret'); },
      async close() { throw new Error('private cleanup stderr'); },
    }; } } : { async launch() { const error = new Error('private URL/#capability Bearer secret');error.name = 'TimeoutError';throw error; } };
    const run = runUiDriver(input, output, async () => chromium, sha256(planBytes()));
    await createFrameWriter(input).write({ version: 1, type: 'start', runId: 'a'.repeat(32), origin: 'http://127.0.0.1:12345',
      launchPath: '/examples/playground/restricted/', capability: 'b'.repeat(64), planSha256: sha256(planBytes()) });
    await assert.rejects(run);decoder.end();
    assert.deepEqual(records, [{ version: 1, type: 'failure', runId: 'a'.repeat(32),
      phase: cleanupFails ? 'CONTEXT' : 'BROWSER_LAUNCH', cause: cleanupFails ? 'UNKNOWN' : 'TIMEOUT', cleanupFailed: cleanupFails, cleanup: { intercept: false, context: false, browser: cleanupFails }, branch: 'UNSPECIFIED' }]);
    assert.ok(!JSON.stringify(records).includes('private'));assert.ok(!JSON.stringify(records).includes('capability'));
  }
});
test('Playwright failure classification retains finite categories and drops URLs/capabilities/call logs', () => {
  for (const [message, cause] of [
    ['page.goto: net::ERR_FAILED at http://127.0.0.1:12345/#private-capability\nCall log: private', 'NET_ERR_FAILED'],
    ['page.goto: net::ERR_ABORTED; maybe frame was detached?', 'NET_ERR_ABORTED'],
    ['page.goto: Target page, context or browser has been closed', 'TARGET_CLOSED'],
    ['browserContext.close: Protocol error (Target.disposeBrowserContext): Target closed.', 'TARGET_CLOSED'],
    ['page.goto: Page crashed\nCall log: private', 'PAGE_CRASHED'],
    ['page.goto: Frame was detached', 'FRAME_DETACHED'],
    ['page.goto: Navigation to "private URL" is interrupted by another navigation to "secret"', 'NAVIGATION_INTERRUPTED'],
    ['locator.click: Error: strict mode violation: private locator', 'STRICT_MODE'],
    ['route.fulfill: Protocol error (Fetch.fulfillRequest): private details', 'PROTOCOL_ERROR'],
    ['arbitrary: net::ERR_FAILED at private URL', 'UNKNOWN'],
    ['page.goto: private net::ERR_FAILED text', 'UNKNOWN'],
  ]) {
    const observed = browserFailure(new Error(message));assert.equal(observed.cause, cause);
    const text = JSON.stringify(observed);for (const forbidden of ['private', 'secret', 'http:', 'Call log']) assert.ok(!text.includes(forbidden));
  }
});
test('navigation failure preserves its category and each failing cleanup operation separately', async () => {
  const input = new PassThrough();const output = new PassThrough();const records = [];
  const decoder = createFrameDecoder(value => records.push(value));output.on('data', value => decoder.push(value));
  const context = { on() {}, off() {}, async route() {},
    async unroute() { throw new Error('browserContext.unroute: Target page, context or browser has been closed'); },
    async close() { throw new Error('browserContext.close: private cleanup'); },
    async newPage() { return { setDefaultTimeout() {}, async goto() {
      throw new Error('page.goto: net::ERR_FAILED at http://private/#capability');
    } }; },
    async newCDPSession() { return { on() {}, async send() { return {}; } }; },
  };
  const browser = { async newContext() { return context; }, async close() { throw new Error('browser.close: private cleanup'); } };
  const run = runUiDriver(input, output, async () => ({ async launch() { return browser; } }), sha256(planBytes()));
  await createFrameWriter(input).write({ version: 1, type: 'start', runId: 'a'.repeat(32), origin: 'http://127.0.0.1:12345',
    launchPath: '/examples/playground/restricted/', capability: 'b'.repeat(64), planSha256: sha256(planBytes()) });
  await assert.rejects(run);decoder.end();
  assert.deepEqual(records, [{ version: 1, type: 'failure', runId: 'a'.repeat(32), phase: 'NAVIGATE', cause: 'NET_ERR_FAILED',
    cleanupFailed: true, cleanup: { intercept: true, context: true, browser: true }, branch: 'UNSPECIFIED' }]);
  assert.ok(!JSON.stringify(records).includes('private'));
});
