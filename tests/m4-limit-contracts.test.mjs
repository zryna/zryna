import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';
import Ajv2020 from 'ajv/dist/2020.js';
import { fixture, stackFixture, sizeFixture } from './m4-limit-contract-fixtures.mjs';
import { calculateContractCoverage, loadReviewedLimitContracts, validateLimitMappings } from '../scripts/m4/limit-contracts.mjs';
import { pinnedBrowserTests, validateNamedGateCases } from '../scripts/m4/named-gate-cases.mjs';

const schema = JSON.parse(fs.readFileSync(new URL('../schemas/zryna-m4-conformance-v1.schema.json', import.meta.url)));
const limitSchema = schema.properties.rows.items.properties.limits.items;
const validateLimit = new Ajv2020({ strict: true }).compile(limitSchema);
const planSchema = JSON.parse(fs.readFileSync(new URL('../schemas/zryna-m4-limit-contracts-v1.schema.json', import.meta.url)));
const validatePlan = new Ajv2020({ strict: true }).compile(planSchema);
const rejects = (mutate, make = fixture) => {
  const value = make(); mutate(value); assert.throws(() => validateLimitMappings(value.document, value.plan));
};

test('owning policy and reachable public pairs retain distinct scopes', () => {
  const { document, plan } = fixture();
  assert.equal(validateLimit(document.rows[0].limits[0]), true);
  validateLimitMappings(document, plan);
  assert.deepEqual(calculateContractCoverage(document, plan), {
    contractComplete: true, executionAuthenticated: false, eligible: false, blockers: [] });
});
test('complete closed plan permits repeated positional exact and first extra scopes', () => {
  const { plan } = fixture();
  assert.equal(validatePlan(plan), true);
  plan.contracts[0].passed = true;
  assert.equal(validatePlan(plan), false);
});
test('closed mapping rejects waiver and caller supplied passed flags', () => {
  for (const flag of [{ passed: true }, { waiver: 'reviewed' }, { eligible: true }]) {
    const { document } = fixture(); Object.assign(document.rows[0].limits[0].mapping, flag);
    assert.equal(validateLimit(document.rows[0].limits[0]), false);
  }
});
test('unresolved source mapping remains blocked without implying runtime failure', () => {
  const { document, plan } = fixture();
  document.rows[0].limits[0].mapping = { kind: 'unresolved', requirements: ['owning-guard-proof'] };
  validateLimitMappings(document, plan);
  assert.equal(calculateContractCoverage(document, plan).contractComplete, false);
});
test('draft null authority cannot be replaced by caller approval', () => {
  assert.throws(() => loadReviewedLimitContracts({ contractPlan: { passed: true } }, '/unused'), /not frozen|unexpected contract plan path/);
});
test('lowered policy ceiling cannot qualify advertised maximum', () => rejects(({ document }) => {
  document.cases[0].boundary.value = 15;
}));
test('first extra must be exactly maximum plus one', () => rejects(({ document }) => {
  document.cases[1].boundary.value = 18;
}));
test('private pair cannot be relabeled as reachable public input', () => rejects(({ document }) => {
  document.rows[0].limits[0].mapping.public = document.rows[0].limits[0].mapping.guard;
}));
test('proof without required actual gate cannot complete source mapping', () => rejects(({ document }) => {
  document.rows[0].requiredGates.pop();
}));
test('missing unchanged owning guard descriptor is rejected', () => rejects(({ document }) => {
  document.sources = [];
}));
test('wrong row cannot borrow another profile source contract', () => rejects(({ plan }) => {
  plan.contracts[0].rowIds = ['server-status-linux'];
}));
test('truncation cannot be certified as first extra rejection', () => rejects(({ plan }) => {
  plan.contracts[0].semantics = 'truncate-output';
}));
test('precisely reviewed native byte exclusion maps enforcement only', () => {
  const { document, plan } = stackFixture();
  validateLimitMappings(document, plan);
  assert.equal(calculateContractCoverage(document, plan).contractComplete, true);
  assert.equal(calculateContractCoverage(document, plan).executionAuthenticated, false);
});
test('unreviewed native exclusion cannot suppress public requirement', () => rejects(({ plan }) => {
  plan.contracts[0].allowedExclusionIds = [];
}, stackFixture));
test('native stack exclusion cannot waive unrelated count bound', () => rejects(({ document, plan }) => {
  document.rows[0].limits[0].id = plan.contracts[0].limitId = plan.exclusions[0].limitId = 'instances';
}, stackFixture));
test('enforcement proof cannot carry fabricated exact byte observation', () => rejects(({ document }) => {
  document.cases.find(item => item.id === 'stack-trap').boundary = { limitId: 'wasm-stack', value: 65536, unit: 'bytes', decision: 'accept' };
}, stackFixture));
test('emission only row cannot inherit runtime policy requirements', () => rejects(({ document }) => {
  document.rows[0].mode = 'checkout-build';
}));
test('private prerequisite cannot substitute public component size acceptance', () => rejects(({ document, plan }) => {
  const contract = plan.contracts[0]; contract.limitId = 'component-bytes'; contract.publicRequirement = 'guard-prerequisite';
  document.rows[0].limits[0].id = 'component-bytes';
  for (const item of document.cases) if (item.boundary) item.boundary.limitId = 'component-bytes';
  document.rows[0].limits[0].mapping.public = { kind: 'guard-prerequisite', caseIds: ['binding'] };
}));
test('private H1 component guard remains partial while public exact size is unproved', () => {
  const { document, plan } = fixture();
  plan.contracts[0].limitId = document.rows[0].limits[0].id = 'component-bytes';
  for (const item of document.cases) if (item.boundary) item.boundary.limitId = 'component-bytes';
  const mapping = document.rows[0].limits[0].mapping;
  document.rows[0].limits[0].mapping = { kind: 'partial', guard: mapping.guard,
    bindingCaseIds: ['binding'], requirements: ['reachable-public-boundary'] };
  assert.equal(validateLimit(document.rows[0].limits[0]), true);
  validateLimitMappings(document, plan);
  assert.equal(calculateContractCoverage(document, plan).contractComplete, false);
});
test('specified private guard prerequisite retains required public binding gate', () => {
  const { document, plan } = fixture();
  plan.contracts[0].publicRequirement = 'guard-prerequisite';
  document.rows[0].limits[0].mapping.public = { kind: 'guard-prerequisite', caseIds: ['binding'] };
  validateLimitMappings(document, plan);
});
test('quantitative exclusion recomputes the full conservative canonical shape bound', () => {
  const { document, plan } = sizeFixture();
  validateLimitMappings(document, plan);
  assert.equal(calculateContractCoverage(document, plan).executionAuthenticated, false);
});
test('size exclusion cannot use a bound equal to its advertised ceiling', () => rejects(({ plan }) => {
  const q = plan.exclusions[0].quantitative;
  q.baselineMaximumBytes = 1048576 - 57855; q.maximumBytes = 1048576;
}, sizeFixture));
test('changed quantitative expression or export framing is rejected', () => rejects(({ plan }) => {
  plan.exclusions[0].quantitative.maximumExportBytes = 127;
}, sizeFixture));
test('quantitative exclusion cannot borrow an unregistered baseline proof', () => rejects(({ plan }) => {
  plan.exclusions[0].quantitative.baselineCaseId = 'unknown';
}, sizeFixture));
test('public source reachability exclusion needs no fabricated accepted ceiling pair', () => {
  const { document, plan } = sizeFixture();
  plan.contracts[0].guardScopes = [];
  document.rows[0].limits[0].mapping = { kind: 'excluded', exclusionId: 'public-size', bindingCaseIds: ['binding'] };
  assert.equal(validatePlan(plan), true);
  assert.equal(validateLimit(document.rows[0].limits[0]), true);
  validateLimitMappings(document, plan);
  assert.equal(calculateContractCoverage(document, plan).contractComplete, true);
  assert.equal(calculateContractCoverage(document, plan).executionAuthenticated, false);
});
test('standalone exclusion cannot remove required native enforcement proof', () => rejects(({ document, plan }) => {
  plan.contracts[0].guardScopes = [];
  document.rows[0].limits[0].mapping = { kind: 'excluded', exclusionId: 'native-byte-oracle', bindingCaseIds: ['binding'] };
}, stackFixture));
test('mode requirement override cannot remove advertised case kinds', () => rejects(({ plan }) => {
  plan.modeRequirements.push({ mode: 'checkout-run', requiredCaseKinds: ['positive'] });
}));

function namedFixture() {
  const producer = { id: 'pinned-browser-three-proof', source: 'scripts/scalar-host/run-conformance.mjs',
    accounting: 'exact-three-proof-pinned-browser-receipt', argv: ['node', 'scripts/scalar-host/run-conformance.mjs', '$ABSOLUTE_CREATE_ONLY_BROWSER_EVIDENCE_DIRECTORY'] };
  const cases = pinnedBrowserTests.map(([pkg, target, selector, source], index) => ({ id: `browser-${index}`,
    gateId: producer.id, test: { runner: 'rust', package: pkg, target, selector, source, ignoredByDefault: true } }));
  return { cases, gates: [producer] };
}
test('pinned browser identity tuple cannot be changed in process', () => {
  assert.throws(() => { pinnedBrowserTests[0][0] = 'substituted-package'; }, TypeError);
});
test('multiple cases sharing browser selector still require exactly three unique proofs', () => {
  const document = namedFixture(); document.cases.push({ ...structuredClone(document.cases[0]), id: 'deadline-extra' });
  assert.equal(validateNamedGateCases(document).browserProofs.length, 3);
});
test('three browser records duplicating one selector do not qualify three proofs', () => {
  const document = namedFixture(); document.cases = [document.cases[0], document.cases[0], document.cases[0]];
  assert.throws(() => validateNamedGateCases(document));
});
test('browser scalar or retained transport substitution is rejected', () => {
  const document = namedFixture(); document.cases[2].test.selector = 'component_only';
  assert.throws(() => validateNamedGateCases(document));
});
test('repeated same selector cannot change source identity or ignored accounting', () => {
  const document = namedFixture(); const extra = structuredClone(document.cases[0]);
  extra.test.source = 'substituted.rs'; document.cases.push(extra);
  assert.throws(() => validateNamedGateCases(document));
});
test('ordinary repeated exact selector maps to one exact gate', () => {
  const document = namedFixture();
  const test = { runner: 'rust', package: 'p', target: 'lib', selector: 'one', source: 'one.rs', ignoredByDefault: false };
  document.gates.push({ id: 'one-gate', source: 'one.rs', accounting: 'one-exact-passing-nonignored-rust-test',
    argv: ['cargo', 'test', '--locked', '-p', 'p', '--lib', 'one', '--', '--exact', '--nocapture'] });
  document.cases.push({ id: 'exact', gateId: 'one-gate', test }, { id: 'extra', gateId: 'one-gate', test: { ...test } });
  assert.equal(validateNamedGateCases(document).ordinaryProofs.length, 1);
});
test('ordinary gate cannot disguise wrong Rust package or target in argv', () => {
  const document = namedFixture();
  document.gates.push({ id: 'bad', source: 'one.rs', accounting: 'one-exact-passing-nonignored-rust-test',
    argv: ['cargo', 'test', '--locked', '-p', 'other', '--lib', 'one', '--', '--exact', '--nocapture'] });
  document.cases.push({ id: 'one', gateId: 'bad', test: { runner: 'rust', package: 'p', target: 'lib', selector: 'one', source: 'one.rs', ignoredByDefault: false } });
  assert.throws(() => validateNamedGateCases(document));
});
test('ordinary exact gate cannot name a different owning source', () => {
  const document = namedFixture();
  document.gates.push({ id: 'bad-source', source: 'substituted.rs', accounting: 'one-exact-passing-nonignored-rust-test',
    argv: ['cargo', 'test', '--locked', '-p', 'p', '--lib', 'one', '--', '--exact', '--nocapture'] });
  document.cases.push({ id: 'one', gateId: 'bad-source', test: { runner: 'rust', package: 'p', target: 'lib', selector: 'one', source: 'one.rs', ignoredByDefault: false } });
  assert.throws(() => validateNamedGateCases(document));
});
