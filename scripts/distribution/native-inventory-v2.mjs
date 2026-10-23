// Source-candidate native installations reuse the original finite content validators.
// This grammar supplies neither a release signature nor authority to execute SDK tools.
import { readFileSync } from 'node:fs';
import { LIMITS, bytes, exactKeys, parseCanonical, requireValue, sha256 } from './canonical.mjs';
import { digestValue, roleFor, targetPaths, validateTuples } from './inventory.mjs';

export const NATIVE_FORMAT = 'zryna.native-distribution.v2';
export const NATIVE_RECIPE_SHA256 = '7d16785043fda96277177353aa2dff379861c9a75a7e557e80a9762b8425c339';
const rawRecipe = readFileSync(new URL('./native-recipe-v2.json', import.meta.url));
requireValue(sha256(rawRecipe) === NATIVE_RECIPE_SHA256, 'native recipe identity');
export const nativeRecipe = parseCanonical(rawRecipe);
const materialRecords = nativeRecipe.rustMaterialRecords.map(pin => {
  const raw = readFileSync(new URL(pin.path, import.meta.url));
  requireValue(sha256(raw) === pin.sha256, 'native Rust supplier record identity');
  return parseCanonical(raw);
});
export const NATIVE_TEXT = Object.freeze(['LICENSE', 'NOTICE', 'README.md', 'SUPPORT.md', 'VERSION']);
const METADATA = ['metadata/materials.json', 'metadata/architecture-receipt.json',
  'metadata/distribution.json', 'metadata/inventory.json', 'metadata/checksums.sha256'];

export function nativeSourceRef(value) {
  if (value === 'refs/heads/main') return true;
  if (typeof value !== 'string' || !value.startsWith('refs/heads/codex/')
    || value.length > 200 || value.split('/').length > 12) return false;
  return value.slice('refs/heads/codex/'.length).split('/').every(part =>
    /^[a-z0-9][a-z0-9._-]{0,63}$/.test(part) && !part.includes('..')
    && !part.endsWith('.') && !part.endsWith('.lock'));
}

export function nativeRustMaterials(target) {
  targetPaths(target);
  const records = materialRecords.flatMap(record => {
    const selected = new Set(record.targets[target]);
    return record.packages.filter(item => selected.has(`${item.name}-${item.version}`));
  }).sort((left, right) => `${left.name}-${left.version}` < `${right.name}-${right.version}` ? -1 : 1);
  requireValue(new Set(records.map(item => `${item.name}-${item.version}`)).size === records.length,
    'native Rust supplier duplicate');
  return structuredClone(records);
}

export function nativeRole(path, target) {
  if (path === targetPaths(target).cli) return 'cli';
  requireValue(NATIVE_TEXT.includes(path) || METADATA.includes(path)
    || /^licenses\/rust\/[^/]+\/[^/]+$/.test(path), 'path has no native installation role');
  const role = roleFor(path, target);
  requireValue(!['provider', 'runtime'].includes(role), 'native runtime payload forbidden');
  return role;
}

export function validateNativeTuples(entries, target, { complete = false } = {}) {
  requireValue(Array.isArray(entries), 'native finite file inventory');
  validateTuples(entries, target);
  for (const file of entries) {
    requireValue(file.role === nativeRole(file.path, target), 'native role mismatch');
  }
  if (!complete) return;
  const paths = new Set(entries.map(file => file.path));
  for (const path of [...NATIVE_TEXT, 'metadata/materials.json', 'metadata/architecture-receipt.json']) {
    requireValue(paths.has(path), 'missing required native payload');
  }
  const expected = nativeRustMaterials(target).flatMap(record => record.files)
    .map(({ path, size, sha256: digest }) => ({ path, size, sha256: digest }))
    .sort((left, right) => left.path < right.path ? -1 : 1);
  const actual = entries.filter(file => file.path.startsWith('licenses/rust/'))
    .map(({ path, size, sha256: digest }) => ({ path, size, sha256: digest }));
  requireValue(bytes(actual).equals(bytes(expected)), 'native Rust supplier notice closure');
}

export function validateNativeDistribution(record) {
  exactKeys(record, ['format', 'version', 'source', 'target', 'recipe', 'files']);
  requireValue(record.format === NATIVE_FORMAT && record.version === nativeRecipe.compilerVersion,
    'native distribution identity');
  exactKeys(record.source, ['repository', 'ref', 'commit', 'tree', 'sourceDateEpoch']);
  requireValue(record.source.repository === nativeRecipe.sourceRepository
    && nativeSourceRef(record.source.ref)
    && /^[a-f0-9]{40}$/.test(record.source.commit) && /^[a-f0-9]{40}$/.test(record.source.tree),
  'native exact source candidate');
  requireValue(Number.isSafeInteger(record.source.sourceDateEpoch)
    && record.source.sourceDateEpoch >= 0 && record.source.sourceDateEpoch <= 0xffffffff,
  'native source epoch');
  exactKeys(record.target, ['triple', 'archiveFormat', 'platformBaseline']);
  const paths = targetPaths(record.target.triple);
  const baseline = record.target.triple === 'x86_64-unknown-linux-gnu'
    ? { os: 'linux', distribution: 'ubuntu', version: '24.04', architecture: 'x86_64' }
    : { os: 'windows', product: 'windows-server', version: '2022', architecture: 'x86_64',
      runtime: 'operating-system-ucrt' };
  requireValue(record.target.archiveFormat === paths.format
    && bytes(record.target.platformBaseline).equals(bytes(baseline)), 'native target baseline');
  exactKeys(record.recipe, ['format', 'sha256']);
  requireValue(record.recipe.format === nativeRecipe.format
    && record.recipe.sha256 === NATIVE_RECIPE_SHA256, 'native recipe binding');
  digestValue(record.recipe.sha256);
  validateNativeTuples(record.files, record.target.triple, { complete: true });
  requireValue(record.files.length <= LIMITS.files - 4, 'native complete file count');
  requireValue(record.files.every(file => ![paths.cli, 'metadata/distribution.json',
    'metadata/inventory.json', 'metadata/checksums.sha256'].includes(file.path)), 'native cyclic payload');
}
