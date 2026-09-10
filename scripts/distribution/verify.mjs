import { bytes, exactKeys, parseCanonical, requireValue, sha256 } from './canonical.mjs';
import { decodeTar } from './archive-tar.mjs';
import { decodeZip } from './archive-zip.mjs';
import { verifyCompiledIdentity } from './binary-identity.mjs';
import { authenticateFiles, checksumBytes, targetPaths } from './inventory.mjs';
import { prepare } from './prepare.mjs';
import { requireArchiveRuntime } from './runtime.mjs';

// The caller obtains expected identities from an independently authenticated release subject.
// This content verifier does not authenticate a signature or trust policy supplied beside bytes.
export async function verifyArchive(archive, expected) {
  requireArchiveRuntime();
  requireValue(Buffer.isBuffer(archive) && archive.length === expected.size
    && sha256(archive) === expected.sha256, 'archive differs from expected release subject');
  const paths = targetPaths(expected.target.triple);
  const root = `zryna-${expected.version}-${expected.target.triple}`;
  requireValue(expected.filename === `${root}.${paths.extension}`, 'archive subject filename');
  const files = paths.format === 'zip'
    ? decodeZip(archive, root, [paths.cli, paths.node])
    : await decodeTar(archive, root, expected.source.sourceDateEpoch);
  const byPath = new Map(files.map(file => [file.path, file]));
  const get = path => {
    requireValue(byPath.has(path), 'missing archive metadata');
    return byPath.get(path);
  };
  const distributionBytes = get('metadata/distribution.json').data;
  const distribution = parseCanonical(distributionBytes);
  for (const key of ['version', 'source', 'target', 'recipe']) {
    requireValue(bytes(distribution[key]).equals(bytes(expected[key])), 'archive release identity mismatch');
  }
  const excluded = new Set([paths.cli, 'metadata/distribution.json', 'metadata/inventory.json',
    'metadata/checksums.sha256']);
  const payload = files.filter(file => !excluded.has(file.path));
  const prepared = prepare(distribution, payload);
  requireValue(prepared.distribution.equals(distributionBytes), 'archive prepared record mismatch');
  requireValue(files.length === prepared.preparedDistribution.archiveFileCount, 'archive file count mismatch');
  const inventory = parseCanonical(get('metadata/inventory.json').data);
  exactKeys(inventory, ['format', 'files']);
  requireValue(inventory.format === 'zryna.distribution-inventory.v1', 'inventory format');
  const indexed = files.filter(file => !['metadata/inventory.json', 'metadata/checksums.sha256'].includes(file.path));
  authenticateFiles(indexed, inventory.files, expected.target.triple);
  const expectedPayload = inventory.files.filter(file => ![paths.cli, 'metadata/distribution.json'].includes(file.path));
  requireValue(bytes(expectedPayload).equals(bytes(distribution.files)), 'inventory payload differs from prepared record');
  const metadata = inventory.files.find(file => file.path === 'metadata/distribution.json');
  requireValue(metadata?.material === 'source' && bytes(metadata.licenses).equals(bytes(['LICENSE'])),
    'distribution metadata origin or license');
  const cli = inventory.files.find(file => file.path === paths.cli);
  const licenses = distribution.files.filter(file => file.role === 'license'
    && (file.path === 'LICENSE' || file.path.startsWith('licenses/rust/'))).map(file => file.path);
  requireValue(cli?.material === 'source' && bytes(cli.licenses).equals(bytes(licenses)), 'CLI license coverage');
  verifyCompiledIdentity(get(paths.cli).data, sha256(distributionBytes), expected.target.triple);
  const checksummed = files.filter(file => file.path !== 'metadata/checksums.sha256');
  requireValue(checksumBytes(checksummed).equals(get('metadata/checksums.sha256').data), 'checksum coverage mismatch');
  requireValue(get('metadata/inventory.json').mode === 0o644 && get('metadata/checksums.sha256').mode === 0o644,
    'unindexed metadata mode');
  return { files, distribution, archiveSha256: sha256(archive) };
}
