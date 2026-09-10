import {
  closeSync, constants, fstatSync, lstatSync, openSync, readSync,
} from 'node:fs';
import { MAX_ENVELOPE_BYTES } from './canonical.mjs';

function resource(message) {
  throw new Error(`R406-RESOURCE: ${message}`);
}

export function readEnvelopeFile(path) {
  let descriptor;
  try {
    const before = lstatSync(path, { bigint: true });
    if (!before.isFile() || before.isSymbolicLink()) {
      resource('envelope must be a direct regular file');
    }
    const platformFlags = process.platform === 'win32' ? 0 : (constants.O_NONBLOCK ?? 0);
    descriptor = openSync(path,
      constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0) | platformFlags);
    const opened = fstatSync(descriptor, { bigint: true });
    if (!opened.isFile() || opened.dev !== before.dev || opened.ino !== before.ino) {
      resource('envelope identity changed before open');
    }
    if (opened.size > BigInt(MAX_ENVELOPE_BYTES)) {
      resource(`envelope exceeds ${MAX_ENVELOPE_BYTES} bytes`);
    }
    const expected = Number(opened.size);
    const bytes = Buffer.alloc(expected + 1);
    let offset = 0;
    while (offset < bytes.length) {
      const count = readSync(descriptor, bytes, offset, bytes.length - offset, null);
      if (count === 0) break;
      offset += count;
    }
    const after = fstatSync(descriptor, { bigint: true });
    if (offset !== expected || after.size !== opened.size
      || after.mtimeNs !== opened.mtimeNs || after.ctimeNs !== opened.ctimeNs) {
      resource('envelope changed during retained read');
    }
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes.subarray(0, offset));
  } catch (error) {
    if (error instanceof Error && error.message.startsWith('R406-')) throw error;
    resource(error instanceof Error ? error.message : String(error));
  } finally {
    if (descriptor !== undefined) closeSync(descriptor);
  }
}
