import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv from 'ajv';
import { canonical, canonicalBounded, parseCanonical } from './canonical.mjs';
import { readEnvelopeFile } from './input.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const SCHEMA_PATH = fileURLToPath(new URL(
  '../../schemas/zryna-release-qualification-input-v1.schema.json', import.meta.url,
));
const ajv = new Ajv({ allErrors: true, strict: true });
const validateSchema = ajv.compile(JSON.parse(readFileSync(SCHEMA_PATH, 'utf8')));
const JOBS = Object.freeze([
  'adapter', 'm0', 'm2', 'm3', 'rust (ubuntu-latest)', 'rust (windows-latest)',
]);
const COMMAND = (target) => [
  'cargo', 'rustc', '--locked', '--release', '--target', target,
  '-p', 'zryna', '--bin', 'zryna',
];

function reject(code, message) {
  throw new Error(`${code}: ${message}`);
}

function strictlySorted(values, project = (value) => value) {
  for (let index = 1; index < values.length; index += 1) {
    if (project(values[index - 1]) >= project(values[index])) return false;
  }
  return true;
}

export function validateReleaseQualificationInput(document) {
  canonicalBounded(document);
  if (!validateSchema(document)) {
    reject('R406-QUALIFICATION-INPUT-SCHEMA',
      ajv.errorsText(validateSchema.errors, { separator: '; ' }));
  }
  const target = document.target.triple;
  const expectedRoot = `zryna-qualification-0.2.0-${target}-${document.source.commit.slice(0, 12)}`;
  const expectedFormat = target === 'x86_64-pc-windows-msvc' ? 'zip' : 'tar-gzip';
  if (document.archive.root !== expectedRoot || document.archive.format !== expectedFormat) {
    reject('R406-QUALIFICATION-ARCHIVE', 'target, root, and archive format differ');
  }
  if (document.architectureReceipt.sourceCommit !== document.source.commit
    || document.architectureReceipt.sourceTree !== document.source.tree
    || document.gateReceipt.sourceCommit !== document.source.commit) {
    reject('R406-QUALIFICATION-SOURCE', 'receipt source identity differs');
  }
  const expectedPaths = [
    [document.sourceReceipt.logicalPath, 'qualification/source.json'],
    [document.architectureReceipt.logicalPath, 'qualification/architecture.json'],
    [document.gateReceipt.logicalPath, 'qualification/gates.json'],
  ];
  if (expectedPaths.some(([actual, expected]) => actual !== expected)) {
    reject('R406-QUALIFICATION-PATH', 'receipt logical path differs');
  }
  if (canonical(document.compile.argv) !== canonical(COMMAND(target))) {
    reject('R406-QUALIFICATION-COMPILE', 'compile command differs');
  }
  for (const [name, values, project] of [
    ['material paths', document.materials.files, ({ path }) => path],
    ['toolchains', document.toolchains, ({ name: value }) => value],
    ['native tools', document.nativeTools, ({ name: value }) => value],
    ['environment', document.compile.environment, ({ name: value }) => value],
  ]) {
    if (!strictlySorted(values, project)) {
      reject('R406-QUALIFICATION-ORDER', `${name} must be unique and sorted`);
    }
  }
  for (const file of document.materials.files) {
    if (!strictlySorted(file.licenses)) {
      reject('R406-QUALIFICATION-ORDER', `${file.path} licenses must be unique and sorted`);
    }
  }
  const expectedTools = [
    ['cargo', '1.97.1', 'repository:rust-toolchain.toml'],
    ['node', '22.22.1', 'https://nodejs.org/dist/v22.22.1/'],
    ['rustc', '1.97.1', 'repository:rust-toolchain.toml'],
  ];
  if (document.toolchains.some((tool, index) => canonical([
    tool.name, tool.version, tool.origin,
  ]) !== canonical(expectedTools[index]))) {
    reject('R406-QUALIFICATION-TOOLS', 'primary toolchain identity differs');
  }
  const jobs = document.gateReceipt.requiredJobs;
  if (jobs.some((job, index) => job.name !== JOBS[index]
    || job.sourceCommit !== document.source.commit)) {
    reject('R406-QUALIFICATION-GATES', 'required protected jobs differ');
  }
  return document;
}

export function validateReleaseQualificationInputText(text) {
  return validateReleaseQualificationInput(parseCanonical(text));
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    if (process.argv.length !== 3) {
      reject('R406-QUALIFICATION-INPUT-USAGE', 'expected one qualification input path');
    }
    validateReleaseQualificationInputText(readEnvelopeFile(resolve(process.argv[2])));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
