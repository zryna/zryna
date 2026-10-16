// Complete compiler-only archive verification, independent of signature/bootstrap authority.
import { decodeTar } from '../../../../scripts/distribution/archive-tar.mjs';
import { requireArchiveRuntime } from '../../../../scripts/distribution/runtime.mjs';
import { archiveRoot, binaryPath, metadataPaths, requireValue, sha256 } from './compiler-publication-constants.mjs';
import { validateCompilerBuildReceipt, validateCompilerElf, validateCompilerRustMaterials } from './compiler-publication-metadata.mjs';
import { validateCompilerSpdx } from './compiler-publication-sbom.mjs';

export async function verifyCompilerPublicationArchive(input, document) {
  requireArchiveRuntime();
  requireValue(Buffer.isBuffer(input) && input.length === document.archive.bytes &&
    sha256(input) === document.archive.sha256, 'compiler publication archive authentication');
  const files = await decodeTar(input, archiveRoot, document.source.sourceDateEpoch);
  requireValue(files.length === document.files.length, 'compiler publication archive inventory count');
  for (let index = 0; index < files.length; index++) {
    const file = files[index], expected = document.files[index];
    requireValue(file.path === expected.path && file.mode === expected.mode &&
      file.data.length === expected.bytes && sha256(file.data) === expected.sha256,
    'compiler publication complete archive inventory');
  }
  const captured = new Map(files.map(file => [file.path, file]));
  const binary = captured.get(binaryPath)?.data;
  requireValue(binary?.length === document.binary.bytes && sha256(binary) === document.binary.sha256,
    'compiler publication binary bytes');
  validateCompilerElf(binary);
  for (const file of files.filter(file => file.path !== binaryPath)) {
    new TextDecoder('utf-8', { fatal: true }).decode(file.data);
  }
  validateCompilerBuildReceipt(captured.get(metadataPaths.buildReceipt).data, document);
  const materials = validateCompilerRustMaterials(captured.get(metadataPaths.rustMaterials).data, document);
  validateCompilerSpdx(captured.get(metadataPaths.sbom).data, { files, envelope: document, materials });
  return { files, binary, materials };
}
