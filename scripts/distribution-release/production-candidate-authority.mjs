import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { canonicalBounded, parseCanonical } from './canonical.mjs';
import { observeProtectedGates } from './create-preassembly-gates.mjs';
import { captureProtectedMainSource } from './create-release-qualification-source.mjs';
import { validatePreassemblyGatesShape } from './validate-preassembly-gates.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const WORKFLOW = '.github/workflows/release-production-candidate.yml';
const OBSERVED_REF = 'refs/heads/main';
const INTENDED_REF = 'refs/tags/v0.2.3';
const OBJECT_ID = /^[0-9a-f]{40}$/;
const DIGEST = /^[0-9a-f]{64}$/;

function reject(message) {
  throw new Error(`R406-PRODUCTION-CANDIDATE-AUTHORITY: ${message}`);
}

function exactKeys(value, keys, label) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)
    || Object.keys(value).sort().join('\0') !== [...keys].sort().join('\0')) {
    reject(`${label} fields differ`);
  }
}

export function validateProductionCandidateAuthority(value) {
  canonicalBounded(value);
  exactKeys(value, [
    'format', 'status', 'productionAdmission', 'versionCandidate', 'observedSourceRef',
    'intendedRelease', 'source', 'workflow', 'gates',
  ], 'authority');
  if (value.format !== 'zryna.release-production-candidate-authority.v1'
    || value.status !== 'production-candidate' || value.productionAdmission !== 'forbidden'
    || value.versionCandidate !== '0.2.3' || value.observedSourceRef !== OBSERVED_REF) {
    reject('candidate identity differs');
  }
  exactKeys(value.intendedRelease, ['ref', 'tagProvenance'], 'intended release');
  if (value.intendedRelease.ref !== INTENDED_REF
    || value.intendedRelease.tagProvenance !== 'not-observed') {
    reject('intended release must not claim tag provenance');
  }
  exactKeys(value.source,
    ['repository', 'ref', 'commit', 'tree', 'sourceDateEpoch'], 'source');
  if (value.source.repository !== 'https://github.com/zryna/zryna'
    || value.source.ref !== OBSERVED_REF || !OBJECT_ID.test(value.source.commit)
    || !OBJECT_ID.test(value.source.tree) || value.source.commit === value.source.tree
    || !Number.isInteger(value.source.sourceDateEpoch) || value.source.sourceDateEpoch < 0
    || value.source.sourceDateEpoch > 4294967295) reject('observed source identity differs');
  exactKeys(value.workflow, ['path', 'size', 'sha256'], 'workflow');
  if (value.workflow.path !== WORKFLOW || !Number.isInteger(value.workflow.size)
    || value.workflow.size < 1 || value.workflow.size > 262144
    || !DIGEST.test(value.workflow.sha256)) reject('workflow descriptor differs');
  const gates = validatePreassemblyGatesShape(value.gates);
  if (gates.sourceCommit !== value.source.commit) reject('CI gates and observed source differ');
  return value;
}

export function validateProductionCandidateAuthorityText(text) {
  return validateProductionCandidateAuthority(parseCanonical(text));
}

export async function createProductionCandidateAuthority({
  environment = process.env, cwd = environment.ZRYNA_SOURCE_ROOT, spawn, fetchImpl,
} = {}) {
  if (!environment.GITHUB_TOKEN || environment.GITHUB_SERVER_URL !== 'https://github.com'
    || environment.GITHUB_WORKFLOW_REF !==
      'zryna/zryna/.github/workflows/release-production-candidate.yml@refs/heads/main') {
    reject('exact protected-main candidate workflow and token are required');
  }
  const captured = captureProtectedMainSource({ environment, cwd, spawn, workflow: WORKFLOW });
  const gates = await observeProtectedGates({ environment, fetchImpl });
  return validateProductionCandidateAuthority({
    format: 'zryna.release-production-candidate-authority.v1',
    status: 'production-candidate', productionAdmission: 'forbidden', versionCandidate: '0.2.3',
    observedSourceRef: OBSERVED_REF,
    intendedRelease: { ref: INTENDED_REF, tagProvenance: 'not-observed' },
    ...captured,
    gates: { format: 'zryna.preassembly-gates.v1', ...gates },
  });
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    process.stdout.write(`${canonicalBounded(await createProductionCandidateAuthority())}\n`);
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
