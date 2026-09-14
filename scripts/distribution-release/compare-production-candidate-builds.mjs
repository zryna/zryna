import { mkdirSync } from 'node:fs';
import { isAbsolute, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { canonical, canonicalBounded, parseCanonical, sha256 } from './canonical.mjs';
import {
  compareReleaseFiles, copyReleaseFile, createReleaseFile, exactReleaseNames,
  MAX_RELEASE_DOCUMENT, readReleaseFile,
} from './release-files.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const MANIFEST = 'production-candidate-build-result.json';
const TARGETS = new Map([
  ['x86_64-unknown-linux-gnu', 'tar.gz'], ['x86_64-pc-windows-msvc', 'zip'],
]);

function reject(message) {
  throw new Error(`R406-PRODUCTION-CANDIDATE-REPRODUCTION: ${message}`);
}

function exactKeys(value, keys, label) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)
    || Object.keys(value).sort().join('\0') !== [...keys].sort().join('\0')) {
    reject(`${label} fields differ`);
  }
}

function artifactNames(target) {
  const base = `zryna-0.2.3-${target}`;
  return {
    archive: `${base}.${TARGETS.get(target)}`,
    buildReceipt: `${base}.build-receipt.json`, sbom: `${base}.spdx.json`,
  };
}

export function validateProductionCandidateBuildResult(value, target, replica) {
  canonicalBounded(value);
  exactKeys(value, [
    'format', 'status', 'productionAdmission', 'version', 'target', 'replica',
    'observedSource', 'intendedRelease', 'recipe', 'artifacts',
  ], 'build result');
  if (value.format !== 'zryna.production-candidate-build-result.v1'
    || value.status !== 'production-candidate' || value.productionAdmission !== 'forbidden'
    || value.version !== '0.2.3' || value.target !== target || value.replica !== replica
    || value.observedSource?.ref !== 'refs/heads/main'
    || value.intendedRelease?.ref !== 'refs/tags/v0.2.3'
    || value.intendedRelease?.tagProvenance !== 'not-observed'
    || value.recipe?.format !== 'zryna.distribution-recipe.v1'
    || !/^[0-9a-f]{64}$/.test(value.recipe?.sha256 ?? '')) reject('build identity differs');
  exactKeys(value.observedSource,
    ['repository', 'ref', 'commit', 'tree', 'sourceDateEpoch'], 'observed source');
  if (value.observedSource.repository !== 'https://github.com/zryna/zryna'
    || !/^[0-9a-f]{40}$/.test(value.observedSource.commit)
    || !/^[0-9a-f]{40}$/.test(value.observedSource.tree)
    || !Number.isInteger(value.observedSource.sourceDateEpoch)
    || value.observedSource.sourceDateEpoch < 0
    || value.observedSource.sourceDateEpoch > 4294967295) reject('observed source differs');
  exactKeys(value.intendedRelease, ['ref', 'tagProvenance'], 'intended release');
  exactKeys(value.recipe, ['format', 'sha256'], 'recipe');
  exactKeys(value.artifacts, ['archive', 'buildReceipt', 'sbom'], 'artifacts');
  for (const [key, path] of Object.entries(artifactNames(target))) {
    const descriptor = value.artifacts[key];
    exactKeys(descriptor, ['path', 'size', 'sha256'], key);
    if (descriptor.path !== path || !Number.isInteger(descriptor.size) || descriptor.size < 1
      || descriptor.size > 2147483648 || !/^[0-9a-f]{64}$/.test(descriptor.sha256)) {
      reject(`${key} descriptor differs`);
    }
  }
  return value;
}

function load(root, target, replica) {
  const bytes = readReleaseFile(root, MANIFEST, MAX_RELEASE_DOCUMENT);
  const value = validateProductionCandidateBuildResult(
    parseCanonical(bytes.toString('utf8')), target, replica,
  );
  exactReleaseNames(root, [MANIFEST, ...Object.values(value.artifacts).map(({ path }) => path)]);
  return { bytes, value };
}

export function validateProductionCandidateReproduction(value, target) {
  canonicalBounded(value);
  exactKeys(value, [
    'format', 'status', 'productionAdmission', 'version', 'target', 'observedSource',
    'intendedRelease', 'recipe', 'artifacts', 'builds', 'comparison',
  ], 'reproduction');
  if (value.format !== 'zryna.production-candidate-reproduction.v1'
    || value.status !== 'production-candidate' || value.productionAdmission !== 'forbidden'
    || value.version !== '0.2.3' || value.target !== target
    || value.comparison !== 'byte-identical' || !Array.isArray(value.builds)
    || value.builds.length !== 2 || value.builds[0]?.replica !== 1
    || value.builds[1]?.replica !== 2
    || value.builds[0].manifestSha256 === value.builds[1].manifestSha256) {
    reject('reproduction identity differs');
  }
  for (const [index, build] of value.builds.entries()) {
    exactKeys(build, ['replica', 'manifestSha256'], `build ${index + 1}`);
    if (build.replica !== index + 1 || !/^[0-9a-f]{64}$/.test(build.manifestSha256)) {
      reject(`build ${index + 1} identity differs`);
    }
  }
  const projected = { ...value, format: 'zryna.production-candidate-build-result.v1', replica: 1 };
  delete projected.builds;
  delete projected.comparison;
  validateProductionCandidateBuildResult(projected, target, 1);
  return value;
}

export function compareProductionCandidateBuilds({ firstRoot, secondRoot, outputRoot, target }) {
  if (![firstRoot, secondRoot, outputRoot].every((path) => isAbsolute(path)
    && resolve(path) === path) || !TARGETS.has(target)
    || new Set([firstRoot, secondRoot, outputRoot]).size !== 3) reject('exact roots differ');
  const first = load(firstRoot, target, 1);
  const second = load(secondRoot, target, 2);
  const withoutReplica = ({ replica, ...value }) => value;
  if (canonical(withoutReplica(first.value)) !== canonical(withoutReplica(second.value))) {
    reject('candidate build identities differ');
  }
  const artifacts = {};
  for (const [key, descriptor] of Object.entries(first.value.artifacts)) {
    const compared = compareReleaseFiles(firstRoot, secondRoot, descriptor.path);
    if (canonical(compared) !== canonical(descriptor)) reject(`${key} bytes differ`);
    artifacts[key] = compared;
  }
  mkdirSync(outputRoot, { recursive: false, mode: 0o700 });
  for (const { path } of Object.values(artifacts)) copyReleaseFile(firstRoot, outputRoot, path);
  const { format: _format, replica: _replica, artifacts: _artifacts, ...identity } = first.value;
  const reproduction = validateProductionCandidateReproduction({
    ...identity, format: 'zryna.production-candidate-reproduction.v1', artifacts,
    builds: [
      { replica: 1, manifestSha256: sha256(first.bytes) },
      { replica: 2, manifestSha256: sha256(second.bytes) },
    ],
    comparison: 'byte-identical',
  }, target);
  createReleaseFile(outputRoot, 'production-candidate-reproduction.json',
    Buffer.from(`${canonicalBounded(reproduction)}\n`));
  exactReleaseNames(outputRoot, [
    'production-candidate-reproduction.json', ...Object.values(artifacts).map(({ path }) => path),
  ]);
  return reproduction;
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    if (process.argv.length !== 8 || process.argv[2] !== '--first'
      || process.argv[4] !== '--second' || process.argv[6] !== '--output') {
      reject('expected --first, --second, and --output once');
    }
    compareProductionCandidateBuilds({
      firstRoot: resolve(process.argv[3]), secondRoot: resolve(process.argv[5]),
      outputRoot: resolve(process.argv[7]), target: process.env.ZRYNA_TARGET,
    });
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
