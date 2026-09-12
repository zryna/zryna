import { mkdirSync, readFileSync } from 'node:fs';
import { isAbsolute, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv from 'ajv';
import { canonical, canonicalBounded, parseCanonical, sha256 } from './canonical.mjs';
import {
  compareReleaseFiles, copyReleaseFile, createReleaseFile, exactReleaseNames,
  MAX_RELEASE_DOCUMENT, readReleaseFile,
} from './release-files.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const SCHEMA_PATH = fileURLToPath(new URL(
  '../../schemas/zryna-release-qualification-result-v1.schema.json', import.meta.url,
));
const validateSchema = new Ajv({ allErrors: true, strict: true })
  .compile(JSON.parse(readFileSync(SCHEMA_PATH, 'utf8')));
const TARGETS = new Set(['x86_64-pc-windows-msvc', 'x86_64-unknown-linux-gnu']);
const MANIFEST = 'qualification-result.json';

function reject(message) {
  throw new Error(`R406-QUALIFICATION-COMPARISON: ${message}`);
}

export function validateQualificationResult(value, target, replica) {
  canonicalBounded(value);
  if (!validateSchema(value)) reject('qualification result schema differs');
  if (value.target !== target || value.replica !== replica) {
    reject('qualification target or replica differs');
  }
  const base = `zryna-qualification-0.2.0-${target}-${value.source.commit.slice(0, 12)}`;
  const expected = {
    binding: `${base}.input.json`,
    cli: `${base}.${target === 'x86_64-pc-windows-msvc' ? 'exe' : 'elf'}`,
    inventory: `${base}.inventory.json`,
    archive: `${base}.${target === 'x86_64-pc-windows-msvc' ? 'zip' : 'tar.gz'}`,
    inspection: `${base}.inspection.json`,
  };
  for (const [key, path] of Object.entries(expected)) {
    if (value.artifacts[key].path !== path) reject(`${key} qualification path differs`);
  }
  return value;
}

function load(root, target, replica) {
  const bytes = readReleaseFile(root, MANIFEST, MAX_RELEASE_DOCUMENT);
  const value = validateQualificationResult(parseCanonical(bytes.toString('utf8')), target, replica);
  exactReleaseNames(root, [MANIFEST, ...Object.values(value.artifacts).map(({ path }) => path)]);
  return { bytes, value };
}

export function compareReleaseQualifications({ firstRoot, secondRoot, outputRoot, target }) {
  if (![firstRoot, secondRoot, outputRoot].every((path) => isAbsolute(path)
    && resolve(path) === path) || new Set([firstRoot, secondRoot, outputRoot]).size !== 3
    || !TARGETS.has(target)) reject('exact distinct roots and supported target are required');
  const first = load(firstRoot, target, 1);
  const second = load(secondRoot, target, 2);
  const project = ({ replica, ...value }) => value;
  if (canonical(project(first.value)) !== canonical(project(second.value))) {
    reject('qualification result identities differ');
  }
  const artifacts = {};
  for (const [key, expected] of Object.entries(first.value.artifacts)) {
    const observed = compareReleaseFiles(firstRoot, secondRoot, expected.path);
    if (canonical(observed) !== canonical(expected)
      || canonical(observed) !== canonical(second.value.artifacts[key])) {
      reject(`${key} bytes differ from qualification descriptors`);
    }
    artifacts[key] = observed;
  }
  mkdirSync(outputRoot, { recursive: false, mode: 0o700 });
  for (const { path } of Object.values(artifacts)) copyReleaseFile(firstRoot, outputRoot, path);
  const comparison = {
    format: 'zryna.release-qualification-comparison.v1',
    status: 'qualification-only',
    productionAdmission: 'forbidden',
    versionCandidate: '0.2.0',
    target,
    source: first.value.source,
    recipeProposal: first.value.recipeProposal,
    artifacts,
    builds: [
      { replica: 1, manifestSha256: sha256(first.bytes) },
      { replica: 2, manifestSha256: sha256(second.bytes) },
    ],
    comparison: 'byte-identical',
  };
  createReleaseFile(outputRoot, 'qualification-comparison.json',
    Buffer.from(`${canonicalBounded(comparison)}\n`));
  exactReleaseNames(outputRoot, [
    'qualification-comparison.json', ...Object.values(artifacts).map(({ path }) => path),
  ]);
  return comparison;
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    if (process.argv.length !== 8 || process.argv[2] !== '--first'
      || process.argv[4] !== '--second' || process.argv[6] !== '--output') {
      reject('expected --first, --second, and --output once');
    }
    compareReleaseQualifications({
      firstRoot: resolve(process.argv[3]), secondRoot: resolve(process.argv[5]),
      outputRoot: resolve(process.argv[7]), target: process.env.ZRYNA_TARGET,
    });
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
