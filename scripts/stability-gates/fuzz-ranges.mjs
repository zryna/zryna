import assert from 'node:assert/strict';
import { exact } from './input.mjs';

export function range(value, text, file = true) {
  exact(value, file ? ['file', 'start', 'end'] : ['start', 'end'], 'source range');
  if (file) assert.equal(value.file, 0);
  assert(Number.isSafeInteger(value.start) && Number.isSafeInteger(value.end));
  assert(value.start >= 0 && value.start <= value.end && value.end <= Buffer.byteLength(text));
  const boundaries = new Set([0]);
  let offset = 0;
  for (const scalar of text) { offset += Buffer.byteLength(scalar); boundaries.add(offset); }
  assert(boundaries.has(value.start) && boundaries.has(value.end), 'UTF-8 source boundary');
}


export function walkRanges(raw, source, version) {
  const pending = [{ value: raw, depth: 0 }];
  let nodes = 0;
  while (pending.length) {
    const { value, depth } = pending.pop();
    assert(++nodes <= 8192 && depth <= 24, 'bounded DTO walk');
    if (value === null || typeof value !== 'object') {
      if (typeof value === 'number') assert(Number.isSafeInteger(value) && value >= 0 && value <= 0xffffffff);
      continue;
    }
    if (Array.isArray(value)) {
      assert(value.length <= 16384, 'bounded DTO array');
      for (const child of value) pending.push({ value: child, depth: depth + 1 });
      continue;
    }
    if (['file', 'start', 'end'].some(key => Object.hasOwn(value, key))) {
      range(value, source.text);
      continue;
    }
    for (const [key, child] of Object.entries(value)) {
      if (key === 'span' || key.endsWith('_span')) {
        const nullable = version > 2 && ['export_span', 'as_span', 'none_span'].includes(key);
        if (child !== null || !nullable) range(child, source.text);
      }
      pending.push({ value: child, depth: depth + 1 });
    }
  }
}
