import assert from 'node:assert/strict';
import path from 'node:path';
import { createRequire } from 'node:module';
import { digest, fail, parseCanonical, portablePath, readStableFile } from './registry-files.mjs';
import { expectedLimitContractsSha256, expectedLimitContractSchemaSha256 } from './limit-contract-authority.mjs';

function unique(items, label) {
  const result = new Map();
  for (const item of items) {
    if (result.has(item.id)) fail(`duplicate ${label}`);
    result.set(item.id, item);
  }
  return result;
}
function equal(actual, expected, label) {
  try { assert.deepEqual(actual, expected); } catch { fail(label); }
}
export function loadReviewedLimitContracts(document, root) {
  if (!expectedLimitContractsSha256 || !expectedLimitContractSchemaSha256) fail('limit contract authority not frozen');
  const descriptor = document.contractPlan;
  if (descriptor.path !== 'tests/m4-limit-contracts-v1.json') fail('unexpected contract plan path');
  const bytes = readStableFile(path.join(root, descriptor.path), 256 * 1024, 'limit contracts');
  if (bytes.length !== descriptor.bytes || digest(bytes) !== descriptor.sha256 ||
      digest(bytes) !== expectedLimitContractsSha256) fail('limit contract authority drifted');
  const schema = readStableFile(path.join(root, 'schemas/zryna-m4-limit-contracts-v1.schema.json'), 128 * 1024, 'limit schema');
  if (digest(schema) !== expectedLimitContractSchemaSha256) fail('limit schema authority drifted');
  const require = createRequire(path.join(root, 'package.json'));
  const Ajv2020 = require('ajv/dist/2020.js');
  const validate = new Ajv2020({ strict: true, allErrors: false }).compile(parseCanonical(schema));
  const plan = parseCanonical(bytes);
  if (!validate(plan)) fail('closed limit contract schema rejected');
  return plan;
}
function bindings(items, sources) {
  const seen = new Set();
  for (const binding of items) {
    portablePath(binding.path);
    if (!sources.has(binding.path)) fail('owning guard source descriptor missing');
    const key = `${binding.path}\0${binding.symbol}`;
    if (seen.has(key)) fail('duplicate owning guard binding');
    seen.add(key);
  }
}
function referenced(id, row, cases) {
  const entry = cases.get(id);
  if (!entry || !entry.rowIds.includes(row.id) || !row.cases.includes(id) ||
      !row.requiredGates.includes(entry.gateId)) fail('proof case or required gate missing');
  return entry;
}
function pair(proof, contract, row, cases, scopes) {
  const entries = [referenced(proof.exactCase, row, cases), referenced(proof.firstExtraCase, row, cases)];
  if (proof.exactCase === proof.firstExtraCase || scopes.length !== 2) fail('invalid pair references/scopes');
  for (let index = 0; index < 2; index++) {
    const entry = entries[index];
    equal(entry.kind, index ? 'first-extra' : 'exact-limit', 'pair kind mismatch');
    equal(entry.boundary, { limitId: contract.limitId, value: contract.maximum + index,
      unit: contract.unit, decision: index ? 'reject' : 'accept' }, 'pair boundary mismatch');
    equal(entry.proofScope, scopes[index], 'pair owning/public scope mismatch');
  }
}
function exclusion(record, contract, row, cases, sources) {
  if (!record || !contract.allowedExclusionIds.includes(record.id) ||
      !record.rowIds.includes(row.id) || record.limitId !== contract.limitId) fail('unreviewed exclusion');
  bindings(record.sourceBindings, sources);
  for (const id of record.proofCaseIds) referenced(id, row, cases);
  const rules = {
    'native-stack-byte-occupancy-not-observable': ['fixed-enforcement', 'runtime'],
    'fixed-duration-not-selectable-input': ['fixed-enforcement', 'runtime'],
    'fixed-fuel-budget-not-selectable-input': ['fixed-enforcement', 'runtime'],
    'diagnostic-frame-truncation-not-rejection': ['truncate-output', 'runtime'],
    'canonical-public-shape-below-ceiling': ['reject-first-extra', 'runtime'],
    'fixed-public-handler-cannot-exhaust-guard': ['reject-first-extra', 'runtime'],
  };
  equal(rules[record.code], [contract.semantics, contract.domain], 'exclusion semantic mismatch');
  const allowedLimits = {
    'native-stack-byte-occupancy-not-observable': ['wasm-stack'],
    'fixed-duration-not-selectable-input': ['deadline'],
    'fixed-fuel-budget-not-selectable-input': ['fuel'],
    'diagnostic-frame-truncation-not-rejection': ['backtrace-frames'],
    'canonical-public-shape-below-ceiling': ['component-bytes'],
    'fixed-public-handler-cannot-exhaust-guard': ['guest-resources', 'guest-callbacks'],
  };
  if (!allowedLimits[record.code].includes(contract.limitId)) fail('exclusion applied to unrelated limit');
  const h1 = ['command-', 'installed-command-'];
  const server = ['server-status-', 'server-clock-'];
  const families = record.code.startsWith('canonical-') || record.code.startsWith('fixed-public-') ? server : h1;
  if (!families.some(prefix => row.id.startsWith(prefix))) fail('exclusion applied to unrelated profile');
  const ceilings = {
    'native-stack-byte-occupancy-not-observable': 65536,
    'fixed-duration-not-selectable-input': 5000,
    'fixed-fuel-budget-not-selectable-input': 100000,
    'diagnostic-frame-truncation-not-rejection': 1,
    'canonical-public-shape-below-ceiling': 1048576,
  };
  if (record.code in ceilings && contract.maximum !== ceilings[record.code]) fail('excluded advertised ceiling changed');
  if (record.code !== 'canonical-public-shape-below-ceiling') {
    if (record.quantitative !== null) fail('unrelated quantitative exclusion');
    return;
  }
  const q = record.quantitative;
  if (!q || contract.limitId !== 'component-bytes' || contract.unit !== 'bytes') fail('quantitative size exclusion required');
  const baseline = referenced(q.baselineCaseId, row, cases);
  if (baseline.kind !== 'identity' || baseline.boundary !== null ||
      baseline.proofScope !== 'conditional-public-shape-reachability-exclusion' ||
      !record.proofCaseIds.includes(baseline.id) || q.expressionNodes !== 16383 ||
      q.maximumLiteralBytes !== 6 || q.exportOccurrences !== 2 ||
      q.maximumExportBytes !== 128 || q.framingAllowanceBytes !== 256) fail('public shape bound changed');
  const leaves = (q.expressionNodes + 1) / 2;
  const bound = q.baselineMaximumBytes + leaves * q.maximumLiteralBytes + leaves - 1 +
    q.exportOccurrences * q.maximumExportBytes + q.framingAllowanceBytes;
  if (!Number.isSafeInteger(bound) || q.maximumBytes !== bound || bound >= contract.maximum) fail('public shape not strictly below ceiling');
}
// Pure structural core for hostile tests. Production callers must load the independently
// pinned plan above; this function does not authenticate execution or approve exclusions.
export function validateLimitMappings(document, plan) {
  const contracts = unique(plan.contracts, 'limit contract');
  const exclusions = unique(plan.exclusions, 'exclusion');
  const cases = unique(document.cases, 'case');
  const sources = new Set(document.sources.map(item => item.path));
  const used = new Set();
  const excluded = new Set();
  const modeIds = plan.modeRequirements.map(item => item.mode);
  if (new Set(modeIds).size !== modeIds.length) fail('duplicate mode requirement authority');
  equal([...modeIds].sort(), [...new Set(document.rows.map(row => row.mode))].sort(), 'mode requirement inventory mismatch');
  for (const row of document.rows) {
    for (const limit of row.limits) {
      const contract = contracts.get(limit.contractId);
      if (!contract || !contract.rowIds.includes(row.id)) fail('limit contract missing or wrong row');
      for (const key of ['minimum', 'maximum', 'unit']) equal(limit[key], contract[key], 'advertised bound changed');
      equal(limit.id, contract.limitId, 'contract limit identity mismatch');
      if (limit.minimum > limit.maximum || limit.maximum >= Number.MAX_SAFE_INTEGER) fail('invalid bounded limit');
      const domain = row.mode === 'checkout-build' ? 'emission' : row.mode === 'checkout-browser' ? 'loader' : 'runtime';
      if (domain !== contract.domain) fail('emission/loader/runtime domain mismatch');
      bindings(contract.sourceBindings, sources);
      used.add(`${contract.id}\0${row.id}`);
      const mapping = limit.mapping;
      if (mapping.kind === 'unresolved') continue;
      if (mapping.kind === 'excluded') {
        const record = exclusions.get(mapping.exclusionId);
        if (contract.publicRequirement !== 'reviewed-exclusion' ||
            record?.code !== 'canonical-public-shape-below-ceiling' || contract.guardScopes.length) {
          fail('standalone exclusion cannot replace owning enforcement proof');
        }
        equal(mapping.bindingCaseIds, contract.requiredBindingCaseIds, 'source exclusion binding inventory mismatch');
        for (const id of mapping.bindingCaseIds) referenced(id, row, cases);
        exclusion(record, contract, row, cases, sources);
        excluded.add(mapping.exclusionId);
        continue;
      }
      if (mapping.guard.kind === 'pair') {
        if (contract.semantics !== 'reject-first-extra') fail('fixed/truncated policy is not a reject pair');
        pair(mapping.guard, contract, row, cases, contract.guardScopes);
      } else {
        if (contract.semantics === 'reject-first-extra') fail('reject policy lacks owning guard pair');
        for (const id of mapping.guard.caseIds) {
          const entry = referenced(id, row, cases);
          if (!contract.guardScopes.includes(entry.proofScope) || entry.boundary !== null) fail('enforcement proof scope mismatch');
        }
      }
      equal(mapping.bindingCaseIds, contract.requiredBindingCaseIds, 'production binding gate inventory mismatch');
      for (const id of mapping.bindingCaseIds) referenced(id, row, cases);
      if (mapping.kind === 'partial') continue;
      if (mapping.public.kind === 'pair') {
        if (contract.publicRequirement !== 'reachable-pair') fail('public pair not the reviewed contract');
        pair(mapping.public, contract, row, cases, contract.publicScopes);
      } else if (mapping.public.kind === 'exclusion') {
        if (contract.publicRequirement !== 'reviewed-exclusion') fail('exclusion substitutes reachable public boundary');
        exclusion(exclusions.get(mapping.public.exclusionId), contract, row, cases, sources);
        excluded.add(mapping.public.exclusionId);
      } else if (mapping.public.kind === 'guard-prerequisite') {
        if (contract.publicRequirement !== 'guard-prerequisite' ||
            contract.limitId === 'component-bytes' || contract.domain !== 'runtime' ||
            !contract.requiredBindingCaseIds.length) fail('private prerequisite substitutes public artifact boundary');
        equal(mapping.public.caseIds, contract.requiredBindingCaseIds, 'private prerequisite lacks public binding execution');
      } else if (contract.publicRequirement !== 'emission-only' || contract.domain !== 'emission') {
        fail('runtime public boundary silently omitted');
      }
    }
  }
  for (const contract of contracts.values()) {
    for (const rowId of contract.rowIds) if (!used.has(`${contract.id}\0${rowId}`)) fail('contract has unadvertised row/limit');
  }
  for (const id of exclusions.keys()) if (!excluded.has(id)) fail('unused or unresolved exclusion authority');
  return document;
}
export function calculateContractCoverage(document, plan) {
  const blockers = [];
  const modes = new Map(plan.modeRequirements.map(item => [item.mode, item.requiredCaseKinds]));
  for (const row of document.rows) {
    const required = modes.get(row.mode);
    if (!required) fail('mode requirements missing');
    const kinds = new Set(document.cases.filter(entry => entry.rowIds.includes(row.id)).map(entry => entry.kind));
    for (const kind of required) if (!kinds.has(kind)) blockers.push(`${row.id}: missing ${kind} case`);
    for (const limit of row.limits) if (!['mapped', 'excluded'].includes(limit.mapping.kind)) {
      blockers.push(`${row.id}/${limit.id}: ${limit.mapping.requirements.join(',')}`);
    }
  }
  return { contractComplete: blockers.length === 0, executionAuthenticated: false, eligible: false, blockers };
}
