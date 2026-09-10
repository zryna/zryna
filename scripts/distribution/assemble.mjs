import { bytes, parseCanonical, requireValue, sha256 } from './canonical.mjs';
import { encodeTar } from './archive-tar.mjs';
import { encodeZip } from './archive-zip.mjs';
import { verifyCompiledIdentity } from './binary-identity.mjs';
import { authenticateFiles, checksumBytes, inventoryBytes, targetPaths } from './inventory.mjs';
import { prepare } from './prepare.mjs';

function descriptorMatches(descriptor, data) {
  requireValue(Buffer.isBuffer(data) && descriptor.size === data.length
    && descriptor.sha256 === sha256(data), 'build input artifact mismatch');
}

function metadataTuple(path, data) {
  return { path, size: data.length, sha256: sha256(data), role: 'metadata', mode: 0o644,
    material: 'source', licenses: ['LICENSE'] };
}

export async function assemble(inputBytes, captured) {
  // The separate release boundary owns this schema and protected input admission.
  const { validateBuildInput } = await import('../distribution-release/validate-build-input.mjs');
  const input = validateBuildInput(parseCanonical(inputBytes));
  const distribution = parseCanonical(captured.distribution);
  const prepared = prepare(distribution, captured.payload);
  requireValue(prepared.distribution.equals(captured.distribution), 'prepared record mismatch');
  for (const key of ['version', 'source', 'target', 'recipe']) {
    requireValue(bytes(input[key]).equals(bytes(distribution[key])), 'assembly identity mismatch');
  }
  requireValue(bytes(input.preparedDistribution).equals(bytes(prepared.preparedDistribution))
    && bytes(input.materials).equals(bytes(prepared.materials)), 'prepared descriptor mismatch');
  descriptorMatches(input.compiledCli, captured.cli);
  descriptorMatches(input.gateReceipt, captured.gates);
  const gates = parseCanonical(captured.gates);
  const { validatePreassemblyGates } = await import('../distribution-release/validate-preassembly-gates.mjs');
  validatePreassemblyGates(gates, input);
  const architecture = captured.payload.find(file => file.path === 'metadata/architecture-receipt.json');
  descriptorMatches(input.architectureReceipt, architecture.data);
  const { validateSourceBuildReceiptText } = await import(
    '../distribution-release/validate-source-build-receipt.mjs');
  validateSourceBuildReceiptText(architecture.data.toString('utf8'), input);
  verifyCompiledIdentity(captured.cli, input.preparedDistribution.sha256, input.target.triple);
  const paths = targetPaths(input.target.triple);
  const cliTuple = { path: paths.cli, size: captured.cli.length, sha256: sha256(captured.cli),
    role: 'cli', mode: 0o755, material: 'source',
    licenses: distribution.files.filter(file => file.role === 'license'
      && (file.path === 'LICENSE' || file.path.startsWith('licenses/rust/'))).map(file => file.path) };
  const entries = [...distribution.files, cliTuple,
    metadataTuple('metadata/distribution.json', captured.distribution)]
    .sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  const files = [...captured.payload, { path: paths.cli, mode: 0o755, data: captured.cli },
    { path: 'metadata/distribution.json', mode: 0o644, data: captured.distribution }]
    .sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  authenticateFiles(files, entries, input.target.triple);
  const inventory = inventoryBytes(entries, input.target.triple);
  files.push({ path: 'metadata/inventory.json', mode: 0o644, data: inventory });
  files.sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  const checksums = checksumBytes(files);
  files.push({ path: 'metadata/checksums.sha256', mode: 0o644, data: checksums });
  files.sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  requireValue(files.length === prepared.preparedDistribution.archiveFileCount, 'archive file count');
  const root = `zryna-${input.version}-${input.target.triple}`;
  const archive = paths.format === 'zip' ? encodeZip(root, files)
    : await encodeTar(root, files, input.source.sourceDateEpoch);
  const receipt = bytes({
    format: 'zryna.distribution-build-receipt.v1',
    inputSha256: sha256(inputBytes),
    archive: { filename: `${root}.${paths.extension}`, size: archive.length, sha256: sha256(archive) },
    metadata: files.filter(file => file.path.startsWith('metadata/')).map(file => ({
      path: file.path, size: file.data.length, sha256: sha256(file.data),
    })),
    inventorySha256: sha256(inventory),
    recipe: input.recipe,
    target: input.target,
  });
  return { archive, receipt, filename: `${root}.${paths.extension}`, files };
}
