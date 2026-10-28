import assert from 'node:assert/strict';
import { chmodSync, constants, copyFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { execFileSync, spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { sha256, readSafe } from './input.mjs';
import { validateLifecycleCargoArtifacts } from './lifecycle-owner-prebuild.mjs';
import { verifyArchive } from '../distribution/verify.mjs';
import { validateReleaseText } from '../distribution-release/validate.mjs';
import { extractVerifiedProductionFiles } from '../distribution-release/extract-verified-files.mjs';
import { authenticateMaterials } from '../../examples/playground/restricted/daemon/lifecycle-owner/authenticate-materials.mjs';
import { verifyCompilerSignature } from '../../examples/playground/restricted/signature.mjs';
import { lifecycleMaterialPins } from './lifecycle-owner-inputs.mjs';
import { save, selection } from './lifecycle-private-io.mjs';
import { acquireCapture } from './lifecycle-private-artifact.mjs';
import { exposeObservations, protectedWork, relocate, restoreInputs, validateAuth } from './lifecycle-private-custody.mjs';
import { captureFile } from '../../examples/playground/restricted/daemon/launcher-capture.mjs';

export async function run(base, input) {
  validateAuth(input.auth, base);
  assert.equal(input.preparation.plan.source.commit, process.env.GITHUB_SHA);
  const work = protectedWork();
  process.stdout.write('retention_root=' + work + '\n');
  try { await executeProtected(base, input, work); }
  finally { exposeObservations(work); }
}

async function executeProtected(base, input, work) {
  const files = await acquireCapture(input.artifact, process.env.GITHUB_SHA, work);
  restoreInputs(input.auth, files, base, work);
  const prep = input.preparation, plan = prep.plan, originals = join(work, 'input/originals');
  const complete = JSON.parse(readSafe(join(work, 'input'), 'capture-complete.json'));
  assert.deepEqual(complete.source, plan.source);
  assert.deepEqual(complete.artifacts, plan.prebuild.artifacts);
  assert.equal(complete.assetsSha256, input.assetsSha256);
  assert.deepEqual(complete.rawCapture, ['capture-main.tar.gz', 'capture-after.tar.gz'].map(path => {
    const row = input.artifact.entries.find(entry => entry.path === path);
    return { path, bytes: row.size, sha256: row.sha256 };
  }), 'both raw archive pins retained');
  const supervisor = prep.runtime.files.find(row => row.path === '/lifecycle/supervisor');
  for (const key of ['bytes', 'sha256']) assert.equal(supervisor[key], complete.supervisor[key]);
  const assets = readSafe(join(work, 'input'), 'download-selection.json'); assert.equal(sha256(assets), input.assetsSha256);
  for (const row of selection(assets.toString(), 'capture').downloads) {
    const bytes = readSafe(join(work, 'input'), 'originals/' + row.name, row.bytes);
    assert.equal(bytes.length, row.bytes); assert.equal(sha256(bytes), row.sha256);
  }
  const raw = Object.fromEntries([['metadata', 'metadata-before.stdout.raw'], ['stdout', 'build.stdout.raw'], ['stderr', 'build.stderr.raw']]
    .map(([key, path]) => [key, readSafe(join(work, 'input'), 'capture/' + path, 16 * 1024 ** 2)]));
  validateLifecycleCargoArtifacts({ root: plan.prebuild.command.cwd, tools: plan.prebuild.tools, artifacts: plan.prebuild.artifacts }, raw);
  for (const key of Object.keys(raw)) assert.deepEqual({ bytes: raw[key].length, sha256: sha256(raw[key]) }, plan.prebuild.raw[key]);
  for (const row of prep.runtimeSources) row.sourcePath = relocate(row.sourcePath, base, work);
  for (const row of Object.values(prep.provenance)) row.path = relocate(row.path, base, work);
  copyFileSync(join(originals, 'cosign'), join(work, 'cosign'), constants.COPYFILE_EXCL); chmodSync(join(work, 'cosign'), 0o500);
  const auth = structuredClone(input.auth);
  for (const key of ['archive', 'envelope', 'bundle', 'trustedRoot']) auth[key].sourcePath = relocate(auth[key].sourcePath, base, work);
  auth.verifierPolicy.path = join(work, 'cosign');
  const captured = {};
  try {
    for (const key of ['archive', 'envelope', 'bundle', 'trustedRoot']) {
      const pin = auth[key];
      captured[key] = captureFile({ path: pin.sourcePath, bytes: pin.bytes, sha256: pin.sha256, ownerUid: pin.ownerUid }, 192 * 1024 ** 2, { mode: pin.mode });
    }
  } finally { for (const value of Object.values(captured)) value.close(); }
  const envelope = captured.envelope.bytes;
  const document = validateReleaseText(envelope.toString('utf8')), subject = document.subjects.find(row => row.target === 'x86_64-unknown-linux-gnu');
  verifyCompilerSignature(envelope, captured.bundle.bytes, auth.verifierPolicy,
    captured.trustedRoot.bytes, document.source.commit);
  const verified = await verifyArchive(captured.archive.bytes, { filename: subject.archive.path, size: subject.archive.size,
    sha256: subject.archive.sha256, version: document.version, source: document.source, target: { triple: subject.target, archiveFormat: 'tar-gzip', platformBaseline: subject.platformBaseline }, recipe: document.recipe });
  const command = (tool, args) => execFileSync(tool, args, { timeout: 30000, maxBuffer: 65536, stdio: ['ignore', 'pipe', 'pipe'] });
  for (const name of ['libc6', 'libgcc-s1', 'libstdcxx6', 'gcc-14-base']) command('/usr/bin/dpkg-deb', ['-x', join(originals, name), join(work, name)]);
  mkdirSync(join(work, 'python')); command('/usr/bin/tar', ['-xzf', join(originals, 'python'), '-C', join(work, 'python')]);
  const oldMask = process.umask(0o022);
  try { extractVerifiedProductionFiles(join(work, 'release'), verified.files); } finally { process.umask(oldMask); }
  const materialSources = lifecycleMaterialPins().sort((a, b) => a.path < b.path ? -1 : 1)
    .map(pin => ({ path: pin.path, sourcePath: join(work, 'release', pin.path.slice('/materials/'.length)), ownerUid: 0 }));
  const authentication = await authenticateMaterials({ ...auth, materialSources });
  save(join(work, 'authentication.json'), authentication); prep.materialSources = authentication.materialSources;
  save(join(work, 'handoff.json'), prep);
  const child = spawnSync('/usr/bin/python3', ['-I', '-S', '-B', join(dirname(fileURLToPath(import.meta.url)), 'lifecycle-private.py')],
    { input: JSON.stringify({ work, preparation: prep }), env: process.env, timeout: 240000, maxBuffer: 65536 });
  for (const stream of ['stdout', 'stderr']) {
    if (Buffer.isBuffer(child[stream])) save(join(work, 'driver-' + stream + '.raw'), child[stream]);
  }
  save(join(work, 'driver-result.json'), { status: child.status, signal: child.signal, error: child.error?.message ?? null });
  assert.deepEqual([child.status, child.signal, child.error], [0, null, undefined]);
}
