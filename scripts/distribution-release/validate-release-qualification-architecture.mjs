import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import Ajv from 'ajv';
import { canonicalBounded } from './canonical.mjs';

const SCHEMA_PATH = fileURLToPath(new URL(
  '../../schemas/zryna-release-qualification-architecture-v1.schema.json', import.meta.url,
));
const ajv = new Ajv({ allErrors: true, strict: true });
const validateSchema = ajv.compile(JSON.parse(readFileSync(SCHEMA_PATH, 'utf8')));
const INPUT_PATHS = Object.freeze([
  'Cargo.lock', 'Cargo.toml', 'rust-toolchain.toml', 'zryna.workspace.json',
]);

export function validateReleaseQualificationArchitecture(value) {
  canonicalBounded(value);
  if (!validateSchema(value)) {
    throw new Error(`R406-QUALIFICATION-ARCH-SCHEMA: ${
      ajv.errorsText(validateSchema.errors, { separator: '; ' })}`);
  }
  if (value.inputs.some(({ logicalPath }, index) => logicalPath !== INPUT_PATHS[index])) {
    throw new Error('R406-QUALIFICATION-ARCH-INPUTS: source inputs must be exact and sorted');
  }
  return value;
}
