import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv from 'ajv';
import { assertTextBounds, canonicalBounded, parseCanonical } from './canonical.mjs';
import { readEnvelopeFile } from './input.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const SCHEMA_PATH = fileURLToPath(new URL(
  '../../schemas/zryna-release-tag-receipt-v1.schema.json', import.meta.url,
));
const ajv = new Ajv({ allErrors: true, strict: true });
const validateSchema = ajv.compile(JSON.parse(readFileSync(SCHEMA_PATH, 'utf8')));

function reject(code, message) {
  throw new Error(`${code}: ${message}`);
}

export function validateReleaseTagReceipt(document) {
  canonicalBounded(document);
  if (!validateSchema(document)) {
    reject('R406-TAG-SCHEMA', ajv.errorsText(validateSchema.errors, { separator: '; ' }));
  }
  if (document.source.ref !== `refs/tags/v${document.version}`) {
    reject('R406-TAG-SOURCE', 'tag reference and release version differ');
  }
  if (document.source.tagObject === document.source.commit) {
    reject('R406-TAG-SOURCE', 'annotated tag object and peeled commit must differ');
  }
  return document;
}

export function validateReleaseTagReceiptText(text) {
  assertTextBounds(text);
  return validateReleaseTagReceipt(parseCanonical(text));
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    if (process.argv.length !== 3) {
      reject('R406-TAG-USAGE', 'expected one release tag receipt path');
    }
    validateReleaseTagReceiptText(readEnvelopeFile(resolve(process.argv[2])));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
