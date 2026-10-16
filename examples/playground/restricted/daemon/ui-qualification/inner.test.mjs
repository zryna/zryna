// Startup boundary checks only. This test must never count a stub as a real Chrome result.
import assert from 'node:assert/strict';
import { PassThrough } from 'node:stream';
import { spawn, ChildProcess } from 'node:child_process';
import test from 'node:test';
import { createFrameWriter, createFrameDecoder } from './framing.mjs';
import { runUiDriver } from './inner.mjs';
import { initialPlan, planBytes } from './plan.mjs';
import { sha256, emptyBrowserProcess } from './wire.mjs';
import { createServer } from 'node:http';
import { EventEmitter } from 'node:events';
import { browserFailure, runInitialBrowserProbe, observeBrowserProcess } from './browser-probe.mjs';

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
  assert.equal(initialPlan.phase, 'initial-abort-corpus-ax');
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
      phase: 'LOAD_CHROMIUM', cause: 'UNKNOWN', cleanupFailed: false, cleanup: { intercept: false, context: false, browser: false }, lifecycle: { browserDisconnected: false, pageClosed: false, pageCrashed: false, contextClosed: false }, process: emptyBrowserProcess(), branch: 'UNSPECIFIED' }]);
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
    const chromium = cleanupFails ? { async launch() { return { on() {},
      async newContext() { throw new Error('private URL/#capability Bearer secret'); },
      async close() { throw new Error('private cleanup stderr'); },
    }; } } : { async launch() { const error = new Error('private URL/#capability Bearer secret');error.name = 'TimeoutError';throw error; } };
    const run = runUiDriver(input, output, async () => chromium, sha256(planBytes()));
    await createFrameWriter(input).write({ version: 1, type: 'start', runId: 'a'.repeat(32), origin: 'http://127.0.0.1:12345',
      launchPath: '/examples/playground/restricted/', capability: 'b'.repeat(64), planSha256: sha256(planBytes()) });
    await assert.rejects(run);decoder.end();
    assert.deepEqual(records, [{ version: 1, type: 'failure', runId: 'a'.repeat(32),
      phase: cleanupFails ? 'CONTEXT' : 'BROWSER_LAUNCH', cause: cleanupFails ? 'UNKNOWN' : 'TIMEOUT', cleanupFailed: cleanupFails, cleanup: { intercept: false, context: false, browser: cleanupFails }, lifecycle: { browserDisconnected: false, pageClosed: false, pageCrashed: false, contextClosed: false }, process: { ...emptyBrowserProcess(), diagnosticFailed: cleanupFails }, branch: 'UNSPECIFIED' }]);
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
test('navigation failure preserves category and independent context/browser cleanup bits', async () => {
  const input = new PassThrough();const output = new PassThrough();const records = [];
  const decoder = createFrameDecoder(value => records.push(value));output.on('data', value => decoder.push(value));
  const context = { on() {}, off() {},
    async close() { throw new Error('browserContext.close: private cleanup'); },
    async newPage() { return { on() {}, setDefaultTimeout() {}, async goto() {
      throw new Error('page.goto: net::ERR_FAILED at http://private/#capability');
    } }; },
    async newCDPSession() { return { on() {}, async send() { return {}; } }; },
  };
  const browser = { on() {}, async newContext() { return context; }, async close() { throw new Error('browser.close: private cleanup'); } };
  const reservation = createServer();await new Promise(resolve => reservation.listen(0, '127.0.0.1', resolve));
  const selectedOrigin = `http://127.0.0.1:${reservation.address().port}`;await new Promise(resolve => reservation.close(resolve));
  const run = runUiDriver(input, output, async () => ({ async launch() { return browser; } }), sha256(planBytes()));
  await createFrameWriter(input).write({ version: 1, type: 'start', runId: 'a'.repeat(32), origin: selectedOrigin,
    launchPath: '/examples/playground/restricted/', capability: 'b'.repeat(64), planSha256: sha256(planBytes()) });
  await assert.rejects(run);decoder.end();
  assert.deepEqual(records, [{ version: 1, type: 'failure', runId: 'a'.repeat(32), phase: 'NAVIGATE', cause: 'NET_ERR_FAILED',
    cleanupFailed: true, cleanup: { intercept: false, context: true, browser: true }, lifecycle: { browserDisconnected: false, pageClosed: false, pageCrashed: false, contextClosed: false }, process: { ...emptyBrowserProcess(), diagnosticFailed: true }, branch: 'UNSPECIFIED' }]);
  assert.ok(!JSON.stringify(records).includes('private'));
});

test('source readiness precedes distinct CDP stages and failure lifecycle excludes our cleanup events', async () => {
  for (const stage of ['SOURCE_READY', 'CDP_ATTACH', 'AX_ENABLE', 'DEBUGGER_ENABLE']) {
    const checkpoints = [];const calls = [];const browser = new EventEmitter();
    const context = new EventEmitter();const page = new EventEmitter();const cdp = new EventEmitter();
    const lifecycle = { browserDisconnected: false, pageClosed: false, pageCrashed: false, contextClosed: false };
    const failAt = value => {
      calls.push(value);if (value !== stage) return;
      if (stage === 'SOURCE_READY') { page.emit('crash');lifecycle.pageCrashed = true; }
      if (stage === 'CDP_ATTACH') { browser.emit('disconnected');lifecycle.browserDisconnected = true; }
      if (stage === 'AX_ENABLE') { page.emit('close');context.emit('close');lifecycle.pageClosed = lifecycle.contextClosed = true; }
      throw new Error((stage === 'SOURCE_READY' ? 'page.waitForFunction' : stage === 'CDP_ATTACH' ? 'browserContext.newCDPSession' : 'cdpSession.send') + ': Target page, context or browser has been closed');
    };
    Object.assign(browser, { async newContext() { return context; }, async close() { browser.emit('disconnected'); } });
    Object.assign(context, { async newPage() { context.emit('page', page);return page; },
      async newCDPSession(selected) { assert.equal(selected, page);failAt('CDP_ATTACH');return cdp; },
      async close() { context.emit('close');page.emit('close');page.emit('crash'); } });
    Object.assign(page, { setDefaultTimeout() {}, async goto() { calls.push('NAVIGATE'); },
      getByRole(role, options) { assert.equal(role, 'textbox');assert.equal(options.name, 'Source');return { async waitFor() { calls.push('SOURCE_VISIBLE'); } }; },
      async waitForFunction() { failAt('SOURCE_READY'); } });
    cdp.send = async method => failAt(method === 'Accessibility.enable' ? 'AX_ENABLE' : 'DEBUGGER_ENABLE');
    const reservation = createServer();await new Promise(resolve => reservation.listen(0, '127.0.0.1', resolve));
    const origin = `http://127.0.0.1:${reservation.address().port}`;await new Promise(resolve => reservation.close(resolve));
    const configuration = { version: 1, type: 'start', runId: 'a'.repeat(32), origin,
      launchPath: '/examples/playground/restricted/', capability: 'b'.repeat(64), planSha256: sha256(planBytes()) };
    let caught;
    try { await runInitialBrowserProbe({ chromium: { async launch() { return browser; } }, configuration,
      send() {}, register() {}, checkpoint(value) { checkpoints.push(value); } }); }
    catch (error) { caught = browserFailure(error); }
    assert.equal(checkpoints.at(-1), stage);assert.equal(caught.cause, 'TARGET_CLOSED');
    assert.deepEqual(caught.lifecycle, lifecycle);assert.equal(caught.cleanupFailed, false);
    assert.deepEqual(calls, ['NAVIGATE', 'SOURCE_VISIBLE', 'SOURCE_READY', 'CDP_ATTACH', 'AX_ENABLE', 'DEBUGGER_ENABLE'].slice(0, ['SOURCE_READY', 'CDP_ATTACH', 'AX_ENABLE', 'DEBUGGER_ENABLE'].indexOf(stage) + 3));
    assert.ok(!JSON.stringify(caught).includes('Target page'));
  }
});

// Constructed ChildProcess/EventEmitter fixtures observe no actual Chrome and qualify no runtime.
function processFixture() {
  const child = new ChildProcess();child.pid = 123;child.spawnfile = '/materials/browser/chrome-linux64/chrome';
  const messages = [];let listener;const implementation = { options: { browserProcess: { process: child },
    browserLogsCollector: { recentLogs() { return messages; }, onMessage(value) { listener = value;for (const line of messages) listener(line); } } } };
  const browser = { _connection: { toImpl(selected) { assert.equal(selected, browser);return implementation; } } };
  return { browser, child, implementation, log(line) { messages.push(line);listener(line); } };
}
test('pinned process observer keeps only native-exit enums and bounded stderr categories before cleanup', () => {
  const selected = processFixture();const stop = observeBrowserProcess(selected.browser);
  selected.log('[pid=124][err] FATAL: private foreign process');
  selected.log('[pid=123][err] unknown private http://127.0.0.1/#capability Authorization Bearer secret');
  for (let index = 0; index < 20; index++) selected.log('[pid=123][err] ordinary ' + 'x'.repeat(1000));
  selected.log('[pid=123][err] FATAL: pthread_create: Resource temporarily unavailable');
  selected.log('[pid=123][err] [123:124:1016/073044.542269:FATAL:zygote_host_impl_linux.cc(202)] Check failed: private');
  for (let index = 0; index < 220; index++) selected.log('[pid=123][err] stack private');
  selected.child.emit('exit', null, 'SIGTRAP');const result = stop();
  assert.equal(result.attached, true);assert.equal(result.identity, 'STABLE');assert.equal(result.exitBeforeCleanup, true);
  assert.equal(result.exitCode, null);assert.equal(result.signal, 'SIGTRAP');assert.equal(result.stderrUnknown, true);
  assert.deepEqual(result.stderrCategories, ['FATAL_CHECK', 'RESOURCE_UNAVAILABLE', 'SANDBOX', 'THREAD_CREATE']);assert.equal(result.stderrTruncated, true);assert.equal(result.fatalScan, 'TRUNCATED');assert.equal(result.fatalLive, 'COMPLETE');
  assert.deepEqual(result.fatalLocations, [{ source: 'zygote_host_impl_linux.cc', line: 202 }]);
  selected.child.emit('exit', 0, null);selected.log('[pid=123][err] FATAL: No usable sandbox!');
  assert.deepEqual(stop(), result);assert.ok(!/private|capability|Bearer|http:/.test(JSON.stringify(result)));
});
test('observer distinguishes missing mapping, detached dispatcher, original process mutation, and unobserved exit', () => {
  const missing = observeBrowserProcess({})();assert.equal(missing.attached, false);assert.equal(missing.diagnosticFailed, true);
  for (const change of ['detached', 'pid', 'replacement', 'oversized', 'aggregate', 'mapping']) {
    const selected = processFixture();const stop = observeBrowserProcess(selected.browser);
    if (change === 'detached') selected.browser._connection.toImpl = () => { throw new Error('disposed'); };
    if (change === 'mapping') selected.browser._connection.toImpl = () => ({ ...selected.implementation });
    if (change === 'aggregate') for (let index = 0; index < 20; index++) selected.log('[pid=123][err] ' + 'private'.repeat(140));
    if (change === 'pid') selected.child.pid++;
    if (change === 'replacement') selected.implementation.options.browserProcess.process = new ChildProcess();
    if (change === 'oversized') selected.log('[pid=123][err] ' + 'private'.repeat(600));
    const result = stop();assert.equal(result.identity, change === 'detached' ? 'DETACHED' : ['pid', 'replacement', 'mapping'].includes(change) ? 'CHANGED' : 'STABLE');
    assert.equal(result.exitBeforeCleanup, false);assert.equal(result.exitCode, null);assert.equal(result.signal, 'NONE');
    assert.equal(result.stderrTruncated, ['oversized', 'aggregate'].includes(change));
  }
  const wrongShape = processFixture();wrongShape.implementation._browserLogsCollector = wrongShape.implementation.options.browserLogsCollector;
  delete wrongShape.implementation.options.browserLogsCollector;assert.equal(observeBrowserProcess(wrongShape.browser)().attached, false);
  const bad = processFixture();bad.child.spawnfile = '/unapproved';assert.equal(observeBrowserProcess(bad.browser)().attached, false);
});
