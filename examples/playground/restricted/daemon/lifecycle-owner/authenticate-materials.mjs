// Private operator bridge: real existing signature verification, complete archive verification,
// then exact ten original source captures. No callback verifier or approval boolean is accepted.
import assert from 'node:assert/strict';
import { verifyCompilerSignature } from '../../signature.mjs';
import { verifyArchive } from '../../../../../scripts/distribution/verify.mjs';
import { validateReleaseText } from '../../../../../scripts/distribution-release/validate.mjs';
import { lifecycleMaterialPins } from '../../../../../scripts/stability-gates/lifecycle-owner-inputs.mjs';
import { exact, sha256 } from '../../../../../scripts/stability-gates/input.mjs';
import { release } from '../cli-parity/plan.mjs';
import { captureFile } from '../launcher-capture.mjs';

function capture(pin, maximum, fixed = null) {
  exact(pin, ['sourcePath', 'bytes', 'sha256', 'ownerUid', 'mode'], 'fresh original input');
  assert(Number.isInteger(pin.mode) && pin.mode >= 0 && pin.mode <= 0o777);
  if (fixed) { assert.equal(pin.bytes, fixed.bytes); assert.equal(pin.sha256, fixed.sha256); }
  const source = captureFile({ path: pin.sourcePath, bytes: pin.bytes, sha256: pin.sha256, ownerUid: pin.ownerUid }, maximum, { mode: pin.mode });
  try { source.revalidate(); return source.bytes; }
  finally { source.close(); }
}

export function selectMaterialBytes(files) {
  assert.equal(files.length, 349); assert.equal(files.reduce((n, file) => n + file.data.length, 0), 154023815);
  const byPath = new Map(files.map(file => [file.path, file])); assert.equal(byPath.size, files.length);
  return lifecycleMaterialPins().map(pin => {
    const file = byPath.get(pin.path.slice('/materials/'.length));
    assert(file && file.data.length === pin.bytes && sha256(file.data) === pin.sha256
      && file.mode === (pin.mode === 0o555 ? 0o755 : 0o644), 'exact original material member');
    return { pin, data: file.data };
  });
}

// These original physical sources are checked again by the unchanged retained-handle
// Python Sources on every archive emission. No second custom staging/custody layer.
export function selectMaterialSources(rows) {
  const pins = lifecycleMaterialPins().sort((a, b) => a.path < b.path ? -1 : 1);
  assert(Array.isArray(rows) && rows.length === pins.length);
  return rows.map((row, index) => {
    exact(row, ['path', 'sourcePath', 'ownerUid'], 'independently selected original material source');
    assert.equal(row.path, pins[index].path, 'complete ordered original ten-file mapping');
    assert(typeof row.sourcePath === 'string' && row.sourcePath.startsWith('/'));
    assert(Number.isInteger(row.ownerUid) && row.ownerUid >= 0 && row.ownerUid <= 4294967295);
    return { ...pins[index], mode: pins[index].mode === 0o555 ? 0o755 : 0o644, sourcePath: row.sourcePath, ownerUid: row.ownerUid };
  });
}

export async function authenticateMaterials(input) {
  exact(input, ['envelope', 'bundle', 'archive', 'trustedRoot', 'verifierPolicy', 'materialSources'], 'private material authentication');
  input = structuredClone(input);
  // No untrusted material executes: only the separately pinned static verifier is invoked.
  const envelope = capture(input.envelope, 262144, { bytes: 7253, sha256: release.envelopeSha256 });
  const bundle = capture(input.bundle, 1048576, { bytes: 10990, sha256: release.bundleSha256 });
  const archive = capture(input.archive, release.archive.bytes, release.archive);
  const root = capture(input.trustedRoot, 262144);
  const document = validateReleaseText(new TextDecoder('utf-8', { fatal: true }).decode(envelope));
  assert.equal(document.source.commit, release.sourceCommit); assert.equal(document.source.tree, release.sourceTree);
  assert.equal(document.version, release.version);
  verifyCompilerSignature(envelope, bundle, input.verifierPolicy, root, release.sourceCommit);
  const subject = document.subjects.find(value => value.target === 'x86_64-unknown-linux-gnu');
  assert(subject && subject.archive.size === archive.length && subject.archive.sha256 === release.archive.sha256);
  const verified = await verifyArchive(archive, { filename: subject.archive.path, size: subject.archive.size,
    sha256: subject.archive.sha256, version: document.version, source: document.source,
    target: { triple: subject.target, archiveFormat: 'tar-gzip', platformBaseline: subject.platformBaseline }, recipe: document.recipe });
  selectMaterialBytes(verified.files); // All ten member bytes/modes are bound to the freshly verified complete archive.
  const original = selectMaterialSources(input.materialSources);
  const captureMaterials = () => {
    for (const { sourcePath, bytes, sha256, ownerUid, mode } of original) capture({ sourcePath, bytes, sha256, ownerUid, mode }, 128 * 1024 ** 2);
  };
  captureMaterials();
  // Re-read all original input paths after verification and original material capture; no historical receipt suffices.
  for (const [pin, maximum] of [[input.envelope, 262144], [input.bundle, 1048576],
    [input.archive, release.archive.bytes], [input.trustedRoot, 262144]]) capture(pin, maximum);
  captureMaterials();
  const materialSources = original.map(({ path, sourcePath }) => ({ path, sourcePath, provenance: 'original-release' }));
  return { materialSources, archiveSha256: verified.archiveSha256, source: document.source,
    flags: { sourceAuthenticated: false, hostAuthenticated: false, runtimeApproved: false,
      materialsApproved: false, ownerExecuted: false,
      executionApproved: false, proofQualified: false, performanceQualified: false },
    scope: 'fresh signature/archive verification and matched original source reads; no runtime/launch authority' };
}
