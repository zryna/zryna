// Exact source-candidate descriptor consistency. This supplies no execution, signature or gate authority.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { bytes, parseCanonical, requireValue, sha256 } from './canonical.mjs';

const MATERIAL_SHA = '434e0bcfea3aba59d6500d4eb4a0b9e5a77ac05657febaa07121b5366a837ff8';
const BINDING_SHA = '8f1f2fed9de28e3fa19fec830904ec0f6b4ac68f07eb34b5fd8f76c7796b5ce4';
export const SECURITY_NATIVE_RECIPE_SHA256 = 'bd94d51417811098caf0fda18add45070d12cf499cc8a13d3d9d895b24cd5b49';
function pinned(path, digest) {
  const data = readFileSync(new URL(path, import.meta.url));
  requireValue(sha256(data) === digest, 'security Rust descriptor identity');
  return parseCanonical(data);
}
const MATERIAL = pinned('./materials-rust-security-candidate-v1.json', MATERIAL_SHA);
const BINDING = pinned('./rust-security-binding-v1.json', BINDING_SHA);
export const securityNativeRecipe = pinned('./native-recipe-security-candidate-v1.json',
  SECURITY_NATIVE_RECIPE_SHA256);
requireValue(MATERIAL.format === 'zryna.security-candidate-rust-materials.v1'
  && BINDING.format === 'zryna.security-candidate-rust-material-binding.v1'
  && [MATERIAL, BINDING].every(record => record.status === 'review-candidate'
    && record.productionAdmission === 'forbidden')
  && BINDING.materialRecordSha256 === MATERIAL_SHA
  && BINDING.nativeRecipeSha256 === SECURITY_NATIVE_RECIPE_SHA256
  && securityNativeRecipe.rustMaterialRecords.length === 1
  && securityNativeRecipe.rustMaterialRecords[0].path === 'materials-rust-security-candidate-v1.json'
  && securityNativeRecipe.rustMaterialRecords[0].sha256 === MATERIAL_SHA
  && securityNativeRecipe.cargoLock.sha256 === MATERIAL.cargoLockSha256
  && BINDING.inputs[0].sha256 === MATERIAL.cargoLockSha256,
'security Rust descriptor relationships');

// Descriptor selection for the independent exact Cargo closure audit; no caller authority is minted.
export function securityRustMaterials(target) {
  requireValue(Object.hasOwn(MATERIAL.targets, target), 'security Rust material target');
  const ids = MATERIAL.targets[target];
  const selected = new Set(ids);
  const records = MATERIAL.packages.filter(record => selected.has(`${record.name}-${record.version}`));
  requireValue(selected.size === ids.length && records.length === ids.length
    && new Set(records.map(record => `${record.name}-${record.version}`)).size === records.length,
  'security Rust material census');
  return structuredClone(records);
}

export function matchesSecurityRustInputs(receipt) {
  return Array.isArray(receipt?.inputs) && bytes(receipt.inputs).equals(bytes(BINDING.inputs));
}

// Read each authoritative logical path; reject drift before any compiler/tool process.
export function requireSecurityRustWorkspace(cwd) {
  const inputs = BINDING.inputs.map(({ logicalPath }) => {
    const data = readFileSync(resolve(cwd, logicalPath));
    return { logicalPath, size: data.length, sha256: sha256(data) };
  });
  requireValue(matchesSecurityRustInputs({ inputs }), 'security Rust exact workspace inputs');
}

export function matchesSecurityRustBinding(receipt, distribution, target) {
  return Object.hasOwn(MATERIAL.targets, target)
    && distribution?.version === BINDING.versionCandidate
    && distribution?.source?.repository === BINDING.sourceRepository
    && distribution?.source?.ref === BINDING.sourceRef
    && /^[a-f0-9]{40}$/.test(distribution?.source?.commit ?? '')
    && /^[a-f0-9]{40}$/.test(distribution?.source?.tree ?? '')
    && distribution?.target?.triple === target
    && distribution?.recipe?.format === 'zryna.distribution-recipe.v1'
    && distribution?.recipe?.sha256 === BINDING.recipeSha256
    && ['repository', 'commit', 'tree'].every(key =>
      receipt?.source?.[key] === distribution.source[key])
    && matchesSecurityRustInputs(receipt);
}
