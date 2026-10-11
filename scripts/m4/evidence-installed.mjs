import assert from 'node:assert/strict';
import path from 'node:path';
import { verifyArchive } from '../distribution/verify.mjs';
import { validateSourceBuildReceiptText } from '../distribution-release/validate-source-build-receipt.mjs';
import { MAX_ENVELOPE_BYTES } from '../distribution-release/canonical.mjs';
import { digest } from './registry-files.mjs';
import { installedNames, positive, invocationArgv } from './installed-cases.mjs';
import { producedJson, validateInstalledObservation } from './installed-observation.mjs';
import { installedByteCases, validateInstalledByteSource, validateInstalledByteDenial } from './installed-byte-boundary.mjs';
export { installedNames } from './installed-cases.mjs';
import { attachmentMap, array, exact, integer, MAX_ARTIFACT, retained, retainedJson, reject, sha, text, utf8 } from './evidence-input.mjs';

export function readInstalledArchitectureReceipt(descriptor, root) {
  // Preserve the distribution producer's sorted compact wire and original digest.
  const bytes = retained(descriptor, root, MAX_ENVELOPE_BYTES);
  const text = utf8(bytes);
  assert(Buffer.from(text, 'utf8').equals(bytes), 'original native UTF-8 byte identity');
  return validateSourceBuildReceiptText(text);
}

export async function installedAccounting(result, { root, sourceRoot, source, host, evidenceDirectory, tools, registry }) {
  const files = attachmentMap(result);
  const get = name => {
    const descriptor = files.get(`installed/${name}`);
    if (!descriptor) reject(`installed ${name} missing`);
    return descriptor;
  };
  const report = retainedJson(get('qualification.json'), root);
  const candidate = retainedJson(get('candidate.json'), root);
  const invocations = retainedJson(get('invocations.json'), root);
  const architecture = readInstalledArchitectureReceipt(get('architecture-receipt.json'), root);
  const paths = host.os === 'windows' ? path.win32 : path.posix;
  array(invocations, 31, 'installed invocation inventory');
  assert.deepEqual(invocations.map(item => item.name), installedNames, 'all 31 real invocation identities');
  assert.equal(report.passed, 31, 'qualification includes every actual invocation');
  assert.equal(report.failed, 0);
  assert.deepEqual(report.rows, invocations.map(item => ({ name: item.name, exitCode: item.exitCode,
    ...(positive.has(item.name) ? { manifestSha256: item.manifestSha256, componentSha256: item.componentSha256 } : {}) })),
  'qualification summaries bind the actual invocation inventory');
  assert.deepEqual(candidate.observedSource, { head: source.commit, tree: source.tree,
    branch: report.observedSource.branch }, 'installed observed source');
  assert.deepEqual(report.observedSource, candidate.observedSource);
  assert.equal(architecture.source.commit, source.commit);
  assert.equal(architecture.source.tree, source.tree);
  assert.equal(candidate.productionAdmission, 'forbidden');
  assert.equal(candidate.signatureAuthentication, 'not-performed');
  assert.equal(report.platform, host.os === 'windows' ? 'win32' : 'linux');
  assert.equal(report.node, 'v22.22.1');
  // Added observation must retain the real runtime path used for --node denial.
  text(report.nodeExecutable, 'actual installed fixture Node executable', 2048);
  assert.equal(report.nodeExecutable, tools[0].executable.path, 'actual fixture uses observed pinned Node');
  text(report.sourceDirectory, 'actual original checkout directory', 2048);
  for (const location of ['installation', 'project', 'cwd', 'invocationPath']) text(report[location], location, 2048);
  assert.equal(paths.dirname(report.invocationPath), paths.join(report.installation, 'bin'));
  assert.equal(report.installation, paths.join(evidenceDirectory, 'installed', 'relocated-install'));
  assert.equal(report.project, paths.join(evidenceDirectory, 'installed', 'ordinary-user-project'));
  assert.equal(report.cwd, paths.join(evidenceDirectory, 'installed', 'unrelated-cwd'));
  const outside = paths.relative(report.sourceDirectory, paths.dirname(report.installation));
  assert(outside && (paths.isAbsolute(outside) || outside.split(paths.sep)[0] === '..'), 'actual installation outside original checkout');
  const archiveExpected = candidate.archive;
  assert.equal(archiveExpected.target.triple, host.os === 'windows' ? 'x86_64-pc-windows-msvc' : 'x86_64-unknown-linux-gnu');
  oidSource(archiveExpected.source, source);
  const archive = retained(get(archiveExpected.filename), root, MAX_ARTIFACT);
  const verified = await verifyArchive(archive, archiveExpected, { productionCandidate: true });
  assert.equal(verified.archiveSha256, digest(archive), 'complete installed archive verification');
  for (const item of invocations) {
    const oracle = positive.get(item.name);
    const keys = ['name', 'arguments', 'exitCode', 'bundleExists', 'stdout', 'stderr'];
    if (oracle) keys.push('manifestSha256', 'componentSha256');
    if (item.name === 'existing-output-preserved') keys.push('preservedManifestSha256');
    if (installedByteCases[item.name]) keys.push('source');
    exact(item, keys, 'real installed invocation');
    integer(item.exitCode, 0, 255, 'installed exitCode');
    array(item.arguments, 40, 'installed argv');
    assert.deepEqual(item.arguments, invocationArgv(item.name, report, paths), 'exact actual installed argv');
    if (typeof item.stdout !== 'string' || typeof item.stderr !== 'string' ||
        Buffer.byteLength(item.stdout) + Buffer.byteLength(item.stderr) > 2 * 1024 * 1024) reject('installed raw logs missing or over bound');
    if (installedByteCases[item.name]) {
      const bytes = retained(get(`boundary-source/${installedByteCases[item.name].file}`), root, 256 * 1024);
      validateInstalledByteSource(item, bytes, sourceRoot);
    }
    if (!oracle) {
      if (!item.exitCode) reject('installed denial did not reject');
      assert.equal(item.bundleExists, item.name === 'existing-output-preserved', 'denied invocation bundle state');
      if (item.name.endsWith('-tamper')) assert(item.stdout.includes('ZRYNA-C4220'), 'actual tamper diagnostic');
      if (item.name === 'component-bytes-first-extra') validateInstalledByteDenial(item, producedJson(Buffer.from(item.stdout), true));
      if (item.name === 'existing-output-preserved') {
        sha(item.preservedManifestSha256);
        assert.equal(item.preservedManifestSha256, invocations[0].manifestSha256, 'existing output retained exact manifest');
      }
      continue;
    }
    assert.equal(item.exitCode, oracle[0] === 'ok' ? 0 : 5);
    assert.equal(item.bundleExists, true);
    const response = producedJson(Buffer.from(item.stdout), true);
    assert.equal(response.profile, 'command-h1-v1');
    assert.equal(response.execution.outcome.runReturn, oracle[0]);
    assert.equal(response.execution.teardown, 'confirmed');
    const manifestDescriptor = get(`${item.name}/manifest.json`);
    const manifestBytes = retained(manifestDescriptor, root, 16384);
    assert.equal(digest(manifestBytes), item.manifestSha256);
    const manifest = producedJson(manifestBytes);
    assert.equal(manifest.schema, 'zryna.wasi-command-manifest.v1');
    assert.equal(manifest.execution.runReturn, oracle[0]);
    assert.equal(manifest.input.kind, oracle[1]);
    assert.equal(manifest.teardown, 'confirmed');
    assert.equal(manifest.grants.effective.length, oracle[1] === 'none' ? 0 : 1);
    const componentBytes = retained(get(`${item.name}/component.wasm`), root, MAX_ARTIFACT);
    assert.equal(digest(componentBytes), item.componentSha256);
    assert.equal(digest(componentBytes), manifest.component.sha256);
    validateInstalledObservation(manifest, response, { item, oracle, componentBytes, sourceRoot, registry });
    for (const value of ['test-only-secret-value', 'ambient-value-never-admitted']) {
      assert(!manifestBytes.includes(Buffer.from(value)) && !item.stdout.includes(value), 'private value retained in public output');
    }
  }
  const allowed = new Set(['qualification.json', 'candidate.json', 'invocations.json', 'architecture-receipt.json',
    ...Object.values(installedByteCases).map(item => `boundary-source/${item.file}`),
    archiveExpected.filename, ...[...positive.keys()].flatMap(name => [`${name}/manifest.json`, `${name}/component.wasm`])]
    .map(name => `installed/${name}`));
  assert.deepEqual([...files.keys()].sort(), [...allowed].sort(), 'installed complete retained file inventory');
  return installedNames;
}
function oidSource(actual, expected) {
  assert.equal(actual.commit, expected.commit, 'archive source commit');
  assert.equal(actual.tree, expected.tree, 'archive source tree');
  assert.equal(actual.ref, 'refs/heads/main', 'intended content ref does not establish admission');
}
