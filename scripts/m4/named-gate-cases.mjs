import assert from 'node:assert/strict';
import { fail } from './registry-files.mjs';

export const pinnedBrowserTests = Object.freeze([
  ['zryna', 'cli', 'browser_component_pinned_real_browser_and_node_agree', 'apps/zryna/tests/cli.rs'],
  ['zryna-driver', 'lib', 'scalar_adapter_interface::tests::host_consumers::pinned_real_browser_and_node_execute_the_same_sealed_scalar_corpus', 'crates/zryna-driver/src/scalar_adapter_interface/tests/host_consumers.rs'],
  ['zryna-driver', 'lib', 'scalar_adapter_interface::tests::host_consumers::retained_esm_transport_accepts_exact_ceiling_and_rejects_first_extra', 'crates/zryna-driver/src/scalar_adapter_interface/tests/host_consumers.rs'],
].map(tuple => Object.freeze(tuple)));
function identity(test) { return JSON.stringify([test.package, test.target, test.selector, test.source]); }
function equal(actual, expected, label) {
  try { assert.deepEqual(actual, expected); } catch { fail(label); }
}
export function validateNamedGateCases(document) {
  const gates = new Map(document.gates.map(gate => [gate.id, gate]));
  const selectors = new Map();
  const gateProofs = new Map();
  const browser = new Set();
  for (const entry of document.cases) {
    if (entry.test.runner !== 'rust') continue;
    const gate = gates.get(entry.gateId);
    if (!gate) fail('named test gate missing');
    const test = entry.test;
    const selectorKey = JSON.stringify([test.package, test.target, test.selector]);
    const observed = selectors.get(selectorKey);
    if (observed) equal(observed, { gateId: entry.gateId, test }, 'same selector changed gate/source/ignored identity');
    selectors.set(selectorKey, { gateId: entry.gateId, test });
    if (entry.gateId === 'pinned-browser-three-proof') {
      if (!test.ignoredByDefault || gate.accounting !== 'exact-three-proof-pinned-browser-receipt') fail('browser ignored proof accounting missing');
      equal(gate.argv, ['node', 'scripts/scalar-host/run-conformance.mjs',
        '$ABSOLUTE_CREATE_ONLY_BROWSER_EVIDENCE_DIRECTORY'], 'pinned browser producer changed');
      equal(gate.source, 'scripts/scalar-host/run-conformance.mjs', 'pinned browser producer source changed');
      browser.add(identity(test));
      continue;
    }
    if (test.ignoredByDefault || gate.accounting !== 'one-exact-passing-nonignored-rust-test') fail('ordinary exact gate accounting mismatch');
    equal(gate.source, test.source, 'exact Rust gate owning source mismatch');
    const argv = ['cargo', 'test', '--locked', '-p', test.package,
      ...(test.target === 'lib' ? ['--lib'] : ['--test', test.target]),
      test.selector, '--', '--exact', '--nocapture'];
    equal(gate.argv, argv, 'exact Rust package/target/selector argv mismatch');
    const prior = gateProofs.get(entry.gateId);
    if (prior && prior !== identity(test)) fail('ordinary gate contains multiple distinct test proofs');
    gateProofs.set(entry.gateId, identity(test));
  }
  equal([...browser].sort(), pinnedBrowserTests.map(test => JSON.stringify(test)).sort(), 'three unique browser proof identities missing/substituted');
  return { ordinaryProofs: [...gateProofs].map(([gateId, proof]) => ({ gateId, proof })),
    browserProofs: [...browser].sort() };
}
