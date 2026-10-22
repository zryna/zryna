import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import test from 'node:test';
import { createSourceLauncher, createPublishedSourceLauncher } from './launcher.mjs';
import { composition, prefix, sourcePaths, sha256, validatePolicy } from './launcher-policy.mjs';

const pin = path => ({ path, bytes: statSync(path).size, ownerUid: statSync(path).uid,
  sha256: createHash('sha256').update(readFileSync(path)).digest('hex') });
const supported = process.platform === 'linux' && process.arch === 'x64' && process.version === 'v22.22.1';
function policy(root) {
  const sourceRoot = resolve(import.meta.dirname, '../../../..');
  const source = { kind: 'operator-reviewed-working-snapshot', repository: 'https://github.com/zryna/zryna', commit: null, tree: null,
    root: { path: sourceRoot, ownerUid: statSync(sourceRoot).uid },
    files: sourcePaths.map(path => ({ path, bytes: statSync(join(sourceRoot, path)).size,
      sha256: sha256(readFileSync(join(sourceRoot, path))), mode: statSync(join(sourceRoot, path)).mode & 0o7777 })) };
  const template = join(root, 'loader.js'); writeFileSync(template, 'inert transport-only template', { mode: 0o600 });
  // False is an inert admission fixture; no compile/evaluate operation runs in this static-route test.
  const compiler = pin('/usr/bin/false');
  const receipt = join(root, 'candidate.json');
  writeFileSync(receipt, JSON.stringify({ type: 'zryna.playground-compiler-candidate.v1',
    status: 'qualification-candidate', product: 'zryna-playground-compiler', productVersion: '0.1.0',
    source: { kind: source.kind, commit: source.commit, tree: source.tree }, compiler: { bytes: compiler.bytes, sha256: compiler.sha256 } }), { mode: 0o600 });
  const approvalPath = join(root, 'operator.json');
  const reviewed = { format: 'zryna.playground-source-launcher.v2', version: 2,
    compositionKind: 'reviewed-source-v1', compositionSha256: '0'.repeat(64), source,
    runtime: { nodeVersion: '22.22.1', node: pin(process.execPath) }, compiler,
    approval: { path: approvalPath, bytes: 1, sha256: '0'.repeat(64), ownerUid: process.getuid() },
    template: pin(template), witSha256: 'd'.repeat(64), compilerPublication: {
      status: 'qualification-candidate', type: 'zryna.playground-compiler-candidate.v1', receipt: pin(receipt) } };
  reviewed.compositionSha256 = sha256(Buffer.from(JSON.stringify(composition(reviewed))));
  writeFileSync(approvalPath, JSON.stringify({ version: 2, composition_kind: reviewed.compositionKind,
    composition_sha256: reviewed.compositionSha256, compiler_executable_sha256: compiler.sha256,
    binding_template_sha256: reviewed.template.sha256, wit_closure_sha256: reviewed.witSha256,
    policy_sha256: 'e'.repeat(64) }), { mode: 0o600 });
  reviewed.approval = pin(approvalPath);
  return reviewed;
}

test('source launcher serves actual pinned private UI only after explicit loopback launch and returns URL once',
  { skip: !supported }, async () => {
    const root = mkdtempSync(join(tmpdir(), 'playground-source-launcher-'));
    let launcher;
    try {
      const reviewed = policy(root);
      launcher = await createSourceLauncher(reviewed);
      assert.equal(launcher.qualificationOnly, true);
      assert.equal(launcher.source.kind, 'operator-reviewed-working-snapshot');
      assert.equal(launcher.source.commit, null); assert.equal(launcher.source.tree, null);
      assert.throws(() => launcher.takeLaunchUrl(), /SOURCE-STATE/);
      const location = await launcher.launch();
      assert.match(location.origin, /^http:\/\/127\.0\.0\.1:[1-9][0-9]*$/);
      const url = launcher.takeLaunchUrl();
      assert.equal(new URL(url).hash.length, 65);
      assert.throws(() => launcher.takeLaunchUrl(), /SOURCE-STATE/);
      const response = await fetch(`${location.origin}/${prefix}`);
      assert.equal(response.status, 200);
      assert.equal(await response.text(), readFileSync(join(reviewed.source.root.path, prefix, 'index.html'), 'utf8'));
      const headers = { authorization: `Bearer ${new URL(url).hash.slice(1)}`, 'sec-fetch-site': 'same-origin' };
      const session = await (await fetch(`${location.origin}/session`, { headers })).json();
      assert.equal(session.version, 2); assert.equal(session.evaluationMode, 'supervised-host');
      assert.equal(session.authority.toolkitSha256, reviewed.compositionSha256, 'explicit v2 source compatibility identity');
      const extra = await fetch(`${location.origin}/${prefix}daemon/launcher.mjs`);
      assert.equal(extra.status, 400, 'host source is not a static route');
      const stagedPath = launcher.recovery().snapshotPath;
      await launcher.stop(); launcher = null;
      assert.throws(() => statSync(stagedPath), /ENOENT/);
    } finally { if (launcher) await launcher.stop(); rmSync(root, { recursive: true }); }
  });

test('external composition pins cannot be changed by adjacent metadata; publication dictionaries grant nothing',
  { skip: !supported }, async () => {
    const root = mkdtempSync(join(tmpdir(), 'playground-source-policy-'));
    try {
      const reviewed = policy(root);
      const wrong = structuredClone(reviewed); wrong.source.files[0].sha256 = '0'.repeat(64);
      assert.throws(() => validatePolicy(wrong), /SOURCE-COMPOSITION/);
      const falseRevision = structuredClone(reviewed); falseRevision.source.commit = 'a'.repeat(40);
      assert.throws(() => validatePolicy(falseRevision), /SOURCE-POLICY/);
      const omitted = structuredClone(reviewed); omitted.source.files.pop();
      assert.throws(() => validatePolicy(omitted), /SOURCE-POLICY/);
      const claimed = structuredClone(reviewed); claimed.compilerPublication.status = 'authenticated';
      await assert.rejects(createSourceLauncher(claimed), /PUBLICATION-VERIFIER-REQUIRED/);
      assert.throws(() => createPublishedSourceLauncher(claimed, { receipt: {}, binary: new Uint8Array() }),
        /COMPILER-PUBLICATION-CAPABILITY/);
    } finally { rmSync(root, { recursive: true }); }
  });
