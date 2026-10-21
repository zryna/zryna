// Actual installed Wasmtime command runs; no Node/bootstrap provider is selected.
import assert from 'node:assert/strict';
import { existsSync, mkdirSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { sha256 } from '../../scripts/distribution/canonical.mjs';
import { ordinaryBytes, saveEvidence, successful } from './commands.mjs';

// Exact registered examples/wasi-command/pure.zry program, checked by the focused test.
export const PURE_H1_SOURCE = 'export function main(): bool {\n  return true;\n}\n';
const STEM = 'pure-h1';
const MANIFEST = `.zryna/out/${STEM}.wasi-command-run/zryna-wasi-command-manifest-v1.json`;

export function runInstalledPureCommandH1({ projects, output, invoke }) {
  const source = Buffer.from(PURE_H1_SOURCE), observations = [];
  let first;
  for (const provider of ['native', null]) {
    const label = provider ?? 'default', project = join(projects, 'command-h1-' + label);
    mkdirSync(project, { mode: 0o700 }); writeFileSync(join(project, 'pure.zry'), source, { flag: 'wx' });
    const selected = provider ? ['--provider', provider] : [];
    const response = JSON.parse(successful(invoke('command-h1-' + label,
      ['run', 'pure.zry', '--project-root', project, '--target', 'wasi-command',
        '--profile', 'command-h1-v1', '--export', 'main', '--name', STEM, '--json', ...selected])));
    assert.equal(response.ok, true); assert.equal(response.version, 1);
    assert.equal(response.command, 'run'); assert.equal(response.profile, 'command-h1-v1');
    assert.equal(response.target, 'wasi-command'); assert.equal(response.manifest, MANIFEST);
    assert.deepEqual(response.diagnostics, []);
    assert.deepEqual(response.execution,
      { outcome: { kind: 'run-returned', runReturn: 'ok' }, teardown: 'confirmed' });
    const manifestPath = join(project, MANIFEST), manifestBytes = ordinaryBytes(manifestPath, 16_384);
    const manifest = JSON.parse(manifestBytes);
    assert.equal(manifest.schema, 'zryna.wasi-command-manifest.v1');
    assert.equal(manifest.source.path, 'pure.zry'); assert.equal(manifest.source.sha256, sha256(source));
    assert.equal(manifest.source.profile, 'command-h1-v1');
    assert.deepEqual(manifest.source.requirements, []); assert.deepEqual(manifest.composition.approved, []);
    assert.deepEqual(manifest.grants.requested, []); assert.deepEqual(manifest.grants.effective, []);
    assert.deepEqual(manifest.grants.staticQuota, Array(10).fill(0));
    assert.deepEqual(manifest.input, { kind: 'none', utf8ByteCount: 0 });
    assert.deepEqual(manifest.execution, { kind: 'run-returned', runReturn: 'ok' });
    assert.equal(manifest.teardown, 'confirmed');
    assert.equal(manifest.component.path, `wasi-command/${STEM}.wasm`);
    assert.equal(manifest.component.kind, 'wasi-command-component-v1');
    assert.equal(manifest.component.world, 'zryna:capability-profiles/command@0.1.0');
    const component = ordinaryBytes(join(dirname(manifestPath), manifest.component.path), 1_048_576);
    assert.equal(sha256(component), manifest.component.sha256);
    assert(component.subarray(0, 8).equals(Buffer.from([0, 97, 115, 109, 13, 0, 1, 0])));
    assert(ordinaryBytes(join(project, 'pure.zry')).equals(source));
    if (first) {
      assert(component.equals(first.component), 'native/default actual component bytes differ');
      assert(manifestBytes.equals(first.manifestBytes), 'native/default entire actual manifests differ');
      assert.deepEqual(response.execution, first.execution);
    } else first = { component, manifestBytes, execution: response.execution };
    observations.push({ id: 'command-h1-' + label, providerArgumentOmitted: provider === null,
      sourceSha256: sha256(source), componentSha256: sha256(component), manifestSha256: sha256(manifestBytes),
      execution: response.execution, grants: 'empty', nodeRuntimeSelected: false });
  }
  for (const provider of ['bootstrap', 'compare']) {
    const project = join(projects, 'command-h1-absent-' + provider);
    mkdirSync(project, { mode: 0o700 }); writeFileSync(join(project, 'pure.zry'), source, { flag: 'wx' });
    const observed = invoke('command-h1-absent-' + provider,
      ['run', 'pure.zry', '--project-root', project, '--target', 'wasi-command',
        '--profile', 'command-h1-v1', '--export', 'main', '--name', STEM, '--provider', provider, '--json']);
    assert.equal(observed.exitCode, 2); const response = JSON.parse(observed.stdoutBytes);
    assert.equal(response.ok, false); assert.equal(response.manifest, null);
    assert(response.diagnostics.some(item => item.code === 'ZRYNA-C4103'));
    assert(!existsSync(join(project, '.zryna/out', STEM + '.wasi-command-run')));
    assert.deepEqual(readdirSync(project), ['pure.zry']);
    observations.push({ id: 'command-h1-absent-' + provider, exitCode: observed.exitCode,
      diagnostic: 'ZRYNA-C4103', artifactPublicationAbsent: true });
  }
  const result = { status: 'actual-installed-pure-h1-native-and-default-passed',
    exactRegisteredSource: 'examples/wasi-command/pure.zry', sourceSha256: sha256(source),
    actualComponentAndEntireManifestBytesEqual: true, observations };
  saveEvidence(output, 'installed-command-h1-smoke.json', result);
  return result;
}
