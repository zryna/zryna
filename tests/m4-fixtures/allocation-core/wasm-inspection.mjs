import assert from 'node:assert/strict';

// Rewrite only the export section of a validated, private in-memory test copy.
export function exposeTestExports(bytes, exports) {
  return rewriteSections(bytes, (kind, payload) => {
    if (kind !== 7) return payload;
    const count = readLeb(payload, 0);
    const added = exports.flatMap(value => {
      const name = new TextEncoder().encode(value.name);
      return [...leb(name.length), ...name, value.kind, ...leb(value.index)];
    });
    return Uint8Array.from([
      ...leb(count.value + exports.length),
      ...payload.slice(count.next),
      ...added,
    ]);
  }, 7);
}

export function findFunctionExport(bytes, selected) {
  let found;
  rewriteSections(bytes, (kind, payload) => {
    if (kind !== 7) return payload;
    let cursor = readLeb(payload, 0);
    for (let index = 0; index < cursor.value; index++) {
      const length = readLeb(payload, cursor.next);
      const end = length.next + length.value;
      const name = new TextDecoder().decode(payload.slice(length.next, end));
      const exportKind = payload[end];
      const target = readLeb(payload, end + 1);
      if (name === selected) {
        assert.equal(exportKind, 0, `${selected} must be a function export`);
        found = target.value;
      }
      cursor = { value: cursor.value, next: target.next };
    }
    assert.equal(cursor.next, payload.length, 'complete export section');
    return payload;
  }, 7);
  assert.ok(Number.isInteger(found), `missing ${selected} export`);
  return found;
}

// Prepend `record(local.get 0)` to one drop helper in a private in-memory copy.
export function instrumentFunctionArgument(bytes, functionIndex, recorderIndex) {
  return rewriteSections(bytes, (kind, payload) => {
    if (kind !== 10) return payload;
    const count = readLeb(payload, 0);
    assert.ok(functionIndex < count.value, 'drop helper index must be defined');
    assert.equal(recorderIndex + 1, count.value, 'observation recorder must remain last');
    let cursor = count.next;
    const bodies = [];
    for (let index = 0; index < count.value; index++) {
      const size = readLeb(payload, cursor);
      const end = size.next + size.value;
      let body = payload.slice(size.next, end);
      if (index === functionIndex) {
        const locals = readLeb(body, 0);
        let instruction = locals.next;
        for (let declaration = 0; declaration < locals.value; declaration++) {
          const localCount = readLeb(body, instruction);
          instruction = localCount.next + 1;
        }
        body = Uint8Array.from([
          ...body.slice(0, instruction),
          0x20, 0x00,
          0x10, ...leb(recorderIndex),
          ...body.slice(instruction),
        ]);
      }
      bodies.push(...leb(body.length), ...body);
      cursor = end;
    }
    assert.equal(cursor, payload.length, 'complete code section');
    return Uint8Array.from([...leb(count.value), ...bodies]);
  }, 10);
}

export function cleanupHandles(trace, cleanup, module = 0) {
  const handles = [];
  let cursor = 0;
  for (const [place, kind] of cleanup) {
    const root = 0x20000000 + module;
    while (cursor + 4 < trace.length
      && !(trace[cursor] === root && trace[cursor + 1] === 0 && trace[cursor + 2] === place)) {
      cursor++;
    }
    assert.ok(cursor + 4 < trace.length, `missing cleanup handle for place ${place}`);
    handles.push(trace[cursor + 3]);
    assert.equal(trace[cursor + 4], 0x10000000 + { string: 1, sequence: 2 }[kind]);
    cursor += 5;
  }
  assert.equal(new Set(handles).size, handles.length, 'each dropped owner needs a distinct handle');
  return handles;
}

function rewriteSections(bytes, transform, requiredKind) {
  assert.ok(WebAssembly.validate(bytes), 'inspection requires a valid original module');
  let offset = 8;
  const sections = [bytes.slice(0, 8)];
  let changed = false;
  while (offset < bytes.length) {
    const kind = bytes[offset++];
    const size = readLeb(bytes, offset);
    const end = size.next + size.value;
    const original = bytes.slice(size.next, end);
    const payload = transform(kind, original);
    if (kind === requiredKind) changed = true;
    sections.push(Uint8Array.from([kind, ...leb(payload.length)]), payload);
    offset = end;
  }
  assert.equal(offset, bytes.length, 'complete validated module');
  assert.ok(changed, `the validated module must have section ${requiredKind}`);
  const result = Buffer.concat(sections);
  assert.ok(WebAssembly.validate(result), 'inspection rewrite must remain valid');
  return result;
}

function readLeb(bytes, offset) {
  let value = 0;
  let shift = 0;
  for (let count = 0; count < 5; count++) {
    const byte = bytes[offset++];
    value += (byte & 127) * 2 ** shift;
    if ((byte & 128) === 0) return { value, next: offset };
    shift += 7;
  }
  throw new Error('invalid unsigned LEB128');
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
