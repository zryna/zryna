// Inert declaration controls; no collection, numeric-table derivation, host or platform authentication.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { parseProspectivePolicy } from '../scripts/stability-gates/cli-performance-prospective-policy-v1.mjs';
const raw = readFileSync(new URL('./cli-performance-prospective-policy-v1.json', import.meta.url));
const fresh = () => JSON.parse(raw);
function rejected(change) {
  const input = fresh(); change(input);
  assert.throws(() => parseProspectivePolicy(Buffer.from(canonical(input))));
  assert.equal(parseProspectivePolicy(raw).performanceQualified, false, 'pristine independent retry');
}
test('reviewed engineering choices remain exact rationals with worse-baseline reference and once-only ceil', () => {
  const policy = parseProspectivePolicy(raw), d = policy.declaration;
  assert.deepEqual(policy.ratios, {
    withinRoundSpread: { wall: { numerator: '1', denominator: '5' }, memory: { numerator: '1', denominator: '10' } },
    crossBaselineMedian: { wall: { numerator: '11', denominator: '10' }, memory: { numerator: '21', denominator: '20' } },
    heldoutOverWorseBaselineMedian: { wall: { numerator: '11', denominator: '10' }, memory: { numerator: '21', denominator: '20' } },
  });
  assert.equal(d.absoluteRule.reference, 'maximum of all ten measured baseline values for the same cell');
  assert.equal(d.absoluteRule.rounding, 'ceil ONCE after exact max and multiplication by the declared ratio');
  assert.equal(d.absoluteRule.qpcConversion, 'ticks * 1000000000 / frequency as an exact rational before max and margin');
  assert.equal(d.absoluteRule.containmentWallNs, '15000000000');
  assert.match(d.absoluteRule.containmentRole, /not a product or performance SLO/u);
  assert.match(d.rationale.betterBaselineBoundary, /21 percent wall and 10.25 percent memory above the better/u);
  for (const key of ['confidenceInterval', 'productSlo', 'lowFalsePositiveRateGuaranteed', 'causalAttribution'])
    assert.equal(d.rationale[key], false);
});
test('no unauthorized tolerance, best-reference, summary-only or rounded-before-margin substitutions', () => {
  for (const mutate of [d => { d.ratios.withinRoundSpread.wall.denominator = '4'; },
    d => { d.ratios.withinRoundSpread.memory.denominator = '9'; },
    d => { d.ratios.crossBaselineMedian.wall.numerator = '12'; },
    d => { d.ratios.crossBaselineMedian.memory.numerator = '22'; },
    d => { d.ratios.heldoutOverWorseBaselineMedian.wall.numerator = '12'; },
    d => { d.ratios.heldoutOverWorseBaselineMedian.memory.numerator = '22'; },
    d => { d.absoluteRule.wallRatio.numerator = '12'; }, d => { d.absoluteRule.memoryRatio.numerator = '22'; },
    d => { d.absoluteRule.reference = 'best baseline median'; },
    d => { d.absoluteRule.rounding = 'round each QPC duration before applying margin'; },
    d => { d.absoluteRule.containmentWallNs = '15000000001'; }]) rejected(mutate);
});
test('actual fact names include kernel/service VM scope while unknowns never become unlimited capacity', () => {
  const d = parseProspectivePolicy(raw).declaration;
  assert.deepEqual(d.host.requiredObservedFacts, ['cpuModel', 'osImage', 'kernel', 'visibleCpuCount', 'affinity', 'virtualization']);
  assert.equal(d.host.serviceVmScopeFact, 'virtualization');
  assert.deepEqual(d.host.unknownFacts, ['outerCpuQuota', 'outerMemoryQuota', 'powerManagement', 'physicalExclusivity']);
  assert.match(d.host.serviceScope, /not exclusive physical host/u);
  assert.match(d.host.unknownMeaning, /never infer unlimited, zero, absent or physically exclusive/u);
  for (const mutate of [p => { p.host.requiredObservedFacts.splice(2, 1); },
    p => { p.host.serviceVmScopeFact = 'unobserved'; }, p => { p.host.unknownMeaning = 'unavailable means unlimited'; },
    p => { p.host.sameClass = 'pool different CPU models'; }, p => { p.host.unknownFacts = []; },
    p => { p.host.memoryMethods['windows-x86_64'] = p.host.memoryMethods['linux-x86_64']; }]) rejected(mutate);
});
test('v4 cache/fresh-process/order identity is fixed without claiming observed compliance', () => {
  const d = parseProspectivePolicy(raw).declaration;
  const methodology = readFileSync(new URL('./cli-performance-methodology-v4.json', import.meta.url));
  assert.equal(sha256(methodology), d.methodologySha256);
  const v4 = JSON.parse(methodology);
  assert.equal(d.sampling.cache, v4.samples.cache); assert.equal(d.sampling.order, v4.samples.order);
  assert.equal(d.sampling.invalid, v4.samples.invalid);
  assert.equal(d.sampling.warmupsPerCell, 1); assert.equal(d.sampling.measuredPerCell, 5);
  for (const mutate of [p => { p.methodologySha256 = 'f'.repeat(64); },
    p => { p.sampling.cache = 'cold OS and warm JIT'; }, p => { p.sampling.order = 'concurrent cells'; },
    p => { p.sampling.measuredPerCell = 4; }, p => { p.sampling.invalid = 'drop the slow sample'; }]) rejected(mutate);
});
test('fresh six-job finite schedule rejects replacement slots, late table anchoring and policy retuning', () => {
  const s = parseProspectivePolicy(raw).declaration.schedule;
  assert.equal(s.freshBaselineJobsPerHost, 2); assert.equal(s.freshBaselineJobsTotal, 4);
  assert.equal(s.heldoutJobsPerHost, 1); assert.equal(s.heldoutJobsTotal, 2); assert.equal(s.measurementJobsTotal, 6);
  assert.equal(s.hostCells, 44); assert.equal(s.baselineOrder, 'serial per host');
  assert.match(s.heldoutPrecondition, /authentically and immutably anchored before either heldout job starts/u);
  for (const mutate of [d => { d.schedule.freshBaselineJobsPerHost = 1; },
    d => { d.schedule.measurementJobsTotal = 7; }, d => { d.schedule.forbidden = []; },
    d => { d.schedule.baselineOrder = 'pick best of repeated runs'; },
    d => { d.schedule.heldoutPrecondition = 'anchor after candidate samples'; }]) rejected(mutate);
});
test('raw-byte/copy inspection grants no source, host, baseline or threshold authority', () => {
  const a = parseProspectivePolicy(raw), b = parseProspectivePolicy(Buffer.from(raw));
  for (const value of [a, b]) {
    for (const key of ['sourceAuthenticated', 'proofQualified', 'performanceQualified', 'baselineAdmitted'])
      assert.equal(value[key], false);
    for (const key of ['baselineAuthority', 'thresholdAuthority', 'thresholds', 'numericTableAnchor'])
      assert.equal(value[key], null);
    assert.equal(value.closureStatus, 'blocked');
  }
  a.declaration.host.unknownMeaning = 'unlimited'; a.ratios.withinRoundSpread.wall.numerator = '999';
  assert.notDeepEqual(a.declaration, b.declaration);
  assert.equal(b.ratios.withinRoundSpread.wall.numerator, '1');
  assert.throws(() => parseProspectivePolicy(Buffer.from(canonical(b))));
  for (const mutate of [d => { d.sourceAuthenticated = true; }, d => { d.baselineAdmitted = true; },
    d => { d.thresholdAuthority = { approved: true }; }, d => { d.numericTableAnchor = 'self-issued'; },
    d => { d.thresholds = { copied: true }; }, d => { d.closureStatus = 'complete'; }]) rejected(mutate);
});
test('caller expected-hash/approval fields and noncanonical/duplicate/extra input never replace frozen policy', () => {
  assert.throws(() => parseProspectivePolicy(raw, sha256(raw)));
  rejected(d => { d.approvedBy = 'caller'; });
  rejected(d => { d.engineeringBasis.decisionSha256 = 'f'.repeat(64); });
  assert.throws(() => parseProspectivePolicy(Buffer.from(JSON.stringify(fresh(), null, 2))));
  const text = raw.toString('utf8');
  assert.throws(() => parseProspectivePolicy(Buffer.from(text.replace('"baselineAdmitted":false', '"baselineAdmitted":false,"baselineAdmitted":false'))));
  assert.throws(() => parseProspectivePolicy(raw.subarray(0, -1)));
});
test('existing v1/v4 authorities remain frozen and unqualified rather than updated by this policy', () => {
  for (const [path, expected] of [
    ['./stability-gates-v1.json', 'f339c9edba6773d63a252908dbfa094875a5ecb7c66843dd8017142f547e787b'],
    ['./cli-performance-methodology-v4.json', 'b73dee72db94b65fb9e0786c43e807e416e8791bf650da2b5babd4c658294b7b'],
  ]) assert.equal(sha256(readFileSync(new URL(path, import.meta.url))), expected);
  assert.equal(parseProspectivePolicy(raw).declaration.numericTableAnchor, null);
});
