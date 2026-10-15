import assert from 'node:assert/strict';
import { exact, reject } from './evidence-input.mjs';
import { rustAccounting } from './evidence-counts.mjs';

const marker = 'browserByteBoundary=';
const selector = 'browser_component_pinned_real_browser_and_node_agree';
// Observations of the ordinary source pair and canonical factory, separately
// checked by source review; these identities do not authenticate execution.
const identities = Object.freeze({
  schema: 'zryna.browser-byte-boundary-observation.v1',
  sourceSha256: 'ef631a1b99fbf55bff698e50c0bd336f1ba32b1ead1f44c9db2f047e3534ecc8',
  firstExtraSourceSha256: '4a7955dc78e5a66ded0b11671914993e4ac20c151f1909c4db0399a74dc1e306',
  sourceBytes: 309364,
  firstExtraSourceBytes: 309365,
  functions: 1863,
  world: 'zryna:capability-profiles/browser@0.1.0',
  witSourceSha256: '72e017162d5080960589e749c8b5a4f662e5f604bf317e7f7adcd085246b3a24',
  coreSha256: '5877a25cc493884b22885d1e1a3a68c2d171ce80ee4ae0e4cd7e1e6e48590b8a',
  interfaceSha256: '3557a78a99ec1cf774ee360c879f0484de5592b2d76bbb59fec8616015969c05',
  componentBytes: 1048576,
  componentSha256: '128b13f47a9195a2e927bf6cdea34588a7194656ad8b265b8e346c8c43d93c97',
  firstExtraComponentBytes: 1048577,
  firstExtraComponentSha256: 'c8345ef73d90facd7f49cbe520588e146c830cd034430504e448a1c29dd90cd6',
});
const factory = Object.freeze({ exactAccepted: true, replayIdentical: true,
  firstExtraExitCode: 4, firstExtraRejection: 'ZRYNA-W4021',
  firstExtraBundleAbsent: true, ordinaryRecovery: true });
const execution = Object.freeze({ executedExports: 1863, wideReturns: 106,
  narrowReturns: 1757, firstExtraRejection: 'ZRYNA-B3991', ordinaryRecovery: 42 });

// The caller authenticates retained bytes, source, browser acquisition, process
// and hosted provenance before composing a qualification.
export function browserByteBoundaryAccounting(output) {
  rustAccounting(output, selector);
  const lines = output.split('\n');
  const records = lines.map((line, index) => ({ line, index }))
    .filter(item => item.line.includes('browserByteBoundary'));
  assert.equal(records.length, 1, 'one actual public browser byte-boundary observation');
  const { line, index } = records[0];
  if (!line.startsWith(marker) || line.length > 8192 ||
      index <= lines.indexOf('running 1 test') ||
      index >= lines.indexOf(`test ${selector} ... ok`)) {
    reject('browser byte-boundary observation outside selected passing test');
  }
  const payload = line.slice(marker.length);
  let record;
  try { record = JSON.parse(payload); } catch { reject('browser byte-boundary JSON'); }
  exact(record, [...Object.keys(identities), 'factory', 'node', 'browser', 'cleanup'], 'browser byte-boundary record');
  assert.equal(JSON.stringify(record), payload, 'compact unambiguous browser observation');
  for (const [key, expected] of Object.entries(identities)) {
    assert.equal(record[key], expected, `genuine browser source/factory ${key}`);
  }
  assert.deepEqual(record.factory, factory, 'real frontend factory acceptance, denial and recovery');
  assert.deepEqual(record.node, execution, 'all source exports execute in Node');
  assert.deepEqual(record.browser, execution, 'all source exports execute in pinned Chrome');
  assert.deepEqual(record.cleanup, { browserConfirmed: true, childExitCode: 0 }, 'confirmed browser child cleanup');
  return record;
}
