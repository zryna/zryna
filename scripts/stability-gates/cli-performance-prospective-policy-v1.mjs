// Frozen engineering declaration, never a numeric-table anchor or authentication/admission issuer.
import assert from 'node:assert/strict';
import { canonical, exact, parse, sha256 } from './input.mjs';
export const PROSPECTIVE_POLICY_SHA256 = '2d034cc834d4d4f13ef0c92ff02c7d082858c3e2598af231460623d9d802861c';
const METHODOLOGY_SHA256 = '5aa0ea425c752976a6eb1a4baa7ae7ca703d655270ef5272b78ec28564dffa1c';
export function parseProspectivePolicy(bytes) {
  assert.equal(arguments.length, 1, 'S418 policy: no caller expected digest, override or admission input');
  const declaration = parse(bytes); // Existing canonical JSON/byte/node/depth limits; no file or process IO.
  exact(declaration, ['format', 'scope', 'methodologySha256', 'engineeringBasis', 'ratios', 'absoluteRule',
    'host', 'sampling', 'schedule', 'rationale', 'sourceAuthenticated', 'proofQualified', 'performanceQualified',
    'baselineAdmitted', 'baselineAuthority', 'thresholdAuthority', 'thresholds', 'numericTableAnchor', 'closureStatus'],
  'prospective policy declaration');
  assert.equal(declaration.format, 'zryna.cli-performance-prospective-policy.v1');
  assert.equal(declaration.scope, 'shared-hosted-vm-regression-only');
  assert.equal(declaration.methodologySha256, METHODOLOGY_SHA256);
  // Whole canonical identity closes every nested field and value; never learn a new policy from input.
  assert.equal(sha256(canonical(declaration)), PROSPECTIVE_POLICY_SHA256, 'S418 policy: frozen engineering decision identity');
  for (const key of ['sourceAuthenticated', 'proofQualified', 'performanceQualified', 'baselineAdmitted'])
    assert.equal(declaration[key], false);
  for (const key of ['baselineAuthority', 'thresholdAuthority', 'thresholds', 'numericTableAnchor'])
    assert.equal(declaration[key], null);
  assert.equal(declaration.closureStatus, 'blocked');
  return Object.freeze({ format: 'zryna.cli-performance-policy-declaration.v1',
    policySha256: PROSPECTIVE_POLICY_SHA256, methodologySha256: METHODOLOGY_SHA256,
    ratios: structuredClone(declaration.ratios), declaration,
    sourceAuthenticated: false, proofQualified: false, performanceQualified: false, baselineAdmitted: false,
    baselineAuthority: null, thresholdAuthority: null, thresholds: null, numericTableAnchor: null,
    closureStatus: 'blocked' });
}
