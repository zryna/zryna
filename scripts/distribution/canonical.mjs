import { createHash } from 'node:crypto';

export const LIMITS = Object.freeze({
  files: 512,
  binary: 256 * 1024 * 1024,
  provider: 16 * 1024 * 1024,
  text: 2 * 1024 * 1024,
  record: 256 * 1024,
  expanded: 512 * 1024 * 1024,
  archive: 513 * 1024 * 1024,
  path: 200,
  depth: 12,
});

export function requireValue(condition, detail) {
  if (!condition) throw new Error(`D422-ADMISSION: ${detail}`);
}

export function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`;
  if (value !== null && typeof value === 'object') {
    return `{${Object.keys(value).sort().map(key =>
      `${JSON.stringify(key)}:${canonical(value[key])}`).join(',')}}`;
  }
  requireValue(value === null || typeof value === 'string' || typeof value === 'boolean'
    || (typeof value === 'number' && Number.isSafeInteger(value)), 'unsupported JSON value');
  return JSON.stringify(value);
}

export function bytes(value) {
  return Buffer.from(`${canonical(value)}\n`, 'utf8');
}

export function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

export function parseCanonical(input) {
  requireValue(Buffer.isBuffer(input) && input.length <= LIMITS.record, 'record byte budget');
  const text = new TextDecoder('utf-8', { fatal: true }).decode(input);
  let depth = 0;
  let quoted = false;
  let escaped = false;
  for (const character of text) {
    if (quoted) {
      if (escaped) escaped = false;
      else if (character === '\\') escaped = true;
      else if (character === '"') quoted = false;
    } else if (character === '"') quoted = true;
    else if (character === '{' || character === '[') {
      requireValue(++depth <= LIMITS.depth, 'record depth budget');
    } else if (character === '}' || character === ']') depth--;
  }
  const value = JSON.parse(text);
  requireValue(bytes(value).equals(input), 'noncanonical or duplicate JSON');
  return value;
}

export function exactKeys(value, keys) {
  requireValue(value !== null && typeof value === 'object' && !Array.isArray(value)
    && Object.keys(value).sort().join('\0') === [...keys].sort().join('\0'), 'record keys');
}

export function portablePath(value) {
  requireValue(typeof value === 'string' && value.length <= LIMITS.path, 'path byte budget');
  const segments = value.split('/');
  requireValue(segments.length <= LIMITS.depth && segments.every(segment =>
    /^[A-Za-z0-9@][A-Za-z0-9@+._-]*$/.test(segment)
    && !/[. ]$/.test(segment)
    && !/^(con|prn|aux|nul|com[0-9]|lpt[0-9])(?:\.|$)/i.test(segment)), 'unsafe portable path');
  return value;
}

export function orderedPaths(paths) {
  requireValue(Array.isArray(paths) && paths.length <= LIMITS.files, 'file count budget');
  const seen = new Set();
  const directories = new Set();
  let previous = '';
  for (const path of paths) {
    portablePath(path);
    requireValue(path > previous, 'unordered or duplicate paths');
    previous = path;
    const folded = path.toLowerCase();
    requireValue(!seen.has(folded) && !directories.has(folded), 'path collision');
    const segments = folded.split('/');
    for (let index = 1; index < segments.length; index++) {
      const parent = segments.slice(0, index).join('/');
      requireValue(!seen.has(parent), 'file/directory collision');
      directories.add(parent);
    }
    seen.add(folded);
  }
  // Differently cased directory names also alias on qualified Windows hosts.
  const spellings = new Map();
  for (const path of paths) {
    const segments = path.split('/');
    for (let index = 1; index <= segments.length; index++) {
      const prefix = segments.slice(0, index).join('/');
      const folded = prefix.toLowerCase();
      requireValue(!spellings.has(folded) || spellings.get(folded) === prefix, 'case collision');
      spellings.set(folded, prefix);
    }
  }
}
