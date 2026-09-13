import {
  closeSync, lstatSync, readSync, unlinkSync,
} from 'node:fs';
import { isAbsolute, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  createReleaseFile, exactReleaseNames, MAX_RELEASE_DOCUMENT, openReleaseFile,
  revalidateReleaseFile,
} from './release-files.mjs';
import { validateReleaseText } from './validate.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const ENVELOPE = 'zryna-release-envelope-v1.json';
const ACTION_SIGNATURE = `${ENVELOPE}.sigstore.json`;
const CANONICAL_SIGNATURE = 'zryna-release-envelope-v1.sigstore.json';

function reject(message) {
  throw new Error(`R406-ENVELOPE-SIGNATURE: ${message}`);
}

function sameIdentity(left, right) {
  return left.dev === right.dev && left.ino === right.ino && left.size === right.size
    && left.mtimeMs === right.mtimeMs && left.mode === right.mode;
}

function readRetained(file) {
  const bytes = Buffer.allocUnsafe(file.metadata.size);
  let offset = 0;
  while (offset < bytes.length) {
    const count = readSync(file.descriptor, bytes, offset, bytes.length - offset, offset);
    if (count === 0) reject('action signature ended before its retained size');
    offset += count;
  }
  revalidateReleaseFile(file);
  return bytes;
}

export function canonicalizeEnvelopeSignature(directory) {
  if (!isAbsolute(directory) || resolve(directory) !== directory) {
    reject('publication directory must be absolute and normalized');
  }
  const envelopeFile = openReleaseFile(directory, ENVELOPE, MAX_RELEASE_DOCUMENT);
  let envelopeBytes;
  try {
    envelopeBytes = readRetained(envelopeFile);
  } finally {
    closeSync(envelopeFile.descriptor);
  }
  const envelope = validateReleaseText(envelopeBytes.toString('utf8'));
  if (envelope.signing.envelopeSignaturePath !== CANONICAL_SIGNATURE) {
    reject('canonical envelope signature path differs');
  }
  const actionInventory = envelope.assetAllowlist.map((name) => (
    name === CANONICAL_SIGNATURE ? ACTION_SIGNATURE : name
  ));
  exactReleaseNames(directory, actionInventory);

  const source = openReleaseFile(directory, ACTION_SIGNATURE, MAX_RELEASE_DOCUMENT);
  try {
    const signature = readRetained(source);
    createReleaseFile(directory, CANONICAL_SIGNATURE, signature);
    revalidateReleaseFile(source);
    const namedSource = lstatSync(source.path);
    if (namedSource.isSymbolicLink() || !sameIdentity(source.metadata, namedSource)) {
      reject('action signature identity changed before canonicalization');
    }
    unlinkSync(source.path);
  } finally {
    closeSync(source.descriptor);
  }
  exactReleaseNames(directory, envelope.assetAllowlist);
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    if (process.argv.length !== 4 || process.argv[2] !== '--directory') {
      reject('expected --directory once');
    }
    canonicalizeEnvelopeSignature(resolve(process.argv[3]));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
