import assert from 'node:assert/strict';

// Rewrite only the export section of a validated, private in-memory test copy.
export function exposeTestExports(bytes, exports) {
  assert.ok(WebAssembly.validate(bytes), 'inspection requires a valid original module');
  let offset = 8;
  const sections = [bytes.slice(0, 8)];
  let changed = false;
  while (offset < bytes.length) {
    const kind = bytes[offset++];
    const size = readLeb();
    const end = offset + size;
    let payload = bytes.slice(offset, end);
    if (kind === 7) {
      const count = readLeb();
      const added = exports.flatMap(value => {
        const name = new TextEncoder().encode(value.name);
        return [...leb(name.length), ...name, value.kind, ...leb(value.index)];
      });
      payload = Uint8Array.from([...leb(count + exports.length), ...bytes.slice(offset, end), ...added]);
      changed = true;
    }
    sections.push(Uint8Array.from([kind, ...leb(payload.length)]), payload);
    offset = end;
  }
  assert.ok(changed, 'the validated module must have an export section');
  return Buffer.concat(sections);

  function readLeb() {
    let value = 0;
    let shift = 0;
    for (let count = 0; count < 5; count++) {
      const byte = bytes[offset++];
      value |= (byte & 127) << shift;
      if ((byte & 128) === 0) return value >>> 0;
      shift += 7;
    }
    throw new Error('invalid section length in validated input');
  }
}

function leb(value) {
  const bytes = [];
  do {
    const next = value & 127;
    value >>>= 7;
    bytes.push(next | (value ? 128 : 0));
  } while (value);
  return bytes;
}
