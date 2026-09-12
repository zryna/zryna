import { createGzip, gunzipSync } from 'node:zlib';
import { Readable } from 'node:stream';
import { LIMITS, requireValue } from './canonical.mjs';
import { archiveEntries, decodedFiles } from './archive-entries.mjs';

const BLOCK = 512;
const TAR_LIMIT = LIMITS.expanded + LIMITS.files * LIMITS.depth * 1024 + 1024;

function octal(header, offset, width, value) {
  const digits = value.toString(8);
  requireValue(digits.length < width, 'tar numeric field overflow');
  header.write(`${digits.padStart(width - 1, '0')}\0`, offset, width, 'ascii');
}

function header(entry, epoch) {
  const result = Buffer.alloc(BLOCK);
  let name = entry.path;
  let prefix = '';
  if (name.length > 100) {
    const split = name.lastIndexOf('/', Math.min(155, name.length - 2));
    prefix = name.slice(0, split);
    name = name.slice(split + 1);
  }
  requireValue(name.length <= 100 && prefix.length <= 155, 'ustar path budget');
  result.write(name, 0, 100, 'ascii');
  octal(result, 100, 8, entry.mode);
  octal(result, 108, 8, 0);
  octal(result, 116, 8, 0);
  octal(result, 124, 12, entry.data.length);
  octal(result, 136, 12, epoch);
  result.fill(32, 148, 156);
  result[156] = entry.directory ? 53 : 48;
  result.write('ustar\0', 257, 'ascii');
  result.write('00', 263, 'ascii');
  result.write(prefix, 345, 155, 'ascii');
  const sum = result.reduce((a, b) => a + b, 0);
  result.write(`${sum.toString(8).padStart(6, '0')}\0 `, 148, 8, 'ascii');
  return result;
}

function* tarChunks(root, files, epoch) {
  requireValue(Number.isSafeInteger(epoch) && epoch >= 0 && epoch <= 0xffffffff, 'source epoch');
  for (const entry of archiveEntries(root, files)) {
    yield header(entry, epoch);
    yield entry.data;
    const padding = (BLOCK - entry.data.length % BLOCK) % BLOCK;
    if (padding) yield Buffer.alloc(padding);
  }
  yield Buffer.alloc(2 * BLOCK);
}

async function* compressedChunks(root, files, epoch) {
  const source = Readable.from(tarChunks(root, files, epoch));
  const stream = createGzip({ level: 9 });
  source.on('error', error => stream.destroy(error));
  stream.on('error', () => source.destroy());
  source.pipe(stream);
  let total = 0;
  for await (const chunk of stream) {
    // No build-host operating system in the gzip header.
    if (total <= 9 && total + chunk.length > 9) chunk[9 - total] = 255;
    total += chunk.length;
    requireValue(total <= LIMITS.archive, 'archive wire budget');
    yield chunk;
  }
}

export async function encodeTar(root, files, epoch) {
  const chunks = [];
  for await (const chunk of compressedChunks(root, files, epoch)) chunks.push(chunk);
  return Buffer.concat(chunks);
}

function stringField(header, offset, width) {
  const field = header.subarray(offset, offset + width);
  const end = field.indexOf(0);
  const value = field.subarray(0, end < 0 ? width : end);
  requireValue(value.every(byte => byte >= 32 && byte < 127), 'tar path encoding');
  return value.toString('ascii');
}

function numberField(header, offset, width) {
  const value = stringField(header, offset, width);
  requireValue(/^[0-7]+$/.test(value), 'tar numeric encoding');
  return Number.parseInt(value, 8);
}

export async function decodeTar(input, root, epoch) {
  requireValue(Buffer.isBuffer(input) && input.length <= LIMITS.archive, 'archive wire budget');
  const payload = gunzipSync(input, { maxOutputLength: TAR_LIMIT });
  const entries = [];
  let offset = 0;
  let total = 0;
  while (offset + BLOCK <= payload.length) {
    const block = payload.subarray(offset, offset + BLOCK);
    if (block.every(byte => byte === 0)) break;
    const name = stringField(block, 0, 100);
    const prefix = stringField(block, 345, 155);
    const size = numberField(block, 124, 12);
    requireValue(size <= LIMITS.binary && offset + BLOCK + size <= payload.length,
      'tar member byte budget');
    total += size;
    requireValue(total <= LIMITS.expanded, 'tar expansion budget');
    requireValue(block[156] === 48 || block[156] === 53, 'tar entry type');
    entries.push({
      path: prefix ? `${prefix}/${name}` : name,
      mode: numberField(block, 100, 8),
      directory: block[156] === 53,
      data: payload.subarray(offset + BLOCK, offset + BLOCK + size),
    });
    requireValue(entries.length <= LIMITS.files * LIMITS.depth + 1, 'tar entry count');
    offset += BLOCK + Math.ceil(size / BLOCK) * BLOCK;
  }
  const files = decodedFiles(root, entries);
  offset = 0;
  for await (const chunk of compressedChunks(root, files, epoch)) {
    requireValue(chunk.equals(input.subarray(offset, offset + chunk.length)), 'noncanonical tar/gzip archive');
    offset += chunk.length;
  }
  requireValue(offset === input.length, 'tar/gzip trailing bytes');
  return files;
}
