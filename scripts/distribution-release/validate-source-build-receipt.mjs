import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv from 'ajv';
import { canonicalBounded, parseCanonical } from './canonical.mjs';
import { readEnvelopeFile } from './input.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const SCHEMA_PATH = fileURLToPath(new URL(
  '../../schemas/zryna-source-build-receipt-v1.schema.json', import.meta.url,
));
const ajv = new Ajv({ allErrors: true, strict: true });
const validateSchema = ajv.compile(JSON.parse(readFileSync(SCHEMA_PATH, 'utf8')));
const INPUT_PATHS = Object.freeze([
  'Cargo.lock', 'Cargo.toml', 'rust-toolchain.toml', 'zryna.workspace.json',
]);

function reject(code, message) {
  throw new Error(`${code}: ${message}`);
}

export function validateSourceBuildReceipt(document) {
  canonicalBounded(document);
  if (!validateSchema(document)) {
    reject('R406-ARCH-SCHEMA', ajv.errorsText(validateSchema.errors, { separator: '; ' }));
  }
  const paths = document.inputs.map(({ logicalPath }) => logicalPath);
  if (paths.some((path, index) => path !== INPUT_PATHS[index])) {
    reject('R406-ARCH-INPUTS', 'source authority inputs must be exact and sorted');
  }
  if (!document.toolchain.cargoVersion.startsWith('cargo 1.97.1 ')
    || !document.toolchain.rustcVersion.startsWith('rustc 1.97.1 ')) {
    reject('R406-ARCH-TOOLCHAIN', 'observed cargo and rustc versions must match the pin');
  }
  return document;
}

export function validateSourceBuildReceiptText(text) {
  return validateSourceBuildReceipt(parseCanonical(text));
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    if (process.argv.length !== 3) reject('R406-ARCH-USAGE', 'expected one receipt path');
    validateSourceBuildReceiptText(readEnvelopeFile(resolve(process.argv[2])));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
