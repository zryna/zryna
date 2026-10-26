// Coverage-declaration mutation controls, not a receipt verifier or host proof campaign.
import assert from 'node:assert/strict';
import test from 'node:test';
import { pristine, rejects, row } from './stability-contract-coverage-support.mjs';
import { parseDeclaredCoverage, validateDeclaredCoverage } from '../scripts/stability-gates/contract-coverage.mjs';

test('m4-mode-coverage-is-exact', () => {
  const document = pristine();
  const ids = document.rows.filter(item => item.id.startsWith('m4-'))
    .flatMap(item => item.existing_proof_owners[0].row_ids);
  assert.equal(ids.length, 12); assert.equal(new Set(ids).size, 12);
  for (const mode of document.rows.filter(item => item.id.startsWith('m4-'))) {
    rejects(doc => { row(doc, mode.id).existing_proof_owners[0].row_ids.pop(); });
    rejects(doc => { row(doc, mode.id).existing_proof_owners[0].case_ids.pop(); });
    rejects(doc => { row(doc, mode.id).existing_proof_owners[0].gate_ids.pop(); });
    rejects(doc => { row(doc, mode.id).evidence_state = 'passed-after-repin'; });
  }
});
test('conditional-m6-stays-conditional', () => {
  const id = 'reviewed-source-playground';
  assert(row(pristine(), id).planning_status.startsWith('conditional advertised support:'));
  rejects(doc => { row(doc, id).planning_status = 'toolkit published'; });
  rejects(doc => { row(doc, id).evidence_state = 'passed'; });
  rejects(doc => { row(doc, id).existing_proof_owners = row(doc, 'lsp-diagnostics-definition').existing_proof_owners; });
  rejects(doc => { row(doc, id).explicit_exclusions = ['none']; });
  rejects(doc => { doc.publication = { finalRecord: 'fabricated' }; });
});
test('historical-and-candidate-receipts-distinct', () => {
  rejects(doc => { doc.sourceSnapshot.candidateRevision = 'a'.repeat(40); });
  rejects(doc => { doc.sourceSnapshot.mergedRevision = 'a'.repeat(40); });
  rejects(doc => { doc.sourceSnapshot.role = 'authenticated-tested-merge'; });
  for (const role of ['old-tag', 'ancestor', 'equal-tree', 'pull-request-head', 'tested-merge']) {
    rejects(doc => { doc.receipts = [{ role, source: 'a'.repeat(40), host: 'linux' }]; });
  }
  assert.equal(validateDeclaredCoverage(pristine()).sourceAuthenticated, false);
});
test('required-ignored-owner-is-not-skipped-proof', () => {
  // This slice rejects proof promotion; it deliberately does NOT reconcile ignored totals.
  const shared = doc => row(doc, 'm7-bounded-generics-candidate')
    .existing_proof_owners[0].required_named_tests.find(command => command.id === 'shared-abi-ignored');
  assert.equal(shared(pristine()).tests.windows.length, 3);
  rejects(doc => { shared(doc).tests.windows.pop(); });
  rejects(doc => { shared(doc).tests.windows[0] = 'unknown_ignored_name'; });
  rejects(doc => { shared(doc).tests.windows[1] = shared(doc).tests.windows[0]; });
  rejects(doc => { doc.proofAccounting = 'allow-all-ignored'; });
  rejects(doc => { doc.rawDefaultCounts = { passed: 1, ignored: 0 }; });
  rejects(doc => { doc.requiredProofs = [{ name: 'fixture', passed: 0, ignored: 1 }]; });
  assert.equal(validateDeclaredCoverage(pristine()).requiredProofAccounting, 'deferred-separate-slice');
});
test('proof-logs-are-complete-and-bound', () => {
  // Exact owner declarations are bound; actual receipt/log validation is a separate next slice.
  const command = doc => row(doc, 'm7-bounded-generics-candidate')
    .existing_proof_owners[0].required_named_tests[0];
  rejects(doc => { command(doc).args.push('--list'); });
  rejects(doc => { command(doc).platforms = ['windows']; });
  rejects(doc => { command(doc).tests.linux = []; });
  rejects(doc => { command(doc).tests.linux.reverse(); });
  rejects(doc => { doc.receipt = { gates: [], passed: 999, source: 'a'.repeat(40), host: 'wrong' }; });
  rejects(doc => { doc.logs = [{ sha256: '0'.repeat(64), stdout: 'test result: ok. 1 passed;' }]; });
});
test('thresholds-cannot-self-learn', () => {
  assert.equal(validateDeclaredCoverage(pristine()).thresholdApproval, null);
  for (const value of [0, 1, { wallMs: 1 }, NaN, Infinity]) {
    rejects(doc => { doc.thresholds = value; });
  }
  rejects(doc => { doc.measurements = { samples: [1], approveOwnCeiling: true }; });
  rejects(doc => { doc.historyPreserved.thresholds = 'approved'; });
});
test('resource-policy-is-not-refilled', () => {
  rejects(doc => { doc.budgets = { fuel: 999999999, childSeconds: 999 }; });
  rejects(doc => { row(doc, 'm4-command-checkout').runtimeFuel = 999999999; });
  rejects(doc => { doc.traps = { SIGILL: 'source-bounds' }; });
  rejects(doc => { doc.unknown = 'x'.repeat(256 * 1024 + 1); });
  assert.throws(() => parseDeclaredCoverage(Buffer.from([255])));
});
test('completion-does-not-follow-unit-success', () => {
  const assessment = validateDeclaredCoverage(pristine());
  assert(Object.isFrozen(assessment)); assert.equal(assessment.proofQualified, false);
  assert.equal(assessment.closureStatus, 'blocked');
  rejects(doc => { doc.closureStatus = 'complete'; });
  rejects(doc => { doc.migrationStatus = '1.0-public-support'; });
  rejects(doc => { doc.remainingDependencies = []; });
  rejects(doc => { doc.remainingDependencies[0] = 'all unit tests passed; no obligations remain'; });
  rejects(doc => { doc.rows[0].evidence_state = 'passed'; });
  rejects(doc => { doc.releaseAuthorized = true; });
});
