// Deterministic unsigned compiler-only bytes. Returned digests require independent review/signing.
import { encodeTar } from '../../../../scripts/distribution/archive-tar.mjs';
import { orderedPaths } from '../../../../scripts/distribution/canonical.mjs';
import { requireArchiveRuntime } from '../../../../scripts/distribution/runtime.mjs';
import { LIMITS, archivePath, archiveRoot, binaryPath, bytes, certificateIdentity, exactKeys,
  issuer, metadataPaths, product, productVersion, profile, requireValue, sha256, target,
  validateSource } from './compiler-publication-constants.mjs';
import { compilerBuildReceipt, validateCompilerRustMaterials } from './compiler-publication-metadata.mjs';
import { createCompilerSpdx } from './compiler-publication-sbom.mjs';
import { validateCompilerPublicationEnvelope } from './compiler-publication-schema.mjs';
import { verifyCompilerPublicationArchive } from './compiler-publication-archive.mjs';

const descriptor = (path, data) => ({ path, bytes: data.length, sha256: sha256(data) });
const sorted = files => files.sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);

export async function createUnsignedCompilerPublication(input) {
  exactKeys(input, ['source', 'files', 'materials', 'provenance']);
  requireArchiveRuntime();
  validateSource(input.source);
  requireValue(Array.isArray(input.files) && input.files.length > 0 && input.files.length <= LIMITS.files - 3,
    'compiler publication unsigned input count');
  let total = 0;
  const files = input.files.map(file => {
    exactKeys(file, ['path', 'data', 'mode']);
    requireValue(Buffer.isBuffer(file.data) && file.data.length > 0 && file.data.length <=
      (file.path === binaryPath ? LIMITS.binary : LIMITS.text) &&
      !Object.values(metadataPaths).includes(file.path), 'compiler publication unsigned payload');
    total += file.data.length;
    requireValue(total <= LIMITS.expanded, 'compiler publication unsigned byte budget');
    return { path: file.path, mode: file.mode, data: Buffer.from(file.data) };
  });
  sorted(files); orderedPaths(files.map(file => file.path));
  const binary = files.find(file => file.path === binaryPath);
  requireValue(binary !== undefined, 'compiler publication unsigned binary missing');
  const source = structuredClone(input.source), provenance = structuredClone(input.provenance);
  exactKeys(provenance, ['workflow', 'toolchain', 'inputs', 'build', 'requiredCi', 'reproduction']);
  const document = { format: 'zryna.playground-compiler-release.v1', version: 1,
    product, productVersion, target, profile, source, signing: { issuer, certificateIdentity },
    archive: null, binary: descriptor(binaryPath, binary.data), provenance, files: [] };
  requireValue(input.materials instanceof Uint8Array && input.materials.length > 0 &&
    input.materials.length <= LIMITS.record, 'compiler publication unsigned material record');
  const materialBytes = Buffer.from(input.materials);
  const materials = validateCompilerRustMaterials(materialBytes, document);
  const buildBytes = bytes(compilerBuildReceipt(document));
  provenance.buildReceipt = descriptor(metadataPaths.buildReceipt, buildBytes);
  provenance.rustMaterials = descriptor(metadataPaths.rustMaterials, materialBytes);
  files.push({ path: metadataPaths.buildReceipt, mode: 0o644, data: buildBytes },
    { path: metadataPaths.rustMaterials, mode: 0o644, data: materialBytes });
  const spdx = createCompilerSpdx({ files, envelope: document, materials });
  provenance.sbom = descriptor(metadataPaths.sbom, spdx);
  files.push({ path: metadataPaths.sbom, mode: 0o644, data: spdx });
  sorted(files);
  document.files = files.map(file => ({ ...descriptor(file.path, file.data), mode: file.mode }));
  const archive = await encodeTar(archiveRoot, files, source.sourceDateEpoch);
  document.archive = descriptor(archivePath, archive);
  const envelope = bytes(document);
  const candidatePolicy = { version: 1, source, envelopeSha256: sha256(envelope),
    archive: document.archive, binary: document.binary, provenanceSha256: sha256(bytes(provenance)) };
  validateCompilerPublicationEnvelope(envelope, candidatePolicy);
  await verifyCompilerPublicationArchive(archive, document);
  return { status: 'unsigned-candidate-not-authenticated', envelope, archive,
    candidatePolicy: structuredClone(candidatePolicy), document: structuredClone(document) };
}
