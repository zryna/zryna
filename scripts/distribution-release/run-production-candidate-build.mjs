import { isAbsolute, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { ACCEPTED_RECIPE_SHA256 } from './check-release-readiness.mjs';
import { canonicalBounded } from './canonical.mjs';
import { createProductionCandidateArchitectureReceipt } from './create-source-build-receipt.mjs';
import {
  validateProductionCandidateAuthorityText,
} from './production-candidate-authority.mjs';
import {
  defaultProductionAdapters, runAuthenticatedProductionBuild,
} from './run-protected-build.mjs';
import { exactReleaseNames, MAX_RELEASE_DOCUMENT, readReleaseFile } from './release-files.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);

function reject(message) {
  throw new Error(`R406-PRODUCTION-CANDIDATE-BUILD: ${message}`);
}

export async function runProductionCandidateBuild({
  admissionRoot, outputRoot, sourceRoot, target, replica,
  environment = process.env, spawn, acceptedRecipeSha256 = ACCEPTED_RECIPE_SHA256, adapters,
}) {
  if (![admissionRoot, outputRoot, sourceRoot].every((path) => isAbsolute(path ?? '')
    && resolve(path) === path) || new Set([admissionRoot, outputRoot, sourceRoot]).size !== 3
    || environment.GITHUB_EVENT_NAME !== 'workflow_dispatch'
    || environment.GITHUB_REPOSITORY !== 'zryna/zryna'
    || environment.GITHUB_REF !== 'refs/heads/main'
    || environment.GITHUB_REF_TYPE !== 'branch' || environment.GITHUB_REF_NAME !== 'main'
    || environment.GITHUB_REF_PROTECTED !== 'true'
    || environment.GITHUB_WORKFLOW_REF !==
      'zryna/zryna/.github/workflows/release-production-candidate.yml@refs/heads/main') {
    reject('exact protected-main candidate build context is required');
  }
  exactReleaseNames(admissionRoot, ['candidate-authority.json']);
  const authorityBytes = readReleaseFile(
    admissionRoot, 'candidate-authority.json', MAX_RELEASE_DOCUMENT,
  );
  const authority = validateProductionCandidateAuthorityText(authorityBytes.toString('utf8'));
  if (authority.source.commit !== environment.GITHUB_SHA
    || authority.source.ref !== environment.GITHUB_REF) reject('observed candidate source differs');
  const implementation = adapters ?? {
    ...await defaultProductionAdapters(),
    createArchitectureReceipt: (options) => Buffer.from(
      `${canonicalBounded(createProductionCandidateArchitectureReceipt(options))}\n`,
    ),
  };
  return runAuthenticatedProductionBuild({
    outputRoot, sourceRoot, target, replica, source: authority.source,
    gatesBytes: Buffer.from(`${canonicalBounded(authority.gates)}\n`), gates: authority.gates,
    resultIdentity: {
      format: 'zryna.production-candidate-build-result.v1',
      status: 'production-candidate', productionAdmission: 'forbidden',
      observedSource: authority.source, intendedRelease: authority.intendedRelease,
    },
    manifestName: 'production-candidate-build-result.json', environment, spawn,
    acceptedRecipeSha256, adapters: implementation, productionCandidate: true,
  });
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    if (process.argv.length !== 6 || process.argv[2] !== '--admission'
      || process.argv[4] !== '--output') reject('expected --admission and --output once');
    await runProductionCandidateBuild({
      admissionRoot: resolve(process.argv[3]), outputRoot: resolve(process.argv[5]),
      sourceRoot: resolve(process.env.ZRYNA_SOURCE_ROOT ?? ''),
      target: process.env.ZRYNA_TARGET, replica: Number(process.env.ZRYNA_REPLICA),
    });
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
