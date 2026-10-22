// Engine Tty=false headers are independent of application u32 framing.
import { bounds, fail } from './wire.mjs';

export function createEngineDemux(stdout, stderr) {
  const header = Buffer.alloc(8);
  let used = 0; let remaining = 0; let channel = 0; let total = 0; let errorBytes = 0; let closed = false;
  return Object.freeze({
    push(chunk) {
      if (closed || !(chunk instanceof Uint8Array) || (total += chunk.length) > bounds.aggregate) fail('ENGINE_001');
      let offset = 0;
      while (offset < chunk.length) {
        if (!remaining) {
          const length = Math.min(8 - used, chunk.length - offset);
          header.set(chunk.subarray(offset, offset + length), used);
          offset += length; used += length;
          if (used !== 8) continue;
          channel = header[0]; remaining = header.readUInt32BE(4); used = 0;
          if (![1, 2].includes(channel) || header[1] || header[2] || header[3] ||
              !remaining || remaining > bounds.frame) fail('ENGINE_002');
        }
        const length = Math.min(remaining, chunk.length - offset);
        const bytes = chunk.subarray(offset, offset + length);
        if (channel === 2) {
          if ((errorBytes += length) > bounds.stderr) fail('ENGINE_003');
          stderr(bytes);
        } else stdout(bytes);
        offset += length; remaining -= length;
      }
    },
    end() { closed = true; if (used || remaining) fail('ENGINE_004'); },
  });
}
