// Synthetic hostile-validator fixtures only: these describe no compiler execution result.
export function fixture() {
  const rowId = 'command-linux';
  const limitId = 'memory-bytes';
  const entry = (id, kind, value, scope) => ({ id, rowIds: [rowId], kind, gateId: `gate-${id}`,
    proofScope: scope, boundary: value === null ? null : { limitId, value, unit: 'bytes',
      decision: kind === 'exact-limit' ? 'accept' : 'reject' } });
  const cases = [entry('guard-exact', 'exact-limit', 16, 'private-policy'),
    entry('guard-extra', 'first-extra', 17, 'private-policy'),
    entry('public-exact', 'exact-limit', 16, 'public-input'),
    entry('public-extra', 'first-extra', 17, 'public-input'),
    entry('binding', 'positive', null, 'public-execution')];
  const mapping = { kind: 'mapped', guard: { kind: 'pair', exactCase: 'guard-exact', firstExtraCase: 'guard-extra' },
    public: { kind: 'pair', exactCase: 'public-exact', firstExtraCase: 'public-extra' }, bindingCaseIds: ['binding'] };
  const document = { sources: [{ path: 'guard.rs' }], cases, rows: [{ id: rowId, mode: 'checkout-run',
    cases: cases.map(item => item.id), requiredGates: cases.map(item => item.gateId),
    limits: [{ id: limitId, minimum: 0, maximum: 16, unit: 'bytes', contractId: 'memory-policy', mapping }] }] };
  const contract = { id: 'memory-policy', rowIds: [rowId], limitId, minimum: 0, maximum: 16, unit: 'bytes',
    semantics: 'reject-first-extra', domain: 'runtime', sourceBindings: [{ path: 'guard.rs', symbol: 'Guard::check' }],
    guardScopes: ['private-policy', 'private-policy'], publicRequirement: 'reachable-pair',
    publicScopes: ['public-input', 'public-input'], allowedExclusionIds: [], requiredBindingCaseIds: ['binding'] };
  return { document, plan: { format: 'zryna.m4-limit-contracts.v1', contracts: [contract], exclusions: [],
    modeRequirements: [{ mode: 'checkout-run', requiredCaseKinds: ['positive', 'exact-limit', 'first-extra'] }] } };
}
export function stackFixture() {
  const value = fixture();
  const { document, plan } = value;
  const contract = plan.contracts[0];
  Object.assign(contract, { limitId: 'wasm-stack', maximum: 65536, semantics: 'fixed-enforcement',
    guardScopes: ['private-stack-trap'], publicRequirement: 'reviewed-exclusion', publicScopes: [],
    allowedExclusionIds: ['native-byte-oracle'] });
  Object.assign(document.rows[0].limits[0], { id: 'wasm-stack', maximum: 65536 });
  document.cases = document.cases.filter(item => item.id === 'binding');
  document.cases.push({ id: 'stack-trap', rowIds: ['command-linux'], kind: 'lifecycle', gateId: 'gate-stack',
    proofScope: 'private-stack-trap', boundary: null });
  document.rows[0].cases = document.cases.map(item => item.id);
  document.rows[0].requiredGates = document.cases.map(item => item.gateId);
  document.rows[0].limits[0].mapping = { kind: 'mapped', guard: { kind: 'enforcement', caseIds: ['stack-trap'] },
    public: { kind: 'exclusion', exclusionId: 'native-byte-oracle' }, bindingCaseIds: ['binding'] };
  plan.exclusions.push({ id: 'native-byte-oracle', code: 'native-stack-byte-occupancy-not-observable',
    rowIds: ['command-linux'], limitId: 'wasm-stack', sourceBindings: contract.sourceBindings,
    proofCaseIds: ['stack-trap'], quantitative: null });
  plan.modeRequirements[0].requiredCaseKinds = ['positive', 'lifecycle'];
  return value;
}
export function sizeFixture() {
  const { document, plan } = fixture();
  const row = document.rows[0];
  row.id = 'server-status-linux'; row.mode = 'checkout-serve';
  const contract = plan.contracts[0];
  Object.assign(contract, { rowIds: [row.id], limitId: 'component-bytes', maximum: 1048576,
    publicRequirement: 'reviewed-exclusion', publicScopes: [], allowedExclusionIds: ['public-size'] });
  Object.assign(row.limits[0], { id: 'component-bytes', maximum: 1048576 });
  for (const entry of document.cases) {
    entry.rowIds = [row.id];
    if (entry.boundary) { entry.boundary.limitId = 'component-bytes'; entry.boundary.value += 1048560; }
  }
  row.limits[0].mapping.public = { kind: 'exclusion', exclusionId: 'public-size' };
  document.cases.push({ id: 'size-proof', rowIds: [row.id], kind: 'identity', gateId: 'size-proof-gate',
    proofScope: 'conditional-public-shape-reachability-exclusion', boundary: null });
  row.cases.push('size-proof'); row.requiredGates.push('size-proof-gate');
  const quantitative = { kind: 'canonical-single-scalar-component', baselineCaseId: 'size-proof',
    baselineMaximumBytes: 2000, expressionNodes: 16383, maximumLiteralBytes: 6,
    exportOccurrences: 2, maximumExportBytes: 128, framingAllowanceBytes: 256, maximumBytes: 59855 };
  plan.exclusions = [{ id: 'public-size', code: 'canonical-public-shape-below-ceiling',
    rowIds: [row.id], limitId: 'component-bytes', sourceBindings: contract.sourceBindings,
    proofCaseIds: ['size-proof'], quantitative }];
  plan.modeRequirements = [{ mode: row.mode, requiredCaseKinds: ['positive', 'exact-limit', 'first-extra'] }];
  return { document, plan };
}
