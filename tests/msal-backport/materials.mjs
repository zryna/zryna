// Test-only material consistency, not a source/proof/advisory-disposition issuer.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { lstatSync, readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { TextDecoder } from 'node:util';
import { fileURLToPath } from 'node:url';
import { parseDocument } from 'yaml';
export const root = fileURLToPath(new URL('../../', import.meta.url));
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
function read(path, maximum = 2 * 1024 * 1024) {
  const stat = lstatSync(path);
  assert(stat.isFile() && !stat.isSymbolicLink() && stat.size <= maximum, 'bounded regular material');
  const bytes = readFileSync(path);
  assert.equal(bytes.length, stat.size, 'complete material read');
  return bytes;
}
function document(path) {
  const parsed = parseDocument(new TextDecoder('utf-8', { fatal: true }).decode(read(path)));
  assert.deepEqual(parsed.errors, [], 'valid workspace/lock document');
  return parsed.toJS();
}
function packageFrom(parent, expected) {
  const path = parent.resolve(`${expected.package}/package.json`);
  const data = JSON.parse(read(path, 64 * 1024));
  assert.equal(data.name, expected.package); assert.equal(data.version, expected.version);
  return { path, require: createRequire(path) };
}
export function auditMaterials() {
  const provenanceBytes = read(new URL('./provenance.json', import.meta.url), 16 * 1024);
  assert.equal(hash(provenanceBytes), 'c236c11e16b7983111840ea3f6051d1422864a3b8e3160906fd601a7c284818f', 'fixed reviewed backport provenance');
  const provenance = JSON.parse(provenanceBytes);
  assert.equal(provenance.format, 'zryna.msal6-upstream-backport-regression.v1');
  const request = createRequire(join(root, 'package.json'));
  const vsce = packageFrom(request, provenance.dependencyPath[0]);
  const identity = packageFrom(vsce.require, provenance.dependencyPath[1]);
  const msal = packageFrom(identity.require, provenance.dependencyPath[2]);
  const patch = read(join(root, provenance.patch.path), 64 * 1024);
  assert.equal(patch.length, provenance.patch.canonicalLfBytes);
  assert.equal(hash(patch), provenance.patch.canonicalLfSha256, 'canonical LF patch bytes');
  assert(!patch.includes(13), 'repository patch must retain canonical LF');
  const lock = document(join(root, 'pnpm-lock.yaml'));
  const workspace = document(join(root, 'pnpm-workspace.yaml'));
  assert.equal(workspace.patchedDependencies['@azure/msal-node@6.0.1'], provenance.patch.path);
  assert.equal(lock.patchedDependencies['@azure/msal-node@6.0.1'], provenance.patch.pnpmNormalizedSha256);
  const packageRoot = dirname(msal.path), variance = [];
  assert.equal(provenance.installedAfterimages.length, 7);
  for (const pin of provenance.installedAfterimages) {
    const bytes = read(join(packageRoot, pin.path));
    const actual = hash(bytes);
    const study = actual === pin.studyRawSha256 && bytes.length === pin.studyRawBytes;
    const deployed = actual === pin.canonicalPatchAppliedSha256 && bytes.length === pin.canonicalPatchAppliedBytes;
    assert(study || deployed, `exact reviewed raw SDK afterimage: ${pin.path}`);
    // Only CRLF pairs normalize: a remaining bare CR is preserved exactly, never stripped.
    const lf = Buffer.from(new TextDecoder('utf-8', { fatal: true }).decode(bytes).replaceAll('\r\n', '\n'));
    assert.equal(lf.length, pin.canonicalLfBytes);
    assert.equal(hash(lf), pin.canonicalLfSha256, `no non-EOL SDK drift: ${pin.path}`);
    if (!study) variance.push({ path: pin.path, rawSha256: actual, studyRawSha256: pin.studyRawSha256,
      policy: 'reviewed canonical-patch application; CRLF-to-LF content equality, bare CR untouched' });
  }
  return { packageRoot, requireMsal: identity.require, provenance, variance };
}
