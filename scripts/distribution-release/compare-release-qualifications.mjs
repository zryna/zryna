import { mkdirSync, readFileSync } from 'node:fs';
import { isAbsolute, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv from 'ajv';
import { canonical, canonicalBounded, parseCanonical, sha256 } from './canonical.mjs';
import {
  compareReleaseFiles, copyReleaseFile, createReleaseFile, exactReleaseNames,
  MAX_RELEASE_DOCUMENT, MAX_RELEASE_FILE, readReleaseFile,
} from './release-files.mjs';
import { verifyQualification } from './release-qualification-core.mjs';
import { validateReleaseQualificationInputText } from './validate-release-qualification-input.mjs';
import { validateReleaseQualificationInspectionText } from './validate-release-qualification-inspection.mjs';

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

function readArtifact(root, descriptor) {
  const bytes = readReleaseFile(root, descriptor.path, MAX_RELEASE_FILE);
  if (bytes.length !== descriptor.size || sha256(bytes) !== descriptor.sha256) {
    reject(`${descriptor.path} bytes differ from their qualification descriptor`);
  }
  return bytes;
}

async function load(root, target, replica, primitives) {
  const bytes = readReleaseFile(root, MANIFEST, MAX_RELEASE_DOCUMENT);
  const value = validateQualificationResult(parseCanonical(bytes.toString('utf8')), target, replica);
  exactReleaseNames(root, [MANIFEST, ...Object.values(value.artifacts).map(({ path }) => path)]);
  const artifacts = Object.fromEntries(Object.entries(value.artifacts)
    .map(([key, descriptor]) => [key, readArtifact(root, descriptor)]));
  const input = validateReleaseQualificationInputText(artifacts.binding.toString('utf8'));
  const boundIdentity = {
    versionCandidate: input.versionCandidate,
    target: input.target.triple,
    source: {
      ref: input.source.ref,
      commit: input.source.commit,
      tree: input.source.tree,
      sourceDateEpoch: input.source.sourceDateEpoch,
    },
    recipeProposal: {
      size: input.recipeProposal.size,
      sha256: input.recipeProposal.sha256,
    },
  };
  const claimedIdentity = {
    versionCandidate: value.versionCandidate,
    target: value.target,
    source: value.source,
    recipeProposal: value.recipeProposal,
  };
  if (canonical(claimedIdentity) !== canonical(boundIdentity)) {
    reject('qualification result identity differs from its verified binding');
  }
  validateReleaseQualificationInspectionText(artifacts.inspection.toString('utf8'), input,
    artifacts.cli);
  const verified = await verifyQualification(artifacts.archive, {
    filename: value.artifacts.archive.path,
    size: value.artifacts.archive.size,
    sha256: value.artifacts.archive.sha256,
    binding: artifacts.binding,
  }, primitives);
  const cliPath = target === 'x86_64-pc-windows-msvc' ? 'zryna.exe' : 'bin/zryna';
  const decodedCli = verified.files.filter(({ path }) => path === cliPath);
  const decodedInventory = verified.files.filter(
    ({ path }) => path === 'qualification/inventory.json',
  );
  if (decodedCli.length !== 1 || !decodedCli[0].data.equals(artifacts.cli)
    || decodedInventory.length !== 1 || !decodedInventory[0].data.equals(artifacts.inventory)) {
    reject('verified archive CLI or inventory differs from the qualification artifacts');
  }
  return { bytes, value };
}

export async function compareReleaseQualifications({
  firstRoot, secondRoot, outputRoot, target, primitives,
}) {
  if (![firstRoot, secondRoot, outputRoot].every((path) => isAbsolute(path)
    && resolve(path) === path) || new Set([firstRoot, secondRoot, outputRoot]).size !== 3
    || !TARGETS.has(target)) reject('exact distinct roots and supported target are required');
  const [first, second] = await Promise.all([
    load(firstRoot, target, 1, primitives), load(secondRoot, target, 2, primitives),
  ]);
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
  for (const descriptor of Object.values(artifacts)) {
    copyReleaseFile(firstRoot, outputRoot, descriptor.path);
    readArtifact(outputRoot, descriptor);
  }
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
    await compareReleaseQualifications({
      firstRoot: resolve(process.argv[3]), secondRoot: resolve(process.argv[5]),
      outputRoot: resolve(process.argv[7]), target: process.env.ZRYNA_TARGET,
    });
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
