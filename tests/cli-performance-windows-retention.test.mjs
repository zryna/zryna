// Pure regression IO, with real byte-budget enforcement. No process/helper/CLI execution.
import assert from 'node:assert/strict';
import test from 'node:test';
import { MAX_LOG } from '../scripts/stability-gates/input.mjs';
import { collectWindowsRound } from '../scripts/stability-gates/cli-performance-windows-round.mjs';
import { document, fixture } from './cli-performance-windows-fixture.mjs';
const id = 'm1-wrap_build_javascript_0';
function bounded(change) {
  const f = fixture(change), original = f.io.retain;
  f.io.retain = (name, bytes) => {
    assert(Buffer.isBuffer(bytes) && bytes.length <= MAX_LOG, 'actual writer byte budget');
    return original(name, bytes);
  };
  return f;
}
async function invalid(f, pattern) {
  await assert.rejects(collectWindowsRound(document, f.capture, f.io), pattern);
  assert.equal(f.calls(), 1, 'no retry after the one attempted command');
  const session = JSON.parse(f.saved.get('session.json'));
  assert.equal(session.status, 'invalid-or-incomplete'); assert.equal(session.cells.length, 0);
  assert(!f.events.some(event => event.startsWith('after:')));
  assert(!f.events.includes('finishRound'));
  assert.equal(f.events.filter(event => event === 'material').length, 3, 'initial, pre-command, unconditional post-command');
  return JSON.parse(f.saved.get(`${id}.process.json`));
}
test('Windows actual MAX_LOG+1 stdout or stderr retains status actual length labeled bounded prefix and unconditional postflight', async () => {
  for (const stream of ['stdout', 'stderr']) {
    const bytes = Buffer.alloc(MAX_LOG + 1, stream === 'stdout' ? 0x41 : 0x42);
    const f = bounded(({ result }) => { result[stream] = bytes; result.exitCode = 23; result.error = 'ENOBUFS: actual captured extra byte'; });
    const process = await invalid(f);
    assert.equal(process.exitCode, 23); assert.equal(process.signal, null);
    assert.equal(process.error, 'ENOBUFS: actual captured extra byte');
    assert.equal(process[`${stream}Bytes`], MAX_LOG + 1);
    const evidence = process.helperStreams[stream];
    assert.deepEqual(evidence, { capturedBytes: MAX_LOG + 1, retainedBytes: MAX_LOG, truncated: true,
      evidence: 'bounded-prefix-ineligible', retainedFile: `${id}.helper.${stream}.prefix` });
    assert(f.saved.get(evidence.retainedFile).equals(bytes.subarray(0, MAX_LOG)));
    assert(!f.saved.has(`${id}.helper.${stream}`), 'prefix never relabeled complete stream');
    assert(f.events.indexOf(`retain:${id}.process.json`) < f.events.indexOf(`retain:${evidence.retainedFile}`));
    assert(f.saved.has(`${id}.metadata.raw`) && f.saved.has(`${id}.stdout`) && f.saved.has(`${id}.stderr`));
    assert(!f.events.includes('parseHelper'));
  }
});
test('Windows truncated streams cannot become measurements even with successful outer process status', async () => {
  const f = bounded(({ result }) => { result.stdout = Buffer.alloc(MAX_LOG + 1, 0x43); });
  const process = await invalid(f, /untruncated helper streams/u);
  assert.equal(process.exitCode, 0); assert.equal(process.error, null);
  assert.equal(process.helperStreams.stdout.truncated, true);
  assert(!f.events.includes('parseHelper'));
});
test('Windows combined helper raw prefixes stay within unchanged MAX_LOG budget', async () => {
  const size = MAX_LOG / 2 + 1;
  const f = bounded(({ result }) => {
    result.stdout = Buffer.alloc(size, 0x44); result.stderr = Buffer.alloc(size, 0x45);
    result.exitCode = 19;
  });
  const process = await invalid(f);
  assert.equal(process.stdoutBytes + process.stderrBytes, MAX_LOG + 2);
  const kept = Object.values(process.helperStreams).reduce((sum, row) => sum + row.retainedBytes, 0);
  assert.equal(kept, MAX_LOG); assert.equal(process.helperStreams.stderr.truncated, true);
  assert.equal(process.helperStreams.stderr.evidence, 'bounded-prefix-ineligible');
});
test('Windows ordinary nonzero control retains nonempty full stderr and real status before invalidation', async () => {
  const bytes = Buffer.from('synthetic helper diagnostic with retained bytes\n');
  const f = bounded(({ result }) => { result.exitCode = 17; result.stderr = bytes; });
  const process = await invalid(f, /helper process successful/u);
  assert.equal(process.exitCode, 17); assert.equal(process.error, null);
  assert.equal(process.stderrBytes, bytes.length);
  assert.equal(process.helperStreams.stderr.truncated, false);
  assert.equal(process.helperStreams.stderr.evidence, 'complete-captured-stream');
  assert(f.saved.get(`${id}.helper.stderr`).equals(bytes));
});
test('Windows writer failure after process metadata still runs postflight and retains no observation', async () => {
  const f = bounded(({ result }) => { result.stderr = Buffer.from('retention failure control'); result.exitCode = 17; });
  const retain = f.io.retain;
  f.io.retain = (name, bytes) => {
    if (name === `${id}.helper.stderr`) throw new Error('synthetic writer failure');
    return retain(name, bytes);
  };
  const process = await invalid(f, /synthetic writer failure/u);
  assert.equal(process.exitCode, 17); assert.equal(process.stderrBytes, 25);
  const finalMaterial = f.events.lastIndexOf('material');
  assert(finalMaterial > f.events.indexOf(`retain:${id}.process.json`));
});
test('Windows parser validation failure runs postflight after the failing validation without retry', async () => {
  const f = bounded();
  f.io.parseHelper = () => { f.events.push('failingParser'); throw new Error('synthetic malformed metadata'); };
  await invalid(f, /synthetic malformed metadata/u);
  assert(f.events.lastIndexOf('material') > f.events.indexOf('failingParser'));
});
