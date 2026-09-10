import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv from 'ajv';
import { assertTextBounds, canonicalBounded, parseCanonical, sha256 } from './canonical.mjs';
import { readEnvelopeFile } from './input.mjs';
import { validateBuildInputText } from './validate-build-input.mjs';

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

export function validateSourceBuildReceipt(document, input) {
  canonicalBounded(document);
  if (!validateSchema(document)) {
    reject('R406-ARCH-SCHEMA', ajv.errorsText(validateSchema.errors, { separator: '; ' }));
  }
  const paths = document.inputs.map(({ logicalPath }) => logicalPath);
  if (paths.some((path, index) => path !== INPUT_PATHS[index])) {
    reject('R406-ARCH-INPUTS', 'source authority inputs must be exact and sorted');
  }
  if (input) {
    if (document.source.repository !== input.source?.repository
      || document.source.commit !== input.source?.commit
      || document.source.tree !== input.source?.tree) {
      reject('R406-ARCH-SOURCE', 'receipt and build source identities differ');
    }
    const cargo = input.toolchains?.find(({ name }) => name === 'cargo');
    const rustc = input.toolchains?.find(({ name }) => name === 'rustc');
    if (cargo?.version !== document.toolchain.channel
      || cargo?.sha256 !== document.toolchain.cargoSha256
      || rustc?.version !== document.toolchain.channel
      || rustc?.sha256 !== document.toolchain.rustcSha256) {
      reject('R406-ARCH-TOOLCHAIN', 'receipt and build toolchain identities differ');
    }
  }
  return document;
}

export function validateSourceBuildReceiptText(text, input) {
  assertTextBounds(text);
  if (input && (Buffer.byteLength(text, 'utf8') !== input.architectureReceipt?.size
    || sha256(text) !== input.architectureReceipt?.sha256)) {
    reject('R406-ARCH-DIGEST', 'receipt bytes differ from the build descriptor');
  }
  return validateSourceBuildReceipt(parseCanonical(text), input);
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    if (process.argv.length !== 4) {
      reject('R406-ARCH-USAGE', 'expected build input and receipt paths');
    }
    const input = validateBuildInputText(readEnvelopeFile(resolve(process.argv[2])));
    validateSourceBuildReceiptText(readEnvelopeFile(resolve(process.argv[3])), input);
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
