import { mkdirSync } from 'node:fs';
import { isAbsolute, resolve } from 'node:path';
import { canonicalBounded, sha256 } from './canonical.mjs';
import { createReleaseFile, exactReleaseNames } from './release-files.mjs';
import { validateReleaseQualificationInputText } from './validate-release-qualification-input.mjs';
import { validateReleaseQualificationInspectionText } from './validate-release-qualification-inspection.mjs';
import { validateQualificationResult } from './compare-release-qualifications.mjs';

function reject(message) {
  throw new Error(`R406-QUALIFICATION-RESULT: ${message}`);
}

function artifact(path, bytes) {
  if (!Buffer.isBuffer(bytes) || bytes.length < 1) reject(`${path} bytes are missing`);
  return { path, size: bytes.length, sha256: sha256(bytes) };
}

export function writeReleaseQualificationResult({
  outputRoot, replica, binding, cli, assembled, inspection,
}) {
  if (!isAbsolute(outputRoot) || resolve(outputRoot) !== outputRoot || ![1, 2].includes(replica)) {
    reject('exact absolute output root and replica are required');
  }
  if (!Buffer.isBuffer(binding) || !Buffer.isBuffer(cli) || !Buffer.isBuffer(inspection)
    || !Buffer.isBuffer(assembled?.archive) || !Buffer.isBuffer(assembled?.inventory)) {
    reject('qualification result bytes are incomplete');
  }
  const input = validateReleaseQualificationInputText(binding.toString('utf8'));
  validateReleaseQualificationInspectionText(inspection.toString('utf8'), input, cli);
  const extension = input.target.triple === 'x86_64-pc-windows-msvc' ? 'exe' : 'elf';
  const names = {
    binding: `${input.archive.root}.input.json`,
    cli: `${input.archive.root}.${extension}`,
    inventory: `${input.archive.root}.inventory.json`,
    archive: assembled.filename,
    inspection: `${input.archive.root}.inspection.json`,
  };
  if (names.archive !== `${input.archive.root}.${input.archive.format === 'zip' ? 'zip' : 'tar.gz'}`) {
    reject('qualification archive filename differs');
  }
  const bytes = {
    binding, cli, inventory: assembled.inventory, archive: assembled.archive, inspection,
  };
  const result = validateQualificationResult({
    format: 'zryna.release-qualification-result.v1',
    status: 'qualification-only',
    productionAdmission: 'forbidden',
    versionCandidate: '0.2.0',
    target: input.target.triple,
    replica,
    source: {
      ref: input.source.ref, commit: input.source.commit, tree: input.source.tree,
      sourceDateEpoch: input.source.sourceDateEpoch,
    },
    recipeProposal: {
      size: input.recipeProposal.size, sha256: input.recipeProposal.sha256,
    },
    artifacts: Object.fromEntries(Object.entries(names).map(([key, path]) => [
      key, artifact(path, bytes[key]),
    ])),
  }, input.target.triple, replica);
  mkdirSync(outputRoot, { recursive: false, mode: 0o700 });
  for (const [key, path] of Object.entries(names)) createReleaseFile(outputRoot, path, bytes[key]);
  createReleaseFile(outputRoot, 'qualification-result.json',
    Buffer.from(`${canonicalBounded(result)}\n`));
  exactReleaseNames(outputRoot, ['qualification-result.json', ...Object.values(names)]);
  return result;
}
