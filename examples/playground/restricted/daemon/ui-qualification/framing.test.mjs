import assert from 'node:assert/strict';
import { PassThrough, Writable } from 'node:stream';
import test from 'node:test';
import { ChildProcess } from 'node:child_process';
import { observeBrowserProcess } from './browser-diagnostics.mjs';
import { bounds, binary, decode, emptyBrowserProcess, validateBrowserProcess, browserProcessExit, browserProcessLog, scanBrowserFatal, inspectBrowserFatal } from './wire.mjs';
import { createFrameDecoder, createFrameWriter } from './framing.mjs';
import { createEngineDemux } from './engine-stream.mjs';

const frame = text => {
  const body = Buffer.from(text); const result = Buffer.alloc(body.length + 4);
  result.writeUInt32LE(body.length); result.set(body, 4); return result;
};
test('partial real stream writes preserve canonical frames and EOF rejects incomplete input', async () => {
  const bytes = []; const output = new PassThrough(); output.on('data', value => bytes.push(value));
  const writer = createFrameWriter(output);
  await writer.write({ version: 1, type: 'abort', id: 1 });
  await writer.write({ version: 1, type: 'aborted', id: 1 }); await writer.stop();
  const observed = []; const decoder = createFrameDecoder(value => observed.push(value));
  for (const byte of Buffer.concat(bytes)) decoder.push(Buffer.from([byte]));
  decoder.end(); assert.deepEqual(observed.map(value => value.type), ['abort', 'aborted']);
  assert.throws(() => decoder.push(Buffer.alloc(0)), /UI-TRANSPORT/);
  for (const partial of [Buffer.from([3]), Buffer.from([3, 0, 0, 0, 123])]) {
    const incomplete = createFrameDecoder(() => assert.fail()); incomplete.push(partial);
    assert.throws(() => incomplete.end(), /UI-TRANSPORT/);
  }
});
test('oversized length, noncanonical JSON and base64, invalid UTF-8 fail before dispatch', () => {
  for (const value of ['{"x":1,"x":1}', '{"x":-0}', '{"x":1e0}', ' {"x":1}', '{"x":1.5}']) {
    assert.throws(() => decode(Buffer.from(value)), /UI-TRANSPORT/);
  }
  assert.throws(() => decode(Buffer.from([255])), /UI-TRANSPORT/);
  assert.throws(() => decode(Buffer.concat([Buffer.from([239, 187, 191]), Buffer.from('{"version":1}')])), /UI-TRANSPORT/);
  assert.throws(() => binary('YR==', 1), /UI-TRANSPORT/);
  assert.throws(() => binary('YQ==', 0), /UI-TRANSPORT/);
  assert.equal(binary('YQ==', 1).toString(), 'a');
  const size = Buffer.alloc(4); size.writeUInt32LE(bounds.frame + 1);
  assert.throws(() => createFrameDecoder(() => assert.fail()).push(size), /UI-TRANSPORT/);
  assert.throws(() => createFrameDecoder(() => assert.fail()).push(frame('{"x":1,"x":2}')), /UI-TRANSPORT/);
});
test('engine stdout/stderr demultiplex independently under fragmented headers and bounded stderr', () => {
  const out = []; const errors = [];
  const demux = createEngineDemux(value => out.push(Buffer.from(value)), value => errors.push(Buffer.from(value)));
  const packet = (channel, value) => {
    const body = Buffer.from(value); const bytes = Buffer.alloc(8 + body.length);
    bytes[0] = channel; bytes.writeUInt32BE(body.length, 4); bytes.set(body, 8); return bytes;
  };
  for (const byte of Buffer.concat([packet(1, 'abc'), packet(2, 'error'), packet(1, 'def')])) {
    demux.push(Buffer.from([byte]));
  }
  demux.end(); assert.equal(Buffer.concat(out).toString(), 'abcdef'); assert.equal(Buffer.concat(errors).toString(), 'error');
  assert.throws(() => demux.push(packet(1, 'x')), /UI-TRANSPORT/);
  assert.throws(() => createEngineDemux(() => {}, () => {}).push(packet(0, 'x')), /UI-TRANSPORT/);
  assert.throws(() => createEngineDemux(() => {}, () => {}).push(packet(2, 'x'.repeat(bounds.stderr + 1))), /UI-TRANSPORT/);
  const partial = createEngineDemux(() => {}, () => {}); partial.push(packet(1, 'x').subarray(0, 8));
  assert.throws(() => partial.end(), /UI-TRANSPORT/);
});
test('writer failure persists through stop and never accepts another frame', async () => {
  let writes = 0;
  const output = new Writable({ write(_bytes, _encoding, callback) { writes++; callback(new Error('inert pipe failure')); } });
  output.on('error', () => {});
  const writer = createFrameWriter(output);
  const first = writer.write({ version: 1 }); const queued = writer.write({ version: 1 });
  await Promise.all([assert.rejects(first, /inert pipe failure/), assert.rejects(queued, /inert pipe failure/)]);
  assert.equal(writes, 1);
  await assert.rejects(writer.stop(), /inert pipe failure/);
  assert.throws(() => writer.write({ version: 1 }), /UI-TRANSPORT/);
});

test('browser process diagnostics are closed, finite, and never retain stderr text or arbitrary signals', () => {
  const value = { ...emptyBrowserProcess(), attached: true, identity: 'STABLE' };
  browserProcessExit(value, 7, null);assert.equal(validateBrowserProcess(value).exitCode, 7);
  const signalled = { ...emptyBrowserProcess(), attached: true, identity: 'DETACHED' };
  browserProcessExit(signalled, null, 'private-signal');assert.equal(validateBrowserProcess(signalled).signal, 'UNKNOWN');
  for (const [text, category] of [['No usable sandbox!', 'SANDBOX'], ['pthread_create: Resource temporarily unavailable', 'THREAD_CREATE'],
    ['Cannot allocate memory', 'MEMORY_ALLOCATION'], ['/dev/shm: No space left on device', 'SHARED_MEMORY'],
    ['Bad system call', 'SECCOMP'], ['Received signal 11', 'CRASH_SIGNAL'], ['Check failed: private details', 'FATAL_CHECK']]) {
    const state = { ...emptyBrowserProcess(), attached: true, identity: 'STABLE' };browserProcessLog(state, text);
    assert.deepEqual(validateBrowserProcess(state).stderrCategories, [category]);assert.ok(!JSON.stringify(state).includes('private'));
  }
  for (const extra of [{ exitCode: -1 }, { exitCode: 256 }, { signal: 'Bearer secret' }, { attached: 1 },
    { identity: 'private mapping' }, { raw: 'private stderr' }, { stderrCategories: ['PRIVATE'] },
    { stderrCategories: ['SANDBOX', 'SANDBOX'] }, { stderrCategories: null }, { stderrSeen: false, stderrCategories: ['SANDBOX'] },
    { exitBeforeCleanup: false, exitCode: 0 }, { exitCode: 0, signal: 'SIGTRAP' }]) {
    assert.throws(() => validateBrowserProcess({ ...value, ...extra }), /UI-TRANSPORT/);
  }
});

test('final fatal scan sees bounded last messages after noise saturation and retains only source basename/line', () => {
  const value = { ...emptyBrowserProcess(), attached: true, identity: 'DETACHED', stderrTruncated: true };
  const logs = Array.from({ length: 200 }, () => '[pid=123][err] ordinary private noise');
  logs.push('[pid=123][err] [123:124:1016/073044.542269:FATAL:base/threading/platform_thread_posix.cc:202] ' +
    'http://127.0.0.1/#capability Bearer secret'.repeat(100) +
    ' libc++abi: terminating due to uncaught exception of type system_error: thread constructor failed: Permission denied');
  scanBrowserFatal(value, logs, '[pid=123][err] ');
  assert.equal(value.fatalScan, 'TRUNCATED');
  assert.deepEqual(value.fatalLocations, [{ source: 'platform_thread_posix.cc', line: 202 }]);
  assert.deepEqual(value.stderrCategories, ['CXX_UNCAUGHT', 'FATAL_CHECK', 'PERMISSION_DENIED', 'THREAD_CREATE']);
  validateBrowserProcess(value);assert.ok(!/private|capability|Bearer|secret|http:/.test(JSON.stringify(value)));
});
test('fatal source parser rejects unanchored headers, URLs, suffix injections, foreign processes and invalid collectors', () => {
  for (const text of ['[INFO] private [123:124:1016/073044.542269:FATAL:base/secret.cc:1]',
    '[123:124:1016/073044.542269:FATAL:http://private/secret.cc:1]',
    '[123:124:1016/073044.542269:FATAL:base/secret.mjs:1]',
    '[123:124:1016/073044.542269:FATAL:base/secret.cc:0]',
    'x'.repeat(2048) + '[123:124:1016/073044.542269:FATAL:base/secret.cc:1]']) {
    const value = { ...emptyBrowserProcess(), attached: true, identity: 'STABLE' };
    scanBrowserFatal(value, ['[pid=123][err] ' + text], '[pid=123][err] ');
    assert.deepEqual(value.fatalLocations, []);
  }
  const foreign = { ...emptyBrowserProcess(), attached: true, identity: 'STABLE' };
  scanBrowserFatal(foreign, ['[pid=124][err] FATAL: No usable sandbox!'], '[pid=123][err] ');
  assert.equal(foreign.stderrSeen, false);assert.deepEqual(foreign.stderrCategories, []);
  scanBrowserFatal(foreign, {}, '[pid=123][err] ');assert.equal(foreign.fatalScan, 'UNAVAILABLE');
  scanBrowserFatal(foreign, Array(301).fill(''), '[pid=123][err] ');assert.equal(foreign.fatalScan, 'UNAVAILABLE');
});
test('fatal diagnostic projection rejects arbitrary paths, raw strings, duplicates and oversized locations', () => {
  const value = { ...emptyBrowserProcess(), attached: true, identity: 'STABLE', stderrSeen: true, fatalScan: 'COMPLETE' };
  for (const extra of [{ fatalLive: 'private' }, { fatalHeader: 'private' }, { fatalHeader: 'PARSED' }, { fatalScan: 'private' }, { fatalLocations: null }, { fatalLocations: [{ source: '/private/x.cc', line: 1 }] },
    { fatalLocations: [{ source: 'x.cc', line: 0 }] }, { fatalLocations: [{ source: 'x.cc', line: 1000000 }] },
    { fatalLocations: [{ source: 'x.cc', line: 1, raw: 'private' }] },
    { fatalLocations: [{ source: 'x.cc', line: 1 }, { source: 'x.cc', line: 1 }] },
    { fatalLocations: Array.from({ length: 5 }, (_, index) => ({ source: 'x.cc', line: index + 1 })) }]) {
    assert.throws(() => validateBrowserProcess({ ...value, ...extra }), /UI-TRANSPORT/);
  }
});

test('structural Chromium headers accept basename and safe relative paths while standard abort forms stay closed', () => {
  for (const header of ['[123:124:1016/073044.542269:FATAL:zygote_host_impl_linux.cc(202)]',
    '[1016/073044.542:FATAL:headless/lib/browser/headless_browser_impl.cc:202]',
    '[FATAL:../../base/memory/raw_ptr.h(202)]']) {
    const value = { ...emptyBrowserProcess(), attached: true, identity: 'STABLE', stderrSeen: true };
    inspectBrowserFatal(value, header + ' Check failed: private http://host/#capability', '');
    assert.equal(value.fatalHeader, 'PARSED');assert.equal(value.fatalLocations[0].line, 202);
    validateBrowserProcess(value);assert.ok(!/private|capability|http:/.test(JSON.stringify(value)));
  }
  for (const [text, wanted] of [['chrome: file.cc:42: Assertion `private` failed.', 'C_ASSERT'],
    ['terminate called after throwing an instance of std::bad_alloc', 'STD_TERMINATE'],
    ['libc++abi: terminating with uncaught exception of type std::bad_alloc', 'CXX_UNCAUGHT'],
    ['libc++abi: terminating due to uncaught exception of type std::bad_alloc', 'BAD_ALLOC']]) {
    const value = { ...emptyBrowserProcess(), attached: true, identity: 'STABLE', stderrSeen: true };
    inspectBrowserFatal(value, text, '');assert.ok(value.stderrCategories.includes(wanted));assert.deepEqual(value.fatalLocations, []);
  }
  const unknown = { ...emptyBrowserProcess(), attached: true, identity: 'STABLE', stderrSeen: true };
  inspectBrowserFatal(unknown, '[FATAL:http://private/capability.cc:42] Check failed:', '');
  assert.equal(unknown.fatalHeader, 'UNPARSED');assert.deepEqual(unknown.fatalLocations, []);
});

test('live fatal diagnostics enforce separate1024-message and64-candidate caps without retaining lines', () => {
  for (const kind of ['messages', 'candidates']) {
    let log;const child = new ChildProcess();child.pid = 123;child.spawnfile = '/materials/browser/chrome-linux64/chrome';
    const implementation = { options: { browserProcess: { process: child }, browserLogsCollector: {
      onMessage(listener) { log = listener; }, recentLogs() { return []; } } } };
    const browser = { _connection: { toImpl() { return implementation; } } };
    const stop = observeBrowserProcess(browser);
    for (let index = 0; index < (kind === 'messages' ? 1024 : 64); index++) {
      log('[pid=123][err] ' + (kind === 'messages' ? 'ordinary private' : '[FATAL:counter.cc(1)] Check failed: private'));
    }
    log('[pid=123][err] Assertion `private forbidden late candidate` failed.');
    const value = stop();assert.equal(value.fatalLive, 'TRUNCATED');assert.ok(!value.stderrCategories.includes('C_ASSERT'));
    assert.equal(value.fatalLocations.length, kind === 'messages' ? 0 : 1);validateBrowserProcess(value);
    assert.ok(!/private|forbidden/.test(JSON.stringify(value)));
  }
});
