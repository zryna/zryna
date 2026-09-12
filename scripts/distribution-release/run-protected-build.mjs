import { spawnSync } from 'node:child_process';
import { mkdirSync } from 'node:fs';
import { isAbsolute, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { ACCEPTED_RECIPE_SHA256 } from './check-release-readiness.mjs';
import { canonical, canonicalBounded, parseCanonical, sha256 } from './canonical.mjs';
import { createSourceBuildReceipt } from './create-source-build-receipt.mjs';
import {
  createReleaseFile, exactReleaseNames, MAX_RELEASE_DOCUMENT, readReleaseFile,
} from './release-files.mjs';
import { validateBuildInput } from './validate-build-input.mjs';
import { validatePreassemblyGatesShape } from './validate-preassembly-gates.mjs';
import { validateReleaseTagReceiptText } from './validate-release-tag-receipt.mjs';
import { releaseSpdxBytes, validateReleaseSpdx } from './release-sbom.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const RECIPE_PATH = 'scripts/distribution/release-recipe-v1.json';
const MAX_GIT_OUTPUT = 16 * 1024 * 1024;
const TARGETS = Object.freeze({
  'x86_64-unknown-linux-gnu': {
    archiveFormat: 'tar-gzip', extension: 'tar.gz', cli: 'zryna',
    platformBaseline: { os: 'linux', distribution: 'ubuntu', version: '24.04', architecture: 'x86_64' },
  },
  'x86_64-pc-windows-msvc': {
    archiveFormat: 'zip', extension: 'zip', cli: 'zryna.exe',
    platformBaseline: { os: 'windows', product: 'windows-server', version: '2022',
      architecture: 'x86_64', runtime: 'operating-system-ucrt' },
  },
});

function reject(message) {
  throw new Error(`R406-PROTECTED-BUILD: ${message}`);
}

function bytes(value, label) {
  if (!Buffer.isBuffer(value) || value.length < 1) reject(`${label} must be non-empty bytes`);
  return value;
}

function artifact(logicalPath, value) {
  bytes(value, logicalPath);
  return { logicalPath, size: value.length, sha256: sha256(value) };
}

function runGit(spawn, args, cwd, encoding = null) {
  const result = spawn('git', args, {
    cwd, encoding, maxBuffer: MAX_GIT_OUTPUT + 1, shell: false, windowsHide: true,
  });
  if (result.error || result.status !== 0) reject(`git ${args[0]} failed`);
  const output = Buffer.isBuffer(result.stdout) ? result.stdout : Buffer.from(result.stdout, 'utf8');
  if (output.length > MAX_GIT_OUTPUT) reject(`git ${args[0]} output exceeds its byte bound`);
  return encoding === null ? output : output.toString(encoding);
}

function gitBlob(spawn, sourceRoot, commit, path) {
  const listing = runGit(spawn, ['ls-tree', '-z', '--full-tree', commit, '--', path], sourceRoot);
  const suffix = Buffer.from(`\t${path}\0`);
  if (!listing.subarray(-suffix.length).equals(suffix) || listing.indexOf(0) !== listing.length - 1) {
    reject('recipe tree entry is missing or ambiguous');
  }
  const match = /^(100644) (blob) ([0-9a-f]{40})$/
    .exec(listing.subarray(0, listing.length - suffix.length).toString('ascii'));
  if (!match) reject('recipe must be one ordinary non-executable Git blob');
  return runGit(spawn, ['cat-file', 'blob', match[3]], sourceRoot);
}

function validateRecipe(recipeBytes, acceptedDigest) {
  if (!/^[0-9a-f]{64}$/.test(acceptedDigest ?? '') || sha256(recipeBytes) !== acceptedDigest) {
    reject('recipe bytes differ from the independently accepted digest');
  }
  const value = parseCanonical(new TextDecoder('utf-8', { fatal: true }).decode(recipeBytes));
  if (value?.format !== 'zryna.distribution-recipe.v1') reject('recipe format differs');
  return { value, reference: { format: value.format, sha256: acceptedDigest } };
}

async function defaultAdapters() {
  const [{ preparePayload }, { assemble }, { verifyArchive }, provisioner] = await Promise.all([
    import('../distribution/payload.mjs'),
    import('../distribution/assemble.mjs'),
    import('../distribution/verify.mjs'),
    import('../distribution/provision-release.mjs'),
  ]);
  for (const name of ['captureReleaseMaterials', 'compileReleaseCli']) {
    if (typeof provisioner[name] !== 'function') reject(`#422 provisioner must export ${name}`);
  }
  return {
    preparePayload, assemble, verifyArchive,
    captureReleaseMaterials: provisioner.captureReleaseMaterials,
    compileReleaseCli: provisioner.compileReleaseCli,
    createReleaseSbom: releaseSpdxBytes,
    createArchitectureReceipt: (options) => Buffer.from(
      `${canonicalBounded(createSourceBuildReceipt(options))}\n`,
    ),
  };
}

function gateDescriptor(gatesBytes, gates) {
  return {
    format: gates.format,
    logicalPath: 'inputs/preassembly-gates.json',
    size: gatesBytes.length,
    sha256: sha256(gatesBytes),
    workflow: gates.workflow,
    runId: gates.runId,
    runAttempt: gates.runAttempt,
    runUrl: gates.runUrl,
    sourceCommit: gates.sourceCommit,
    requiredContexts: gates.requiredContexts,
    requiredJobs: gates.requiredJobs,
  };
}

const VERSION = '0.2.0';

export async function runProtectedBuild({
  admissionRoot, outputRoot, sourceRoot, target, replica,
  environment = process.env, spawn = spawnSync,
  acceptedRecipeSha256 = ACCEPTED_RECIPE_SHA256, adapters,
}) {
  if (![admissionRoot, outputRoot, sourceRoot].every((path) => isAbsolute(path)
    && resolve(path) === path) || !Object.hasOwn(TARGETS, target)
    || ![1, 2].includes(replica) || new Set([admissionRoot, outputRoot, sourceRoot]).size !== 3) {
    reject('exact distinct absolute roots, supported target, and replica are required');
  }
  if (environment.GITHUB_REPOSITORY !== 'zryna/zryna'
    || environment.GITHUB_REF !== 'refs/tags/v0.2.0'
    || environment.GITHUB_SHA !== environment.GITHUB_WORKFLOW_SHA
    || environment.ZRYNA_TARGET !== target || Number(environment.ZRYNA_REPLICA) !== replica) {
    reject('exact protected workflow build identity is required');
  }
  exactReleaseNames(admissionRoot, ['preassembly-gates.json', 'tag-receipt.json']);
  const tagBytes = readReleaseFile(admissionRoot, 'tag-receipt.json', MAX_RELEASE_DOCUMENT);
  const tag = validateReleaseTagReceiptText(tagBytes.toString('utf8'));
  const gatesBytes = readReleaseFile(admissionRoot, 'preassembly-gates.json', MAX_RELEASE_DOCUMENT);
  const gates = validatePreassemblyGatesShape(parseCanonical(gatesBytes.toString('utf8')));
  if (tag.version !== VERSION || tag.source.commit !== environment.GITHUB_SHA
    || gates.sourceCommit !== tag.source.commit) reject('admission source identity differs');

  const recipeBytes = gitBlob(spawn, sourceRoot, tag.source.commit, RECIPE_PATH);
  const recipe = validateRecipe(recipeBytes, acceptedRecipeSha256);
  const implementation = adapters ?? await defaultAdapters();
  for (const name of ['preparePayload', 'assemble', 'verifyArchive', 'captureReleaseMaterials',
    'compileReleaseCli', 'createReleaseSbom', 'createArchitectureReceipt']) {
    if (typeof implementation[name] !== 'function') reject(`build adapter ${name} is missing`);
  }
  const source = {
    repository: tag.source.repository, ref: tag.source.ref, commit: tag.source.commit,
    tree: tag.source.tree, sourceDateEpoch: tag.source.sourceDateEpoch,
  };
  const targetRecord = {
    triple: target,
    archiveFormat: TARGETS[target].archiveFormat,
    platformBaseline: TARGETS[target].platformBaseline,
  };
  const receiptEnvironment = { ...environment };
  if (!receiptEnvironment.ZRYNA_RUSTUP_PATH && isAbsolute(receiptEnvironment.CARGO_HOME ?? '')) {
    receiptEnvironment.ZRYNA_RUSTUP_PATH = join(
      receiptEnvironment.CARGO_HOME,
      process.platform === 'win32' ? 'bin/rustup.exe' : 'bin/rustup',
    );
  }
  const architectureReceipt = bytes(await implementation.createArchitectureReceipt({
    environment: receiptEnvironment, cwd: sourceRoot, spawn,
  }), 'architecture receipt');
  const capturedMaterials = await implementation.captureReleaseMaterials({
    recipe: recipe.value, recipeBytes, sourceRoot, source, target: targetRecord,
    architectureReceipt,
  });
  if (!Array.isArray(capturedMaterials)) reject('captured material list is missing');
  const prepared = implementation.preparePayload({
    version: VERSION, source, target: targetRecord, recipe: recipe.reference,
  }, capturedMaterials, architectureReceipt);
  if (!Buffer.isBuffer(prepared?.distribution) || !Array.isArray(prepared?.payload)) {
    reject('prepared distribution or payload is missing');
  }
  const compiled = await implementation.compileReleaseCli({
    recipe: recipe.value, recipeBytes, sourceRoot, source, target: targetRecord, replica,
    preparedDistribution: prepared.preparedDistribution,
  });
  const cli = bytes(compiled?.cli, 'compiled CLI');
  if (!Array.isArray(compiled.toolchains)) reject('compiled toolchain evidence is missing');
  const compiledCli = artifact(`inputs/compiled-cli/${TARGETS[target].cli}`, cli);
  const input = validateBuildInput({
    format: 'zryna.distribution-build-input.v1',
    version: VERSION,
    tag: `v${VERSION}`,
    channel: 'beta',
    source,
    target: targetRecord,
    toolchains: compiled.toolchains,
    materials: prepared.materials,
    preparedDistribution: prepared.preparedDistribution,
    compiledCli,
    architectureReceipt: {
      format: 'zryna.source-build-receipt.v1',
      ...artifact('inputs/source-build-receipt.json', architectureReceipt),
      sourceCommit: source.commit,
      sourceTree: source.tree,
    },
    gateReceipt: gateDescriptor(gatesBytes, gates),
    recipe: recipe.reference,
  });
  const inputBytes = Buffer.from(`${canonicalBounded(input)}\n`);
  const assembled = await implementation.assemble(inputBytes, {
    distribution: prepared.distribution, payload: prepared.payload, cli, gates: gatesBytes,
  });
  const archive = bytes(assembled?.archive, 'release archive');
  const receipt = bytes(assembled?.receipt, 'build receipt');
  const archiveName = `zryna-${VERSION}-${target}.${TARGETS[target].extension}`;
  if (assembled.filename !== archiveName) reject('assembled archive filename differs');
  const archiveDescriptor = { filename: archiveName, size: archive.length, sha256: sha256(archive) };
  const verified = await implementation.verifyArchive(archive, {
    ...archiveDescriptor, version: VERSION, source, target: targetRecord, recipe: recipe.reference,
  });
  if (verified?.archiveSha256 !== archiveDescriptor.sha256
    || canonical(verified?.distribution) !== canonical(prepared.record)
    || !Array.isArray(verified?.files)
    || canonical(verified.files.map(({ path, mode, data }) => ({
      path, mode, size: data?.length, sha256: Buffer.isBuffer(data) ? sha256(data) : null,
    }))) !== canonical(assembled.files.map(({ path, mode, data }) => ({
      path, mode, size: data?.length, sha256: Buffer.isBuffer(data) ? sha256(data) : null,
    })))) {
    reject('independent archive verification differs from assembly');
  }
  const sbom = bytes(await implementation.createReleaseSbom({
    files: verified.files, recipe: recipe.value, source, target, targetRecord, input, assembled, verified,
    archive: archiveDescriptor,
  }), 'SPDX SBOM');
  validateReleaseSpdx(sbom, { files: verified.files, source, target, archive: archiveDescriptor });

  mkdirSync(outputRoot, { recursive: false, mode: 0o700 });
  const prefix = `zryna-${VERSION}-${target}`;
  const buildReceiptName = `${prefix}.build-receipt.json`;
  const sbomName = `${prefix}.spdx.json`;
  createReleaseFile(outputRoot, archiveName, archive);
  createReleaseFile(outputRoot, buildReceiptName, receipt);
  createReleaseFile(outputRoot, sbomName, sbom);
  const result = {
    format: 'zryna.release-build-result.v1', version: VERSION, target, replica,
    source: { ...source, tagObject: tag.source.tagObject },
    recipe: recipe.reference,
    artifacts: {
      archive: { path: archiveName, size: archive.length, sha256: archiveDescriptor.sha256 },
      buildReceipt: { path: buildReceiptName, size: receipt.length, sha256: sha256(receipt) },
      sbom: { path: sbomName, size: sbom.length, sha256: sha256(sbom) },
    },
  };
  createReleaseFile(outputRoot, 'build-result.json', Buffer.from(`${canonicalBounded(result)}\n`));
  exactReleaseNames(outputRoot, ['build-result.json', archiveName, buildReceiptName, sbomName]);
  return result;
}

function argumentsFrom(argv) {
  const values = new Map();
  for (let index = 0; index < argv.length; index += 2) {
    if (!['--admission', '--output'].includes(argv[index]) || argv[index + 1] === undefined
      || values.has(argv[index])) reject('expected --admission and --output once');
    values.set(argv[index], argv[index + 1]);
  }
  if (values.size !== 2) reject('expected --admission and --output once');
  return values;
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    const values = argumentsFrom(process.argv.slice(2));
    await runProtectedBuild({
      admissionRoot: resolve(values.get('--admission')),
      outputRoot: resolve(values.get('--output')),
      sourceRoot: resolve(process.env.ZRYNA_SOURCE_ROOT ?? ''),
      target: process.env.ZRYNA_TARGET,
      replica: Number(process.env.ZRYNA_REPLICA),
    });
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
