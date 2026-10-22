// Offline ownership-contract controls; synthetic output is never actual CLI/browser evidence.
import assert from 'node:assert/strict';
import test from 'node:test';
import { boundedAdapterCall } from './adapter.mjs';
import { produceParity } from './producer.mjs';
import { authority, witSha256, cliCapture, compilation, observation } from './fixture.test-support.mjs';
const turn = () => new Promise(resolve => setImmediate(resolve));
const never = () => new Promise(() => {});
const options = { kind: 'compile', id: 'addition', activeMs: 10000 };
test('pending adapter has one finite deadline, abort signal and distinct bounded cleanup on timeout', async t => {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  const calls = []; let runSignal;
  const result = boundedAdapterCall({ run(input, context) { runSignal = context.signal; calls.push('run'); return never(); },
    cleanup(context) { calls.push(context); return { confirmed: true }; } }, {}, options);
  const rejected = assert.rejects(result, value => {
    assert.equal(value.cleanupConfirmed, true); assert.match(value.primary.message, /DEADLINE/); return true;
  });
  await turn(); assert.equal(runSignal.aborted, false); t.mock.timers.tick(10000); await rejected;
  assert.equal(runSignal.aborted, true); assert.equal(calls.length, 2); assert.equal(calls[1].cancelled, true);
  assert.notEqual(calls[1].signal, runSignal); assert.equal(calls[1].signal.aborted, true);
});
test('pending cleanup terminates at2s and preserves primary and unknown cleanup rather than returning late output', async t => {
  t.mock.timers.enable({ apis: ['setTimeout'] }); let cleanupSignal;
  const result = boundedAdapterCall({ run() { throw new Error('primary fixture'); },
    cleanup(context) { cleanupSignal = context.signal; return never(); } }, {}, options);
  const rejected = assert.rejects(result, value => {
    assert.equal(value.cleanupConfirmed, false); assert.equal(value.primary.message, 'primary fixture');
    assert.match(value.cleanup.message, /CLEANUP-DEADLINE/); return true;
  });
  await turn(); assert.equal(cleanupSignal.aborted, false); t.mock.timers.tick(2000); await rejected;
  assert.equal(cleanupSignal.aborted, true);
});
test('caller abort never invokes pre-aborted work and midflight abort still performs owner cleanup', async () => {
  const controller = new AbortController(); const calls = [];
  const adapter = { run(_, context) { calls.push(context); return never(); },
    cleanup(context) { calls.push(context); return { confirmed: true }; } };
  controller.abort(); await assert.rejects(boundedAdapterCall(adapter, {}, { ...options, signal: controller.signal }), /ABORTED/);
  assert.equal(calls.length, 0);
  const live = new AbortController(); const result = boundedAdapterCall(adapter, {}, { ...options, signal: live.signal });
  const rejected = assert.rejects(result, value => value.cleanupConfirmed === true && /ABORTED/.test(value.primary.message));
  await turn(); live.abort(); await rejected;
  assert.equal(calls.length, 2); assert.equal(calls[0].signal.aborted, true); assert.equal(calls[1].cancelled, true);
});
test('cleanup is required after success, failure and malformed cleanup; no arbitrary budget override', async () => {
  let cleaned = 0;
  const adapter = { run: () => 42, cleanup() { cleaned++; return { confirmed: true }; } };
  assert.equal(await boundedAdapterCall(adapter, {}, options), 42); assert.equal(cleaned, 1);
  for (const proof of [undefined, {}, { confirmed: false }, { confirmed: true, extra: true }]) {
    await assert.rejects(boundedAdapterCall({ run: () => 42, cleanup: () => proof }, {}, options),
      value => value.cleanupConfirmed === false && value.primary === null);
  }
  await assert.rejects(boundedAdapterCall(adapter, {}, { ...options, activeMs: 10001 }), /CLI-PARITY/);
});
test('producer validates all adapters before effects and stops immediately after parse failure or unknown cleanup', async () => {
  let invoked = 0;
  const cli = { run() { invoked++; return {}; }, cleanup: () => ({ confirmed: true }) };
  const other = { run() { throw new Error('must never advance'); }, cleanup: () => ({ confirmed: true }) };
  await assert.rejects(produceParity({ runCli: cli, compilePlayground: other,
    evaluatePlayground: () => {}, authority, witSha256 }), /CLI-PARITY/);
  assert.equal(invoked, 0);
  await assert.rejects(produceParity({ runCli: cli, compilePlayground: other,
    evaluatePlayground: other, authority, witSha256 }), /CLI-PARITY/);
  assert.equal(invoked, 1);
  cli.cleanup = () => ({ confirmed: false });
  await assert.rejects(produceParity({ runCli: cli, compilePlayground: other,
    evaluatePlayground: other, authority, witSha256 }), value => value.cleanupConfirmed === false);
  assert.equal(invoked, 2);
});
test('producer compares every fixed synthetic operation and cleans each call before advancing', async () => {
  const events = []; const components = new Map();
  const adapter = (kind, run) => ({ run(input, context) {
    events.push([kind, 'run', context.id]); assert.equal(context.signal.aborted, false); return run(input);
  }, cleanup(context) { events.push([kind, 'cleanup', context.id]); return { confirmed: true }; } });
  const result = await produceParity({ authority, witSha256,
    runCli: adapter('cli', cliCapture),
    compilePlayground: adapter('compile', async input => {
      const value = await compilation(input.corpusId, input.sourceRequest.revision);
      const size = new DataView(value.frameBytes.buffer, value.frameBytes.byteOffset).getUint32(0, true);
      const metadata = JSON.parse(Buffer.from(value.frameBytes).subarray(4, 4 + size).toString());
      components.set(input.corpusId, metadata.identity?.componentSha256); return value;
    }), evaluatePlayground: adapter('evaluate', input => observation(input.operationId, components.get(input.corpusId))) });
  assert.equal(result.comparisonStatus, 'matched'); assert.equal(result.acceptance, false);
  assert.equal(events.length, 76);
  for (let index = 0; index < events.length; index += 2) {
    assert.equal(events[index][1], 'run'); assert.equal(events[index + 1][1], 'cleanup');
    assert.equal(events[index][0], events[index + 1][0]); assert.equal(events[index][2], events[index + 1][2]);
  }
});
