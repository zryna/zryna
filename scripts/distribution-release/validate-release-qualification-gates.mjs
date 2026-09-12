import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import Ajv from 'ajv';
import { canonicalBounded } from './canonical.mjs';

const SCHEMA_PATH = fileURLToPath(new URL(
  '../../schemas/zryna-release-qualification-gates-v1.schema.json', import.meta.url,
));
const ajv = new Ajv({ allErrors: true, strict: true });
const validateSchema = ajv.compile(JSON.parse(readFileSync(SCHEMA_PATH, 'utf8')));
const JOBS = Object.freeze([
  'adapter', 'm0', 'm2', 'm3', 'rust (ubuntu-latest)', 'rust (windows-latest)',
]);

export function validateReleaseQualificationGates(value) {
  canonicalBounded(value);
  if (!validateSchema(value)) {
    throw new Error(`R406-QUALIFICATION-GATES: ${
      ajv.errorsText(validateSchema.errors, { separator: '; ' })}`);
  }
  if (value.requiredContexts.some((name, index) => name !== JOBS[index])
    || value.requiredJobs.some((job, index) => job.name !== JOBS[index]
      || job.sourceCommit !== value.sourceCommit || job.runId !== value.runId
      || job.runAttempt !== value.runAttempt)) {
    throw new Error('R406-QUALIFICATION-GATES: protected job projection differs');
  }
  return value;
}
