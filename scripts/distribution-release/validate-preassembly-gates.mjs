import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv from 'ajv';
import {
  assertTextBounds, canonical, canonicalBounded, parseCanonical, sha256,
} from './canonical.mjs';
import { readEnvelopeFile } from './input.mjs';
import { validateBuildInputText } from './validate-build-input.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const SCHEMA_PATH = fileURLToPath(new URL(
  '../../schemas/zryna-preassembly-gates-v1.schema.json', import.meta.url,
));
const ajv = new Ajv({ allErrors: true, strict: true });
const validateSchema = ajv.compile(JSON.parse(readFileSync(SCHEMA_PATH, 'utf8')));

function reject(code, message) {
  throw new Error(`${code}: ${message}`);
}

export function validatePreassemblyGates(document, input) {
  canonicalBounded(document);
  if (!validateSchema(document)) {
    reject('R406-GATES-SCHEMA', ajv.errorsText(validateSchema.errors, { separator: '; ' }));
  }
  const expected = input?.gateReceipt;
  if (!expected || document.repository !== input?.source?.repository
    || document.sourceCommit !== input?.source?.commit) {
    reject('R406-GATES-SOURCE', 'gate and build source identities differ');
  }
  if (document.runUrl !== `https://github.com/zryna/zryna/actions/runs/${document.runId}`) {
    reject('R406-GATES-RUN', 'gate run URL and ID differ');
  }
  for (const field of ['format', 'workflow', 'runId', 'runAttempt', 'runUrl', 'sourceCommit']) {
    if (document[field] !== expected[field]) {
      reject('R406-GATES-RUN', `${field} differs from the authenticated build input`);
    }
  }
  if (canonical(document.requiredJobs) !== canonical(expected.requiredJobs)) {
    reject('R406-GATES-JOBS', 'required jobs differ from the authenticated build input');
  }
  return document;
}

export function validatePreassemblyGatesText(text, input) {
  assertTextBounds(text);
  if (Buffer.byteLength(text, 'utf8') !== input?.gateReceipt?.size
    || sha256(text) !== input?.gateReceipt?.sha256) {
    reject('R406-GATES-DIGEST', 'gate bytes differ from the authenticated descriptor');
  }
  return validatePreassemblyGates(parseCanonical(text), input);
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    if (process.argv.length !== 4) {
      reject('R406-GATES-USAGE', 'expected build input and gate receipt paths');
    }
    const input = validateBuildInputText(readEnvelopeFile(resolve(process.argv[2])));
    validatePreassemblyGatesText(readEnvelopeFile(resolve(process.argv[3])), input);
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
