import { readFileSync } from 'node:fs';
import { bytes, exactKeys, orderedPaths, parseCanonical, requireValue, sha256 }
  from '../../distribution/canonical.mjs';
import { validateSource } from '../../../examples/playground/restricted/daemon/compiler-publication-constants.mjs';
import { compilerMetadataSelection, COMPILER_TARGET, selectCompilerGraph } from './material-graph.mjs';
import { identity, REGISTRY } from './material-lock.mjs';
import { validateCompilerCrateCache } from './material-cache.mjs';
import { captureCompilerLicenses } from './material-licenses.mjs';

export const MATERIAL_PINS = Object.freeze({
  base: '32b6343a6dfae19d7c3cdcc50658762c2c0ad08aca15124bdce85b28db38a84b',
  additions: '499be6581463e71e54ede6d2c5807c346173f34e7505e6faeaae95944c254483',
});

function approvedRecords() {
  const base = readFileSync(new URL('../../distribution/materials-rust-v1.json', import.meta.url));
  const additions = readFileSync(new URL('../../distribution/materials-rust-command-candidate-v1.json', import.meta.url));
  requireValue(sha256(base) === MATERIAL_PINS.base && sha256(additions) === MATERIAL_PINS.additions,
    'compiler independently approved material descriptor pins');
  const record = parseCanonical(base);
  const candidate = parseCanonical(additions);
  requireValue(candidate.baseMaterialRecordSha256 === MATERIAL_PINS.base
    && candidate.status === 'review-candidate' && candidate.productionAdmission === 'forbidden',
  'compiler descriptor-only candidate status');
  const records = new Map();
  for (const entry of [...record.packages, ...candidate.packages]) {
    requireValue(!records.has(identity(entry)), 'compiler approved material ambiguity');
    records.set(identity(entry), entry);
  }
  return records;
}
const APPROVED = approvedRecords();

function validateMaterialSource(source) {
  const keys = ['repository', 'commit', 'tree', 'sourceDateEpoch'];
  if (Object.keys(source ?? {}).length === 7) return validateSource(source);
  exactKeys(source, keys);
  requireValue(source.repository === 'https://github.com/zryna/zryna'
    && /^[0-9a-f]{40}$/.test(source.commit) && /^[0-9a-f]{40}$/.test(source.tree)
    && Number.isSafeInteger(source.sourceDateEpoch) && source.sourceDateEpoch >= 0
    && source.sourceDateEpoch <= 0xffffffff, 'compiler honest candidate source');
}

// Cargo's locked/offline/filter-platform invocation and raw metadata are retained by the builder.
// This helper never promotes the old CLI candidate or runs a dependency resolver.
export function selectCompilerCrates(metadata, sourceRoot) {
  const { selected, packages } = compilerMetadataSelection(metadata, sourceRoot);
  const seen = new Set();
  const result = [];
  for (const id of selected) {
    const pkg = packages.get(id);
    requireValue(!seen.has(identity(pkg)), 'compiler selected material identity ambiguity');
    seen.add(identity(pkg));
    if (pkg.source !== null) {
      requireValue(pkg.source === REGISTRY && APPROVED.has(identity(pkg)),
        'compiler selected unknown external material');
      result.push({ name: pkg.name, version: pkg.version, manifest_path: pkg.manifest_path,
        source: pkg.source });
    }
  }
  return result.sort((a, b) => identity(a) < identity(b) ? -1 : 1);
}

export function captureCompilerMaterials(input) {
  exactKeys(input, ['metadata', 'source', 'sourceRoot', 'cargoLock', 'crateArchives', 'upstreamLicense']);
  const { metadata, source, sourceRoot, cargoLock, crateArchives, upstreamLicense } = input;
  validateMaterialSource(source);
  const { external, proof } = selectCompilerGraph({ metadata, sourceRoot, cargoLock, approved: APPROVED });
  requireValue(Array.isArray(crateArchives) && crateArchives.length === external.length,
    'compiler exact original crate capture count');
  const captures = new Map();
  for (const capture of crateArchives) {
    exactKeys(capture, ['identity', 'archive', 'cacheRoot']);
    requireValue(typeof capture.identity === 'string' && !captures.has(capture.identity)
      && Buffer.isBuffer(capture.archive), 'compiler original crate capture identity');
    captures.set(capture.identity, capture);
  }
  requireValue(upstreamLicense === null || Buffer.isBuffer(upstreamLicense), 'compiler upstream license carrier');
  const files = [];
  const cacheProof = [];
  const packages = [];
  for (const { pin, cacheRoot } of external) {
    const name = `${pin.name}-${pin.version}`;
    const capture = captures.get(name);
    requireValue(capture && capture.cacheRoot === cacheRoot, 'compiler metadata/cache correspondence');
    cacheProof.push(validateCompilerCrateCache(capture, pin.crateSha256));
    files.push(...captureCompilerLicenses(pin, capture.archive, upstreamLicense));
    packages.push({ name: pin.name, version: pin.version, crateSha256: pin.crateSha256,
      licenses: pin.files.map(file => ({ path: file.path, bytes: file.size, sha256: file.sha256 })) });
  }
  files.sort((a, b) => a.path < b.path ? -1 : 1);
  orderedPaths(files.map(file => file.path));
  requireValue(files.length <= 505, 'compiler license inventory leaves required archive slots');
  const materials = bytes({ format: 'zryna.playground-compiler-rust-materials.v1', version: 1,
    source: structuredClone(source), target: COMPILER_TARGET, cargoLockSha256: sha256(cargoLock), packages });
  requireValue(materials.length <= 262144, 'compiler material consumer record budget');
  return { materials, files,
    closure: bytes({ ...proof, source: structuredClone(source), materialDescriptorPins: MATERIAL_PINS, cacheProof }) };
}
