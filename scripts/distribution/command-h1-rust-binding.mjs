import { readFileSync } from 'node:fs';
import { bytes, parseCanonical, requireValue, sha256 } from './canonical.mjs';

const INPUT = readFileSync(new URL('./command-h1-rust-binding-v1.json', import.meta.url));
requireValue(sha256(INPUT) === '8fff5058dff2e414b8d60d79f01af25b71d8337a269f72924d8e33a92a3617b3',
  'command H1 Rust binding identity');
const BINDING = parseCanonical(INPUT);
requireValue(BINDING.format === 'zryna.command-h1-rust-material-binding.v1'
  && BINDING.status === 'review-candidate' && BINDING.productionAdmission === 'forbidden',
  'command H1 Rust binding status');

// This exact content binding is available only to the existing main-candidate context.
// It establishes no protected-main, gate, signature, recipe-execution or release authority.
export function matchesCommandH1RustBinding(
  receipt, distribution, target, materialRecordSha256, candidateMaterialRecordSha256,
) {
  if (!/^[0-9a-f]{64}$/.test(candidateMaterialRecordSha256 ?? '')
    || distribution?.version !== BINDING.versionCandidate
    || distribution?.source?.repository !== BINDING.sourceRepository
    || distribution?.source?.ref !== BINDING.sourceRef
    || distribution?.target?.triple !== target
    || distribution?.recipe?.format !== 'zryna.distribution-recipe.v1'
    || distribution?.recipe?.sha256 !== BINDING.recipeSha256
    || materialRecordSha256 !== BINDING.materialRecordSha256
    || candidateMaterialRecordSha256 !== BINDING.candidateMaterialRecordSha256
    || !['repository', 'commit', 'tree'].every(key =>
      receipt?.source?.[key] === distribution.source[key])
    || !Array.isArray(receipt?.inputs)) return false;
  return bytes(receipt.inputs).equals(bytes(BINDING.inputs));
}
