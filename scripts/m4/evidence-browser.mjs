import assert from 'node:assert/strict';
import path from 'node:path';
import { readStableFile, parseCanonical, digest } from './registry-files.mjs';
import { attachmentMap, MAX_ARTIFACT, MAX_LOG, retained, retainedJson, reject, utf8 } from './evidence-input.mjs';
import { rustAccounting } from './evidence-counts.mjs';
import { browserByteBoundaryAccounting } from './evidence-browser-byte-boundary.mjs';

export const browserProofs = Object.freeze([
  { label: 'component', package: 'zryna', target: ['--test', 'cli'],
    selector: 'browser_component_pinned_real_browser_and_node_agree' },
  { label: 'scalar', package: 'zryna-driver', target: ['--lib'],
    selector: 'scalar_adapter_interface::tests::host_consumers::pinned_real_browser_and_node_execute_the_same_sealed_scalar_corpus' },
  { label: 'transport', package: 'zryna-driver', target: ['--lib'],
    selector: 'scalar_adapter_interface::tests::host_consumers::retained_esm_transport_accepts_exact_ceiling_and_rejects_first_extra' },
]);
export const browserArgv = proof => ['cargo', 'test', '--locked', '-p', proof.package,
  ...proof.target, proof.selector, '--', '--ignored', '--exact', '--nocapture'];
export function browserAccounting(result, { root, sourceRoot, source, host }) {
  const files = attachmentMap(result);
  const get = name => {
    const file = files.get(`browser/${name}`);
    if (!file) reject(`browser ${name} missing`);
    return file;
  };
  const report = retainedJson(get('conformance.json'), root);
  const acquisition = retainedJson(get('acquisition.json'), root);
  const inventory = retained(get('inventory.json'), root);
  const pin = parseCanonical(readStableFile(path.join(sourceRoot, 'tests/scalar-host/browser-pin.json'), 64 * 1024));
  const platform = host.os === 'windows' ? 'win32-x64' : 'linux-x64';
  const selected = pin.platforms[platform];
  if (!selected) reject('unsupported browser host');
  assert.equal(report.commit, source.commit, 'browser actual checked-out source');
  assert.equal(report.host, platform, 'browser actual host');
  assert.equal(report.node, 'v22.22.1');
  assert.equal(report.browserVersion, pin.browserVersion);
  assert.equal(report.runnerVersion, pin.runnerVersion);
  assert.equal(report.archiveSha256, selected.archiveSha256);
  assert.equal(report.inventorySha256, selected.inventorySha256);
  assert.equal(acquisition.url, selected.url);
  assert.equal(acquisition.finalUrl, selected.publisherUrl);
  assert.equal(acquisition.archiveSha256, selected.archiveSha256);
  assert.equal(acquisition.inventorySha256, selected.inventorySha256);
  assert.equal(digest(inventory), selected.inventorySha256, 'retained complete browser inventory');
  const archive = retained(get('archive.zip'), root, MAX_ARTIFACT);
  assert.equal(digest(archive), selected.archiveSha256, 'actual pinned browser archive');
  assert.deepEqual(report.proofs.map(proof => proof.test), browserProofs.map(proof => proof.selector));
  for (const [index, proof] of browserProofs.entries()) {
    // Producer summary flags/counts are intentionally not authority.
    assert.deepEqual(report.proofs[index].command, browserArgv(proof), 'actual browser proof argv');
    const stdout = utf8(retained(get(`${proof.label}.stdout.log`), root, MAX_LOG)).replace(/\r\n/g, '\n');
    rustAccounting(stdout, proof.selector);
    if (proof.label === 'component') browserByteBoundaryAccounting(stdout);
    retained(get(`${proof.label}.stderr.log`), root, MAX_LOG);
  }
  for (const phase of ['identity', 'final-identity']) {
    assert.equal(utf8(retained(get(`${phase}.stdout.log`), root)).trim(), source.commit, 'browser before/after commit');
    retained(get(`${phase}.stderr.log`), root);
  }
  for (const phase of ['working-tree', 'final-working-tree']) {
    assert.equal(utf8(retained(get(`${phase}.stdout.log`), root)), '', 'browser before/after clean');
    retained(get(`${phase}.stderr.log`), root);
  }
  const allowed = new Set(['conformance.json', 'acquisition.json', 'inventory.json', 'archive.zip',
    ...browserProofs.flatMap(proof => [`${proof.label}.stdout.log`, `${proof.label}.stderr.log`]),
    ...['identity', 'final-identity', 'working-tree', 'final-working-tree', 'acquisition']
      .flatMap(label => [`${label}.stdout.log`, `${label}.stderr.log`])].map(name => `browser/${name}`));
  for (const descriptor of files.values()) {
    if (!allowed.has(descriptor.path)) reject('unexpected browser retained input');
    retained(descriptor, root, descriptor.path.endsWith('.zip') ? MAX_ARTIFACT : MAX_LOG);
  }
  return browserProofs.map(proof => proof.selector);
}
