import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtempSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { bytes, sha256 } from '../../../../scripts/distribution/canonical.mjs';
import { encodeTar } from '../../../../scripts/distribution/archive-tar.mjs';
import { archiveRoot, certificateIdentity, issuer, sourceRef, verifierPins } from './compiler-publication-constants.mjs';
import { compilerPublicationFixture } from './compiler-publication-fixture.mjs';
import { createUnsignedCompilerPublication } from './compiler-publication-producer.mjs';
import { validateCompilerPublicationEnvelope } from './compiler-publication-schema.mjs';
import { verifyCompilerPublicationArchive } from './compiler-publication-archive.mjs';
import { validateCompilerElf, validateCompilerRustMaterials } from './compiler-publication-metadata.mjs';
import { captureCompilerPublicationVerifier, compilerPublicationVerificationArguments } from './compiler-publication-verifier.mjs';
import { authenticateCompilerPublication, compilerPublicationReceipt, compilerPublicationBinary,
  compilerPublicationFiles, compilerPublicationVerification } from './compiler-publication-capability.mjs';

// Synthetic unsigned archives exercise byte/schema rejection only. No successful auth capability is fabricated.
const prepared = await createUnsignedCompilerPublication(compilerPublicationFixture());
function policyFor(document) {
  const envelope = bytes(document);
  return { version: 1, source: structuredClone(document.source), envelopeSha256: sha256(envelope),
    archive: structuredClone(document.archive), binary: structuredClone(document.binary),
    provenanceSha256: sha256(bytes(document.provenance)) };
}
function validate(document) { return validateCompilerPublicationEnvelope(bytes(document), policyFor(document)); }
const clone = () => structuredClone(prepared.document);

test('compiler-only unsigned producer is deterministic and its complete archive verifies without conferring authority', async () => {
  const again = await createUnsignedCompilerPublication(compilerPublicationFixture());
  assert.equal(again.status, 'unsigned-candidate-not-authenticated');
  assert.deepEqual(again.archive, prepared.archive); assert.deepEqual(again.envelope, prepared.envelope);
  const document = validateCompilerPublicationEnvelope(again.envelope, again.candidatePolicy);
  const verified = await verifyCompilerPublicationArchive(again.archive, document);
  assert.equal(verified.files.length, 8); assert.equal(verified.materials.packages.length, 1);
  assert.throws(() => compilerPublicationReceipt(again.candidatePolicy), /CAPABILITY/);
});

for (const [name, mutate] of [
  ['source repository', d => { d.source.repository = 'https://foreign.invalid'; }],
  ['unprotected tag spelling', d => { d.source.ref = 'refs/tags/playground-v0.1.0'; }],
  ['nonannotated source', d => { d.source.tagType = 'commit'; }],
  ['tag masquerading as commit', d => { d.source.tagObject = d.source.commit; }],
  ['source epoch bounds', d => { d.source.sourceDateEpoch = 2 ** 32; }],
  ['foreign workflow signer', d => { d.signing.certificateIdentity = 'https://foreign.invalid'; }],
  ['foreign issuer', d => { d.signing.issuer = 'https://accounts.google.com'; }],
  ['wrong product', d => { d.product = 'zryna'; }],
  ['wrong version', d => { d.productVersion = '0.2.3'; }],
  ['unsupported target', d => { d.target = 'x86_64-pc-windows-msvc'; }],
  ['unsupported profile', d => { d.profile = 'trusted-native'; }],
  ['builder source substitution', d => { d.provenance.workflow.sha = 'f'.repeat(40); }],
  ['pull request builder', d => { d.provenance.workflow.event = 'pull_request'; }],
  ['unlocked build', d => { d.provenance.build.locked = false; }],
  ['wrong source env', d => { d.provenance.build.sourceCommitEnv = 'f'.repeat(40); }],
  ['wrong toolchain', d => { d.provenance.toolchain.rust = 'stable'; }],
  ['missing sixth locked input', d => { delete d.provenance.inputs.workspaceManifest; }],
  ['wrong locked input path', d => { d.provenance.inputs.rustToolchain.path = 'other.toml'; }],
  ['stale CI tree', d => { d.provenance.requiredCi.headTree = 'f'.repeat(40); }],
  ['failed CI', d => { d.provenance.requiredCi.conclusion = 'failure'; }],
  ['unclaimed reproduction', d => { d.provenance.reproduction.status = 'not-claimed'; }],
  ['different second build', d => { d.provenance.reproduction.secondBuildSha256 = 'f'.repeat(64); }],
  ['missing inventory member', d => { d.files = d.files.filter(f => f.path !== 'NOTICE'); }],
  ['extra binary descriptor field', d => { d.binary.owner = 1000; }],
  ['privileged executable mode', d => { d.files.find(f => f.path.startsWith('bin/')).mode = 0o4755; }],
  ['missing metadata relationship', d => { d.provenance.sbom.sha256 = 'f'.repeat(64); }],
  ['duplicated inventory path', d => { d.files.splice(1, 0, { ...d.files[0] }); }],
  ['unknown root authority', d => { d.command = '/bin/sh'; }],
]) {
  test(`closed compiler publication rejects ${name} even after adjacent metadata is resealed`, () => {
    const document = clone(); mutate(document); assert.throws(() => validate(document));
  });
}

test('independent source archive binary envelope and provenance pins cannot be replaced by a signed-looking record', () => {
  for (const mutate of [p => { p.source.commit = 'f'.repeat(40); }, p => { p.archive.sha256 = 'f'.repeat(64); },
    p => { p.binary.sha256 = 'f'.repeat(64); }, p => { p.envelopeSha256 = 'f'.repeat(64); },
    p => { p.provenanceSha256 = 'f'.repeat(64); }, p => { p.extra = true; }]) {
    const policy = structuredClone(prepared.candidatePolicy); mutate(policy);
    assert.throws(() => validateCompilerPublicationEnvelope(prepared.envelope, policy));
  }
});

test('canonical compiler envelope rejects duplicate keys trailing bytes and carrier overflow', () => {
  for (const wire of [Buffer.concat([prepared.envelope, Buffer.from(' ')]),
    Buffer.from(prepared.envelope.toString().replace('"version":1', '"version":1,"version":1')),
    Buffer.alloc(262145)]) {
    const policy = { ...prepared.candidatePolicy, envelopeSha256: sha256(wire) };
    assert.throws(() => validateCompilerPublicationEnvelope(wire, policy));
  }
});

test('compiler-only archive refuses Node providers browser scripts unknown executables and alias paths', async () => {
  for (const addition of [
    { path: 'runtime/node/bin/node', mode: 0o755, data: Buffer.from('inert') },
    { path: 'lib/zryna/bootstrap/worker.mjs', mode: 0o644, data: Buffer.from('inert') },
    { path: 'browser/chrome', mode: 0o755, data: Buffer.from('inert') },
    { path: 'scripts/helper.mjs', mode: 0o644, data: Buffer.from('inert') },
    { path: 'notice', mode: 0o644, data: Buffer.from('case alias') },
    { path: '../outside', mode: 0o644, data: Buffer.from('traversal') },
  ]) {
    const input = compilerPublicationFixture(); input.files.push(addition);
    await assert.rejects(createUnsignedCompilerPublication(input));
  }
});

test('complete archive verification rejects changed member bytes even when only the archive digest is resealed', async () => {
  const verified = await verifyCompilerPublicationArchive(prepared.archive, prepared.document);
  verified.files.find(f => f.path === 'README.md').data = Buffer.from('substituted inert text');
  const archive = await encodeTar(archiveRoot, verified.files, prepared.document.source.sourceDateEpoch);
  const document = clone(); document.archive = { ...document.archive, bytes: archive.length, sha256: sha256(archive) };
  await assert.rejects(verifyCompilerPublicationArchive(archive, document), /inventory/);
});

test('ELF validation rejects wrong machine writable executable segments out-of-range segments and executable stack', () => {
  const binary = compilerPublicationFixture().files.find(f => f.path.startsWith('bin/')).data;
  for (const mutate of [b => b.writeUInt16LE(183, 18), b => b.writeUInt32LE(7, 68),
    b => b.writeBigUInt64LE(129n, 96), b => b.writeUInt32LE(0x6474e551, 64)]) {
    const changed = Buffer.from(binary); mutate(changed); assert.throws(() => validateCompilerElf(changed));
  }
});

test('Rust material source lock package order and exact declared license bytes remain bound', () => {
  const input = compilerPublicationFixture(); const document = clone();
  for (const mutate of [d => { d.source.tree = 'f'.repeat(40); }, d => { d.cargoLockSha256 = 'f'.repeat(64); },
    d => { d.packages = []; }, d => { d.packages.push(structuredClone(d.packages[0])); },
    d => { d.packages[0].licenses[0].path = 'licenses/rust/other-1.0/LICENSE'; }]) {
    const data = JSON.parse(Buffer.from(input.materials).toString()); mutate(data);
    assert.throws(() => validateCompilerRustMaterials(bytes(data), document));
  }
});

test('fixed public signature invocation retains exact identity source and all transparency checks', () => {
  const args = compilerPublicationVerificationArguments('1'.repeat(40), '/tmp/unit-only');
  for (const pair of [['--certificate-identity', certificateIdentity], ['--certificate-oidc-issuer', issuer],
    ['--certificate-github-workflow-ref', sourceRef], ['--certificate-github-workflow-trigger', 'push'],
    ['--certificate-github-workflow-sha', '1'.repeat(40)]]) {
    assert.equal(args[args.indexOf(pair[0]) + 1], pair[1]);
  }
  assert(args.includes('--offline')); assert(args.includes('--insecure-ignore-tlog=false'));
  assert(args.includes('--insecure-ignore-sct=false')); assert(args.includes('--private-infrastructure=false'));
});

test('unsupported verifier hosts and arbitrary hash root version ownership or symlink claims never select an executable', () => {
  if (process.platform !== 'linux' || process.arch !== 'x64') {
    assert.throws(() => captureCompilerPublicationVerifier({ ...verifierPins, path: '/never-selected', ownerUid: 0 }));
    return;
  }
  const root = mkdtempSync(path.join(tmpdir(), 'compiler-publication-negative-'));
  try {
    const file = path.join(root, 'inert'); writeFileSync(file, 'inert; never executable', { mode: 0o400 });
    const policy = { ...verifierPins, path: file, ownerUid: process.getuid?.() ?? 0 };
    for (const change of [{ sha256: 'f'.repeat(64) }, { version: '3.0.4' },
      { trustedRootSha256: 'f'.repeat(64) }, { ownerUid: -1 }, { bytes: 17 }, {}]) {
      assert.throws(() => captureCompilerPublicationVerifier({ ...policy, ...change }));
    }
    const linked = path.join(root, 'linked'); symlinkSync(file, linked);
    assert.throws(() => captureCompilerPublicationVerifier({ ...policy, path: linked }));
  } finally { rmSync(root, { recursive: true, force: true }); }
});

test('plain objects mirrored unsigned receipts foreign brands and invalid acquisition cannot mint public activation', async () => {
  for (const fake of [{}, prepared.candidatePolicy, prepared.document, { status: 'authenticated' }, null]) {
    for (const get of [compilerPublicationReceipt, compilerPublicationBinary, compilerPublicationFiles, compilerPublicationVerification]) {
      assert.throws(() => get(fake), /CAPABILITY/);
    }
  }
  await assert.rejects(authenticateCompilerPublication({ envelope: prepared.envelope,
    bundle: new Uint8Array([1]), archive: prepared.archive, policy: prepared.candidatePolicy,
    verifierPolicy: { ...verifierPins, path: '/never-selected', ownerUid: 1000 },
    trustedRoot: new Uint8Array([1]) }), /CARRIER-SIZE/);
});

test('authentication snapshots all source policy and byte carriers synchronously before dynamic verifier loading', async () => {
  const input = { envelope: Buffer.from(prepared.envelope), bundle: new Uint8Array([1]),
    archive: Buffer.from(prepared.archive), policy: structuredClone(prepared.candidatePolicy),
    verifierPolicy: { ...verifierPins, path: '/never-selected', ownerUid: 1000 },
    trustedRoot: new Uint8Array(verifierPins.trustedRootBytes) };
  const pending = authenticateCompilerPublication(input);
  input.envelope.fill(0); input.archive.fill(0); input.policy.source.commit = 'f'.repeat(40);
  input.policy.archive.sha256 = 'f'.repeat(64); input.verifierPolicy.path = '/substituted';
  // The owned valid envelope/archive reaches the deliberately invalid root check, never an executable.
  await assert.rejects(pending, /signature materials/);
});
