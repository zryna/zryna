import { mkdirSync, readFileSync } from 'node:fs';
import { isAbsolute, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { TextDecoder } from 'node:util';
import Ajv from 'ajv';
import { canonical, canonicalBounded, parseCanonical, sha256 } from './canonical.mjs';
import {
  compareReleaseFiles, copyReleaseFile, createReleaseFile, exactReleaseNames,
  MAX_RELEASE_DOCUMENT, readReleaseFile,
} from './release-files.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const SCHEMA_PATH = fileURLToPath(new URL(
  '../../schemas/zryna-release-build-result-v1.schema.json', import.meta.url,
));
const REPRODUCTION_SCHEMA_PATH = fileURLToPath(new URL(
  '../../schemas/zryna-release-reproduction-v1.schema.json', import.meta.url,
));
const validateSchema = new Ajv({ allErrors: true, strict: true })
  .compile(JSON.parse(readFileSync(SCHEMA_PATH, 'utf8')));
const validateReproductionSchema = new Ajv({ allErrors: true, strict: true })
  .compile(JSON.parse(readFileSync(REPRODUCTION_SCHEMA_PATH, 'utf8')));
const TARGETS = new Map([
  ['x86_64-unknown-linux-gnu', 'tar.gz'],
  ['x86_64-pc-windows-msvc', 'zip'],
]);
const MANIFEST = 'build-result.json';

function reject(message) {
  throw new Error(`R406-REPRODUCTION: ${message}`);
}

function text(bytes) {
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    reject('build result is not UTF-8');
  }
}

function validateResult(value, expectedTarget, expectedReplica) {
  canonicalBounded(value);
  if (!validateSchema(value)) reject('build result schema differs');
  if (value.target !== expectedTarget || value.replica !== expectedReplica) {
    reject('build result target or replica differs');
  }
  const base = `zryna-0.2.0-${expectedTarget}`;
  const expected = {
    archive: `${base}.${TARGETS.get(expectedTarget)}`,
    buildReceipt: `${base}.build-receipt.json`,
    sbom: `${base}.spdx.json`,
  };
  for (const [key, path] of Object.entries(expected)) {
    if (value.artifacts[key].path !== path) reject(`${key} release path differs`);
  }
  return value;
}

function load(root, target, replica) {
  const bytes = readReleaseFile(root, MANIFEST, MAX_RELEASE_DOCUMENT);
  const value = validateResult(parseCanonical(text(bytes)), target, replica);
  const names = [MANIFEST, ...Object.values(value.artifacts).map(({ path }) => path)];
  exactReleaseNames(root, names);
  return { bytes, value };
}

function sameIdentity(left, right) {
  const project = ({ replica, ...value }) => value;
  return canonical(project(left)) === canonical(project(right));
}

export function validateReproduction(value, expectedTarget) {
  canonicalBounded(value);
  if (!validateReproductionSchema(value)) reject('reproduction result schema differs');
  if (value.target !== expectedTarget
    || value.builds[0].replica !== 1 || value.builds[1].replica !== 2
    || value.builds[0].manifestSha256 === value.builds[1].manifestSha256) {
    reject('reproduction target or independent build identities differ');
  }
  const base = `zryna-0.2.0-${expectedTarget}`;
  const expected = {
    archive: `${base}.${TARGETS.get(expectedTarget)}`,
    buildReceipt: `${base}.build-receipt.json`,
    sbom: `${base}.spdx.json`,
  };
  for (const [key, path] of Object.entries(expected)) {
    if (value.artifacts[key].path !== path) reject(`${key} reproduced path differs`);
  }
  return value;
}

export function validateReproductionText(value, expectedTarget) {
  return validateReproduction(parseCanonical(value), expectedTarget);
}

export function compareReleaseBuilds({ firstRoot, secondRoot, outputRoot, target }) {
  if (![firstRoot, secondRoot, outputRoot].every((path) => isAbsolute(path) && resolve(path) === path)
    || !TARGETS.has(target) || new Set([firstRoot, secondRoot, outputRoot]).size !== 3) {
    reject('exact distinct absolute roots and supported target are required');
  }
  const first = load(firstRoot, target, 1);
  const second = load(secondRoot, target, 2);
  if (!sameIdentity(first.value, second.value)) reject('build result identities differ');
  const compared = {};
  for (const [key, descriptor] of Object.entries(first.value.artifacts)) {
    const observed = compareReleaseFiles(firstRoot, secondRoot, descriptor.path);
    if (canonical(observed) !== canonical(descriptor)
      || canonical(observed) !== canonical(second.value.artifacts[key])) {
      reject(`${key} bytes differ from the build descriptors`);
    }
    compared[key] = observed;
  }
  mkdirSync(outputRoot, { recursive: false, mode: 0o700 });
  for (const { path } of Object.values(compared)) copyReleaseFile(firstRoot, outputRoot, path);
  const reproduction = {
    format: 'zryna.release-reproduction.v1',
    version: first.value.version,
    target,
    source: first.value.source,
    recipe: first.value.recipe,
    artifacts: compared,
    builds: [
      { replica: 1, manifestSha256: sha256(first.bytes) },
      { replica: 2, manifestSha256: sha256(second.bytes) },
    ],
    comparison: 'byte-identical',
  };
  validateReproduction(reproduction, target);
  createReleaseFile(outputRoot, 'reproduction.json',
    Buffer.from(`${canonicalBounded(reproduction)}\n`));
  exactReleaseNames(outputRoot, [
    'reproduction.json', ...Object.values(compared).map(({ path }) => path),
  ]);
  return reproduction;
}

function argumentsFrom(argv) {
  const values = new Map();
  for (let index = 0; index < argv.length; index += 2) {
    if (!['--first', '--second', '--output'].includes(argv[index]) || argv[index + 1] === undefined
      || values.has(argv[index])) reject('expected --first, --second, and --output once');
    values.set(argv[index], resolve(argv[index + 1]));
  }
  if (values.size !== 3) reject('expected --first, --second, and --output once');
  return values;
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    const values = argumentsFrom(process.argv.slice(2));
    compareReleaseBuilds({
      firstRoot: values.get('--first'),
      secondRoot: values.get('--second'),
      outputRoot: values.get('--output'),
      target: process.env.ZRYNA_TARGET,
    });
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
