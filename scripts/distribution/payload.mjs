import { bytes, orderedPaths, requireValue, sha256 } from './canonical.mjs';
import { installationDocuments } from './documents.mjs';
import { roleFor, targetPaths, validateTuples } from './inventory.mjs';
import { prepare } from './prepare.mjs';

function material(path, target) {
  if (path === targetPaths(target).node || path === 'licenses/node-LICENSE') return 'node-22.22.1';
  // A full package-identity hash preserves versions containing '+' within the 64-byte ID bound.
  if (path.startsWith('licenses/rust/')) return sha256(Buffer.from(path.split('/')[2], 'ascii'));
  if (path.includes('/@typescript/typescript6/') || path === 'licenses/typescript6-LICENSE.txt') {
    return 'typescript6-6.0.2';
  }
  if (path.includes('/@typescript/old/') || path.startsWith('licenses/typescript-')) {
    return 'typescript-6.0.3';
  }
  return 'source';
}

function licenses(path, role, origin) {
  if (role === 'license') return [path];
  if (origin === 'node-22.22.1') return ['licenses/node-LICENSE'];
  if (origin === 'typescript6-6.0.2') return ['licenses/typescript6-LICENSE.txt'];
  if (origin === 'typescript-6.0.3') {
    return ['licenses/typescript-LICENSE.txt', 'licenses/typescript-ThirdPartyNoticeText.txt'];
  }
  return ['LICENSE'];
}

function tuple(file, target) {
  requireValue(Buffer.isBuffer(file.data), 'captured payload bytes');
  const role = roleFor(file.path, target);
  const origin = material(file.path, target);
  return { path: file.path, size: file.data.length, sha256: sha256(file.data), role,
    mode: file.mode, material: origin, licenses: licenses(file.path, role, origin) };
}

// Captured bytes must already come from authenticated source objects and material archives.
// Constructing tuples here establishes neither acquisition provenance nor publication authority.
export function preparePayload(identity, capturedMaterials, architectureReceipt) {
  orderedPaths(capturedMaterials.map(file => file.path));
  const target = identity.target.triple;
  const generated = installationDocuments(identity.version, target);
  for (const file of capturedMaterials) {
    requireValue(!file.path.startsWith('metadata/') && file.path !== targetPaths(target).cli
      && !generated.some(document => document.path === file.path), 'unexpected captured payload');
  }
  const payload = [...capturedMaterials, ...generated]
    .sort((left, right) => left.path < right.path ? -1 : left.path > right.path ? 1 : 0);
  const files = payload.map(file => tuple(file, target));
  validateTuples(files, target);
  const metadata = [
    { path: 'metadata/architecture-receipt.json', mode: 0o644, data: architectureReceipt },
    { path: 'metadata/materials.json', mode: 0o644,
      data: bytes({ format: 'zryna.distribution-materials.v1', files }) },
  ];
  payload.push(...metadata);
  files.push(...metadata.map(file => tuple(file, target)));
  payload.sort((left, right) => left.path < right.path ? -1 : left.path > right.path ? 1 : 0);
  files.sort((left, right) => left.path < right.path ? -1 : left.path > right.path ? 1 : 0);
  return { ...prepare({ ...identity, files }, payload), payload };
}
