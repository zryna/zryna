import assert from 'node:assert/strict';
import { PassThrough, Writable } from 'node:stream';
import test from 'node:test';
import { bounds, binary, decode } from './wire.mjs';
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
