import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import Ajv from 'ajv';
import { canonicalBounded, parseCanonical, sha256 } from './canonical.mjs';

const SCHEMA_PATH = fileURLToPath(new URL(
  '../../schemas/zryna-release-qualification-inspection-v1.schema.json', import.meta.url,
));
const ajv = new Ajv({ allErrors: true, strict: true });
const validateSchema = ajv.compile(JSON.parse(readFileSync(SCHEMA_PATH, 'utf8')));

function reject(message) {
  throw new Error(`R406-QUALIFICATION-INSPECTION: ${message}`);
}

export function validateReleaseQualificationInspection(value, input, cli) {
  canonicalBounded(value);
  if (!validateSchema(value)) {
    reject(ajv.errorsText(validateSchema.errors, { separator: '; ' }));
  }
  if (value.target !== input.target.triple || value.sourceCommit !== input.source.commit
    || value.bindingSha256 !== sha256(Buffer.from(`${canonicalBounded(input)}\n`))
    || value.binary.format !== (value.target === 'x86_64-pc-windows-msvc' ? 'PE' : 'ELF')
    || !Buffer.isBuffer(cli) || value.binary.size !== cli.length
    || value.binary.sha256 !== sha256(cli)) {
    reject('inspection, qualification input, and binary identities differ');
  }
  const dependencies = value.checks.dynamicDependencies;
  if (dependencies.some((dependency, index) => index > 0
    && dependencies[index - 1].toLowerCase() >= dependency.toLowerCase())) {
    reject('dynamic dependencies must be unique and case-insensitively sorted');
  }
  return value;
}

export function validateReleaseQualificationInspectionText(text, input, cli) {
  return validateReleaseQualificationInspection(parseCanonical(text), input, cli);
}
