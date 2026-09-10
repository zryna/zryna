import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv from 'ajv';
import { canonicalBounded, parseCanonical } from './canonical.mjs';
import { readEnvelopeFile } from './input.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const SCHEMA_PATH = fileURLToPath(new URL(
  '../../schemas/zryna-distribution-build-input-v1.schema.json', import.meta.url,
));
const ajv = new Ajv({ allErrors: true, strict: true });
const validateSchema = ajv.compile(JSON.parse(readFileSync(SCHEMA_PATH, 'utf8')));
const REQUIRED_JOBS = Object.freeze([
  'adapter', 'm0', 'm2', 'm3', 'rust (ubuntu-latest)', 'rust (windows-latest)',
]);
const REQUIRED_TOOLCHAINS = Object.freeze([
  Object.freeze({ name: 'cargo', version: '1.97.1', origin: 'repository:rust-toolchain.toml' }),
  Object.freeze({
    name: 'node', version: '22.22.1', origin: 'https://nodejs.org/dist/v22.22.1/',
  }),
  Object.freeze({ name: 'rustc', version: '1.97.1', origin: 'repository:rust-toolchain.toml' }),
]);

function reject(code, message) {
  throw new Error(`${code}: ${message}`);
}

function strictlySorted(values, project) {
  for (let index = 1; index < values.length; index += 1) {
    if (project(values[index - 1]) >= project(values[index])) return false;
  }
  return true;
}

export function validateBuildInput(document) {
  canonicalBounded(document);
  if (!validateSchema(document)) {
    reject('R406-BUILD-SCHEMA', ajv.errorsText(validateSchema.errors, { separator: '; ' }));
  }
  if (document.tag !== `v${document.version}` || document.source.ref !== `refs/tags/${document.tag}`) {
    reject('R406-BUILD-SOURCE', 'version, tag, and source ref differ');
  }
  const windows = document.target.triple === 'x86_64-pc-windows-msvc';
  const expectedFormat = windows ? 'zip' : 'tar-gzip';
  const expectedOs = windows ? 'windows' : 'linux';
  if (document.target.archiveFormat !== expectedFormat
    || document.target.platformBaseline.os !== expectedOs) {
    reject('R406-BUILD-TARGET', 'target, archive format, and platform baseline differ');
  }
  const expectedPaths = {
    materials: 'prepared/metadata/materials.json',
    preparedDistribution: 'prepared/metadata/distribution.json',
    compiledCli: `inputs/compiled-cli/zryna${windows ? '.exe' : ''}`,
    architectureReceipt: 'inputs/source-build-receipt.json',
    gateReceipt: 'inputs/preassembly-gates.json',
  };
  for (const [name, expected] of Object.entries(expectedPaths)) {
    if (document[name].logicalPath !== expected) {
      reject('R406-BUILD-PATH', `${name} must use ${expected}`);
    }
  }
  if (document.materials.fileCount >= document.preparedDistribution.archiveFileCount) {
    reject('R406-BUILD-MATERIALS', 'archive must add CLI and metadata to the material files');
  }
  if (document.architectureReceipt.sourceCommit !== document.source.commit
    || document.architectureReceipt.sourceTree !== document.source.tree
    || document.gateReceipt.sourceCommit !== document.source.commit) {
    reject('R406-BUILD-SOURCE', 'receipt source identity differs from build source');
  }
  if (document.gateReceipt.runUrl
    !== `https://github.com/zryna/zryna/actions/runs/${document.gateReceipt.runId}`) {
    reject('R406-BUILD-GATES', 'gate run URL and ID differ');
  }
  for (const [name, records] of [
    ['toolchains', document.toolchains],
    ['required jobs', document.gateReceipt.requiredJobs],
  ]) {
    if (!strictlySorted(records, (entry) => entry.name)) {
      reject('R406-BUILD-ORDER', `${name} must be unique and sorted by name`);
    }
  }
  for (const [index, expected] of REQUIRED_TOOLCHAINS.entries()) {
    const observed = document.toolchains[index];
    if (observed.name !== expected.name || observed.version !== expected.version
      || observed.origin !== expected.origin) {
      reject('R406-BUILD-TOOLCHAINS', 'toolchain names, versions, and origins must be exact');
    }
  }
  if (document.gateReceipt.requiredContexts.some((name, index) => name !== REQUIRED_JOBS[index])) {
    reject('R406-BUILD-GATES', 'current protected branch context set differs');
  }
  if (document.gateReceipt.requiredJobs.length !== REQUIRED_JOBS.length) {
    reject('R406-BUILD-GATES', 'required job set must be exact');
  }
  for (const [index, job] of document.gateReceipt.requiredJobs.entries()) {
    if (job.name !== REQUIRED_JOBS[index] || job.sourceCommit !== document.source.commit
      || job.runId !== document.gateReceipt.runId
      || job.runAttempt !== document.gateReceipt.runAttempt) {
      reject('R406-BUILD-GATES', 'required jobs must exactly bind the selected run attempt');
    }
  }
  return document;
}

export function validateBuildInputText(text) {
  return validateBuildInput(parseCanonical(text));
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    if (process.argv.length !== 3) reject('R406-BUILD-USAGE', 'expected one build input path');
    validateBuildInputText(readEnvelopeFile(resolve(process.argv[2])));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
