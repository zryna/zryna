import { bounds, decode, fail } from './wire.mjs';

export function createFrameDecoder(accept) {
  const header = Buffer.alloc(4);
  let headerBytes = 0; let payload = null; let used = 0;
  let total = 0; let count = 0; let closed = false;
  return Object.freeze({
    push(chunk) {
      if (closed || !(chunk instanceof Uint8Array) || (total += chunk.length) > bounds.aggregate) fail('FRAMING_001');
      let offset = 0;
      while (offset < chunk.length) {
        if (!payload) {
          const length = Math.min(4 - headerBytes, chunk.length - offset);
          header.set(chunk.subarray(offset, offset + length), headerBytes);
          offset += length; headerBytes += length;
          if (headerBytes !== 4) continue;
          const size = header.readUInt32LE();
          if (!size || size > bounds.frame || ++count > bounds.frames) fail('FRAMING_002');
          payload = Buffer.alloc(size); used = 0;
        }
        const length = Math.min(payload.length - used, chunk.length - offset);
        payload.set(chunk.subarray(offset, offset + length), used);
        offset += length; used += length;
        if (used === payload.length) {
          const value = decode(payload);
          payload = null; headerBytes = 0; used = 0;
          accept(value);
        }
      }
    },
    end() { closed = true; if (headerBytes || payload) fail('FRAMING_003'); },
  });
}

export function createFrameWriter(stream) {
  let pending = 0; let total = 0; let count = 0; let stopped = false; let failure = null;
  let chain = Promise.resolve();
  return Object.freeze({
    write(value) {
      if (stopped) fail('FRAMING_004');
      const bytes = Buffer.from(JSON.stringify(value));
      if (!bytes.length || bytes.length > bounds.frame || ++count > bounds.frames ||
          (total += bytes.length + 4) > bounds.aggregate || (pending += bytes.length + 4) > bounds.frame * 2) fail('FRAMING_005');
      const frame = Buffer.alloc(bytes.length + 4);
      frame.writeUInt32LE(bytes.length); frame.set(bytes, 4);
      const sent = chain.then(() => new Promise((resolve, reject) => {
        if (failure) { reject(failure); return; }
        stream.write(frame, error => error ? reject(error) : resolve());
      })).finally(() => { pending -= frame.length; });
      chain = sent.catch(error => { stopped = true; failure ??= error; });
      return sent;
    },
    async stop() { stopped = true; await chain; if (failure) throw failure; },
  });
}
