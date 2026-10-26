// Declared coverage only. No receipt issuer, gate runner, admission or completion authority.
import assert from 'node:assert/strict';
import { canonical, exact, parse, portable, readSafe, sha256 } from './input.mjs';
import { INDEX_PATH, INDEX_SHA256, LEGACY_SHA256, MATERIAL_SHA256, MERGED_REVISION, MERGED_TREE, PLAN_SHA256, POLICY_SHA256, ROWS } from './contract-coverage-authority.mjs';

export const MAX_MATERIAL_BYTES = 2 * 1024 * 1024;
const MAX_TOTAL_BYTES = 16 * 1024 * 1024;
const ROOT_KEYS = ['format', 'scope', 'closureStatus', 'thresholds', 'proofAccounting',
  'migrationStatus', 'sourceSnapshot', 'historyPreserved', 'scopeRule', 'rows', 'materialPins',
  'remainingDependencies', 'notDependencies'];
const ROW_KEYS = ['id', 'versions', 'advertised_surface', 'authority_files', 'fixture_or_proof_inputs',
  'existing_proof_owners', 'migration_rule_to_freeze', 'platforms', 'explicit_exclusions',
  'planning_status', 'evidence_state'];
const fingerprint = value => sha256(canonical(value));

function strings(value, label) {
  assert(Array.isArray(value) && value.length > 0 && value.length <= 256, `${label}: bounded list`);
  assert(value.every(item => typeof item === 'string' && item.length > 0 && item.length <= 4096), label);
  assert.equal(new Set(value).size, value.length, `${label}: unique`);
}
function snapshot(value) {
  exact(value, ['planningIndexSha256', 'candidateRevision', 'mergedRevision', 'mergedTree', 'role', 'refresh'], 'snapshot');
  assert.equal(value.planningIndexSha256, PLAN_SHA256, 'planning provenance');
  assert.equal(value.candidateRevision, null, 'no authenticated candidate revision in this snapshot');
  assert.equal(value.mergedRevision, MERGED_REVISION, 'reviewed merged-base reference only');
  assert.equal(value.mergedTree, MERGED_TREE, 'reviewed merged tree reference only');
  assert.equal(value.role, 'reviewed-merged-base-reference-not-candidate-or-host-evidence');
  assert.equal(value.refresh, 'required-on-selected-candidate-change-and-independent-review');
}
function history(value) {
  exact(value, ['registry', 'sha256', 'reader', 'closure_status', 'thresholds'], 'legacy history');
  assert.equal(value.registry, 'tests/stability-gates-v1.json');
  assert.equal(value.sha256, LEGACY_SHA256, 'immutable historical registry');
  assert.equal(value.reader, 'scripts/stability-gates/registry.mjs');
  assert.equal(value.closure_status, 'blocked');
  assert.equal(value.thresholds, 'null; unchanged');
}
function materialCatalogue(pins) {
  assert(Array.isArray(pins) && pins.length === 184, 'exact selected material catalogue');
  const paths = new Set();
  let previous = '', total = 0;
  for (const pin of pins) {
    exact(pin, ['path', 'bytes', 'sha256'], 'material pin');
    portable(pin.path);
    assert(!paths.has(pin.path) && pin.path > previous, 'unique ordered material paths');
    assert(Number.isSafeInteger(pin.bytes) && pin.bytes > 0 && pin.bytes <= MAX_MATERIAL_BYTES, 'material size');
    assert.match(pin.sha256, /^[a-f0-9]{64}$/u, 'material digest');
    total += pin.bytes;
    assert(total <= MAX_TOTAL_BYTES, 'aggregate selected material bytes');
    paths.add(pin.path); previous = pin.path;
  }
  assert.equal(fingerprint(pins), MATERIAL_SHA256, 'reviewed material catalogue identity');
  return paths;
}
function rows(value, paths) {
  assert(Array.isArray(value) && value.length === ROWS.length, 'complete declared contract inventory');
  for (const [index, row] of value.entries()) {
    exact(row, ROW_KEYS, 'contract row');
    assert.equal(row.id, ROWS[index][0], 'exact unique ordered contract identity');
    strings(row.versions, 'contract versions');
    strings(row.authority_files, 'contract authorities');
    strings(row.fixture_or_proof_inputs, 'contract fixtures');
    strings(row.explicit_exclusions, 'contract exclusions');
    assert(typeof row.migration_rule_to_freeze === 'string' && row.migration_rule_to_freeze.length > 0,
      'explicit proposed migration rule');
    assert.equal(row.evidence_state, 'not-collected-by-this-plan', 'declarations never certify proof');
    for (const path of [...row.authority_files, ...row.fixture_or_proof_inputs]) {
      portable(path); assert(paths.has(path), 'authority/fixture must have selected material pin');
    }
    // The independently frozen planning rows bind selectors, versions, migration, owners,
    // host exclusions and API/CLI/conditional scope; callers cannot supply another baseline.
    assert.equal(fingerprint(row), ROWS[index][1], `reviewed declared semantics: ${row.id}`);
  }
}
export function validateDeclaredCoverage(document) {
  canonical(document); // Existing bounded JSON traversal, strings, depth and document size.
  exact(document, ROOT_KEYS, 'declared coverage');
  assert.equal(document.format, 'zryna.stability-contract-coverage.v2');
  assert.equal(document.scope, 'declared-snapshot-only');
  assert.equal(document.closureStatus, 'blocked', 'coverage cannot close an issue/release');
  assert.equal(document.thresholds, null, 'threshold approval is a separate obligation');
  assert.equal(document.proofAccounting, 'deferred-separate-required-owner-slice');
  assert.equal(document.migrationStatus, 'proposed-not-1.0-activation');
  snapshot(document.sourceSnapshot); history(document.historyPreserved);
  assert.equal(document.scopeRule,
    'actual public CLI surfaces plus expressly restricted/conditional library/tooling contracts; no new 1.0 advertisement or public activation');
  strings(document.remainingDependencies, 'remaining dependencies');
  strings(document.notDependencies, 'separate obligations');
  assert.equal(document.remainingDependencies.length, 6);
  assert.equal(document.notDependencies.length, 3);
  const policy = { ...document };
  delete policy.rows; delete policy.materialPins;
  assert.equal(fingerprint(policy), POLICY_SHA256, 'reviewed snapshot policy/dependencies');
  rows(document.rows, materialCatalogue(document.materialPins));
  return Object.freeze({ status: 'declared-index-valid', rows: ROWS.length, materials: 184,
    closureStatus: 'blocked', proofQualified: false, sourceAuthenticated: false,
    requiredProofAccounting: 'deferred-separate-slice', thresholdApproval: null });
}
export function parseDeclaredCoverage(bytes) {
  const document = parse(bytes);
  assert.equal(sha256(bytes), INDEX_SHA256, 'frozen declared snapshot bytes');
  validateDeclaredCoverage(document);
  return document;
}
export function loadDeclaredCoverage(root) {
  return parseDeclaredCoverage(readSafe(root, INDEX_PATH));
}
// A pure byte check; callback bytes are not authenticated host/source/release evidence.
export function checkDeclaredMaterials(document, bytesForPath) {
  validateDeclaredCoverage(document);
  assert.equal(typeof bytesForPath, 'function', 'bounded selected material byte reader');
  let total = 0;
  for (const pin of document.materialPins) {
    const bytes = bytesForPath(pin.path, pin.bytes);
    assert(Buffer.isBuffer(bytes), 'material bytes');
    assert.equal(bytes.length, pin.bytes, `material byte count: ${pin.path}`);
    assert.equal(sha256(bytes), pin.sha256, `material digest: ${pin.path}`);
    total += bytes.length;
  }
  return Object.freeze({ status: 'declared-material-bytes-match', files: 184, bytes: total,
    closureStatus: 'blocked', proofQualified: false, sourceAuthenticated: false });
}
