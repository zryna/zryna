import { createHash } from 'node:crypto';

export const MAX_ENVELOPE_BYTES = 256 * 1024;
export const MAX_ENVELOPE_DEPTH = 32;
export const MAX_ENVELOPE_CONTAINERS = 1024;
export const MAX_ENVELOPE_VALUES = 4096;

function resource(message) {
  throw new Error(`R406-RESOURCE: ${message}`);
}

export function assertTextBounds(text) {
  if (typeof text !== 'string') resource('envelope text must be a string');
  if (Buffer.byteLength(text, 'utf8') > MAX_ENVELOPE_BYTES) {
    resource(`envelope exceeds ${MAX_ENVELOPE_BYTES} bytes`);
  }
  let depth = 0;
  let containers = 0;
  let quoted = false;
  let escaped = false;
  for (const character of text) {
    if (quoted) {
      if (escaped) escaped = false;
      else if (character === '\\') escaped = true;
      else if (character === '"') quoted = false;
      continue;
    }
    if (character === '"') quoted = true;
    else if (character === '{' || character === '[') {
      depth += 1;
      containers += 1;
      if (depth > MAX_ENVELOPE_DEPTH) resource(`envelope exceeds depth ${MAX_ENVELOPE_DEPTH}`);
      if (containers > MAX_ENVELOPE_CONTAINERS) {
        resource(`envelope exceeds ${MAX_ENVELOPE_CONTAINERS} containers`);
      }
    } else if (character === '}' || character === ']') {
      depth -= 1;
    }
  }
}

export function assertDocumentBounds(document) {
  const pending = [{ value: document, depth: 0 }];
  const seen = new WeakSet();
  let containers = 0;
  let values = 1;
  let stringBytes = 0;
  while (pending.length > 0) {
    const { value, depth } = pending.pop();
    if (typeof value === 'string') {
      stringBytes += Buffer.byteLength(value, 'utf8');
      if (stringBytes > MAX_ENVELOPE_BYTES) {
        resource(`envelope strings exceed ${MAX_ENVELOPE_BYTES} bytes`);
      }
    } else if (value !== null && typeof value === 'object') {
      if (seen.has(value)) resource('envelope contains a cycle');
      seen.add(value);
      containers += 1;
      if (containers > MAX_ENVELOPE_CONTAINERS) {
        resource(`envelope exceeds ${MAX_ENVELOPE_CONTAINERS} containers`);
      }
      if (depth + 1 > MAX_ENVELOPE_DEPTH) resource(`envelope exceeds depth ${MAX_ENVELOPE_DEPTH}`);
      const enqueue = (entry, key) => {
        values += 1;
        if (values > MAX_ENVELOPE_VALUES) {
          resource(`envelope exceeds ${MAX_ENVELOPE_VALUES} values`);
        }
        if (key !== undefined) stringBytes += Buffer.byteLength(key, 'utf8');
        if (stringBytes > MAX_ENVELOPE_BYTES) {
          resource(`envelope strings exceed ${MAX_ENVELOPE_BYTES} bytes`);
        }
        pending.push({ value: entry, depth: depth + 1 });
      };
      if (Array.isArray(value)) {
        for (let index = 0; index < value.length; index += 1) enqueue(value[index]);
      } else {
        for (const key in value) {
          if (Object.hasOwn(value, key)) enqueue(value[key], key);
        }
      }
    } else if (!['boolean', 'number'].includes(typeof value) && value !== null) {
      resource('envelope contains a non-JSON value');
    }
  }
}

export function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`;
  if (value !== null && typeof value === 'object') {
    const keys = Object.keys(value).sort();
    return `{${keys.map((key) => `${JSON.stringify(key)}:${canonical(value[key])}`).join(',')}}`;
  }
  return JSON.stringify(value);
}

export function canonicalBounded(value) {
  assertDocumentBounds(value);
  const wire = canonical(value);
  if (Buffer.byteLength(`${wire}\n`, 'utf8') > MAX_ENVELOPE_BYTES) {
    resource(`canonical envelope exceeds ${MAX_ENVELOPE_BYTES} bytes`);
  }
  return wire;
}

export function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

export function parseCanonical(text) {
  assertTextBounds(text);
  let value;
  try {
    value = JSON.parse(text);
  } catch {
    throw new Error('R406-CANONICAL: document is not valid JSON');
  }
  const wire = `${canonicalBounded(value)}\n`;
  if (wire !== text) throw new Error('R406-CANONICAL: document is not canonical JSON');
  return value;
}
