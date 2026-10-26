// Candidate packaging only. Initial execution must be authenticated independently.
// Node is a build/archive tool here; no Node/bootstrap bytes enter this installation.
import { LIMITS, bytes, exactKeys, parseCanonical, requireValue, sha256 } from './canonical.mjs';
import { encodeTar, decodeTar } from './archive-tar.mjs';
import { encodeZip, decodeZip } from './archive-zip.mjs';
import { verifyCompiledIdentity } from './binary-identity.mjs';
import { authenticateFiles, checksumBytes, targetPaths } from './inventory.mjs';
import { validateMaterials } from './material-record.mjs';
import { validateSourceReceipt } from './source-receipt.mjs';
import { requireArchiveRuntime } from './runtime.mjs';
import {
  NATIVE_FORMAT, nativeRecipe, nativeRecipeFor, nativeRole, validateNativeDistribution,
  validateNativeTuples,
} from './native-inventory-v2.mjs';

import { securityNativeRecipe, matchesSecurityRustInputs } from './rust-security-materials.mjs';

const byPath = (left, right) => left.path < right.path ? -1 : left.path > right.path ? 1 : 0;
const metadataTuple = (path, data) => ({ path, size: data.length, sha256: sha256(data),
  role: 'metadata', mode: 0o644, material: 'source', licenses: ['LICENSE'] });

export function prepareNativeDistribution(identity, payload) {
  const record = { format: NATIVE_FORMAT, ...identity };
  validateNativeDistribution(record);
  authenticateFiles(payload, record.files, record.target.triple);
  const get = path => payload.find(file => file.path === path).data;
  const materialBytes = get('metadata/materials.json');
  validateMaterials(materialBytes, record);
  const source = validateSourceReceipt(get('metadata/architecture-receipt.json'), record);
  const selectedRecipe = nativeRecipeFor(record.recipe);
  const lock = source.inputs.find(file => file.logicalPath === 'Cargo.lock');
  requireValue(lock?.size === selectedRecipe.cargoLock.bytes
    && lock.sha256 === selectedRecipe.cargoLock.sha256, 'native supplier Cargo.lock binding');
  requireValue(selectedRecipe !== securityNativeRecipe || matchesSecurityRustInputs(source),
    'native security supplier exact source inputs');
  requireValue(get('VERSION').equals(Buffer.from(`${record.version}\n`)), 'native VERSION mismatch');
  const distribution = bytes(record);
  requireValue(distribution.length <= LIMITS.record, 'native manifest byte budget');
  return { record, distribution, payload };
}

export function nativeInstallationDocuments(version, target) {
  targetPaths(target);
  requireValue(version === nativeRecipe.compilerVersion, 'native document version');
  const documents = {
    'NOTICE': 'Zryna native source-candidate installation. Rust dependency notices are retained under licenses/rust/.\n',
    'README.md': `# Zryna native ${version}\n\nThis source candidate uses the embedded native frontend. Compilation requires no Node.js or pnpm.\nJavaScript/WebAssembly execution requires a separately selected runtime; native linking retains its supported SDK requirements.\n`,
    'SUPPORT.md': 'Source-candidate installation for qualified Linux x86-64 and Windows x64 hosts.\nThis candidate is not an authenticated outer production release. Verify its independently selected source and complete archive before execution.\n',
    'VERSION': `${version}\n`,
  };
  return Object.entries(documents).map(([path, data]) => ({ path, mode: 0o644, data: Buffer.from(data) }));
}

export function prepareNativePayload(identity, capturedMaterials, architectureReceipt) {
  const target = identity.target.triple;
  const documents = nativeInstallationDocuments(identity.version, target);
  for (const file of capturedMaterials) {
    requireValue(Buffer.isBuffer(file.data) && (file.path === 'LICENSE'
      || file.path.startsWith('licenses/rust/')), 'unexpected native captured material');
  }
  const payload = [...capturedMaterials, ...documents].sort(byPath);
  const files = payload.map(file => {
    const role = nativeRole(file.path, target);
    return { path: file.path, size: file.data.length, sha256: sha256(file.data), role,
      mode: file.mode, material: file.path.startsWith('licenses/rust/')
        ? sha256(Buffer.from(file.path.split('/')[2], 'ascii')) : 'source',
      licenses: role === 'license' ? [file.path] : ['LICENSE'] };
  });
  validateNativeTuples(files, target);
  const metadata = [
    { path: 'metadata/architecture-receipt.json', mode: 0o644, data: architectureReceipt },
    { path: 'metadata/materials.json', mode: 0o644,
      data: bytes({ format: 'zryna.distribution-materials.v1', files }) },
  ];
  payload.push(...metadata); payload.sort(byPath);
  files.push(...metadata.map(file => metadataTuple(file.path, file.data))); files.sort(byPath);
  return prepareNativeDistribution({ ...identity, files }, payload);
}

export async function assembleNativeInstallation(prepared, cli) {
  requireArchiveRuntime();
  const checked = prepareNativeDistribution(prepared.record, prepared.payload);
  requireValue(checked.distribution.equals(prepared.distribution), 'native prepared bytes changed');
  const record = checked.record, paths = targetPaths(record.target.triple);
  requireValue(Buffer.isBuffer(cli) && cli.length <= LIMITS.binary, 'native CLI byte budget');
  verifyCompiledIdentity(cli, sha256(checked.distribution), record.target.triple);
  const licenses = record.files.filter(file => file.role === 'license')
    .map(file => file.path);
  const entries = [...record.files, { path: paths.cli, size: cli.length, sha256: sha256(cli),
    role: 'cli', mode: 0o755, material: 'source', licenses },
  metadataTuple('metadata/distribution.json', checked.distribution)].sort(byPath);
  const files = [...checked.payload, { path: paths.cli, mode: 0o755, data: cli },
    { path: 'metadata/distribution.json', mode: 0o644, data: checked.distribution }].sort(byPath);
  validateNativeTuples(entries, record.target.triple);
  authenticateFiles(files, entries, record.target.triple);
  const inventory = bytes({ format: 'zryna.distribution-inventory.v1', files: entries });
  requireValue(inventory.length <= LIMITS.record, 'native inventory byte budget');
  files.push({ path: 'metadata/inventory.json', mode: 0o644, data: inventory }); files.sort(byPath);
  files.push({ path: 'metadata/checksums.sha256', mode: 0o644, data: checksumBytes(files) }); files.sort(byPath);
  requireValue(files.length === record.files.length + 4, 'native complete archive count');
  const root = `zryna-native-${record.version}-${record.target.triple}`;
  const archive = paths.format === 'zip' ? encodeZip(root, files)
    : await encodeTar(root, files, record.source.sourceDateEpoch);
  const subject = { format: 'zryna.native-distribution-subject.v2', version: record.version,
    source: record.source, target: record.target, recipe: record.recipe,
    filename: `${root}.${paths.extension}`, size: archive.length, sha256: sha256(archive) };
  return { archive, subject, files };
}

export async function verifyNativeArchive(archive, expected) {
  requireArchiveRuntime();
  exactKeys(expected, ['format', 'version', 'source', 'target', 'recipe', 'filename', 'size', 'sha256']);
  requireValue(expected.format === 'zryna.native-distribution-subject.v2'
    && Buffer.isBuffer(archive) && archive.length <= LIMITS.archive && archive.length === expected.size
    && sha256(archive) === expected.sha256, 'native archive subject mismatch');
  const paths = targetPaths(expected.target.triple);
  const root = `zryna-native-${expected.version}-${expected.target.triple}`;
  requireValue(expected.filename === `${root}.${paths.extension}`, 'native archive filename');
  const files = paths.format === 'zip' ? decodeZip(archive, root, [paths.cli])
    : await decodeTar(archive, root, expected.source.sourceDateEpoch);
  const get = path => { const file = files.find(item => item.path === path);
    requireValue(file, 'missing native archive file'); return file; };
  const recordBytes = get('metadata/distribution.json').data;
  const record = parseCanonical(recordBytes);
  for (const key of ['version', 'source', 'target', 'recipe']) {
    requireValue(bytes(record[key]).equals(bytes(expected[key])), 'native archive identity');
  }
  const excluded = [paths.cli, 'metadata/distribution.json', 'metadata/inventory.json', 'metadata/checksums.sha256'];
  const prepared = prepareNativeDistribution(record, files.filter(file => !excluded.includes(file.path)));
  requireValue(prepared.distribution.equals(recordBytes), 'native archive prepared bytes');
  const inventory = parseCanonical(get('metadata/inventory.json').data);
  exactKeys(inventory, ['format', 'files']);
  requireValue(inventory.format === 'zryna.distribution-inventory.v1', 'native inventory format');
  const indexed = files.filter(file => !['metadata/inventory.json', 'metadata/checksums.sha256'].includes(file.path));
  validateNativeTuples(inventory.files, record.target.triple);
  authenticateFiles(indexed, inventory.files, record.target.triple);
  requireValue(bytes(inventory.files.filter(file => ![paths.cli, 'metadata/distribution.json'].includes(file.path)))
    .equals(bytes(record.files)), 'native inventory payload differs');
  const cli = inventory.files.find(file => file.path === paths.cli);
  requireValue(cli?.material === 'source' && bytes(cli.licenses).equals(bytes(record.files
    .filter(file => file.role === 'license').map(file => file.path))), 'native CLI license coverage');
  const manifest = inventory.files.find(file => file.path === 'metadata/distribution.json');
  requireValue(manifest?.material === 'source' && bytes(manifest.licenses).equals(bytes(['LICENSE'])),
    'native manifest origin');
  verifyCompiledIdentity(get(paths.cli).data, sha256(recordBytes), record.target.triple);
  requireValue(files.length === record.files.length + 4
    && checksumBytes(files.filter(file => file.path !== 'metadata/checksums.sha256'))
      .equals(get('metadata/checksums.sha256').data), 'native complete checksum coverage');
  for (const path of ['metadata/inventory.json', 'metadata/checksums.sha256']) {
    requireValue(get(path).mode === 0o644, 'native metadata mode');
  }
  return { files, distribution: record, archiveSha256: sha256(archive) };
}
