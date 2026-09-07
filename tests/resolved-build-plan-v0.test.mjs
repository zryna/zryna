import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import test from 'node:test';

import { digest as packageDigest } from '../scripts/package-release/canonical.mjs';

import {
  canonicalBytes,
  deriveCacheKey,
  deriveTargetCacheKey,
  loadBuildPlan,
  validateBuildPlan,
  validateBuildPlanBytes,
  validateCacheEntry,
  validatePublicationObservation,
  verifySourceMaterials,
  workspaceRoot,
} from '../scripts/build-plan/validate.mjs';

function withCache(document) {
  document.cacheKey = deriveCacheKey(document);
  return document;
}

function nativePlan(sourceOnly) {
  const document = structuredClone(sourceOnly);
  document.sourcePlan.targets.push({
    id: 'native-linux-x86_64',
    triple: 'x86_64-unknown-linux-gnu',
    abi: 'zryna-native-scalar-v1',
    features: [],
    composition: {
      contract: 'zryna.cross-target-profiles.v1',
      row: 'NATIVE-HOST',
      hostPolicySha256: '0'.repeat(64),
      approvedRequestSha256: '1'.repeat(64),
    },
    runtime: { name: 'zryna-native-runtime', version: '1', sha256: '3'.repeat(64) },
  });
  document.sourcePlan.hostTools = [
    { name: 'clang', version: '18.1.8', sha256: '4'.repeat(64), runsOn: 'x86_64-unknown-linux-gnu', targets: ['native-linux-x86_64'] },
    { name: 'lld', version: '18.1.8', sha256: '5'.repeat(64), runsOn: 'x86_64-unknown-linux-gnu', targets: ['native-linux-x86_64'] },
    { ...document.sourcePlan.hostTools[0], targets: ['javascript', 'native-linux-x86_64'] },
  ];
  document.sourcePlan.outputs.push({ path: 'native/app.elf', target: 'native-linux-x86_64' });
  document.nativeAppendix = {
    status: 'provisional-pending-364',
    target: 'native-linux-x86_64',
    abi: {
      identity: 'zryna-native-c-interop-v0',
      version: '0',
      targetTriple: 'x86_64-unknown-linux-gnu',
      callingConvention: 'system-v-amd64-c',
      carrierModel: 'native-c-interop-v0-carriers',
      ownershipModel: 'native-c-interop-v0-resources',
      runtime: { name: 'zryna-native-runtime', version: '1', sha256: '3'.repeat(64) },
      decisionIssue: 364,
    },
    acquisition: {
      targetLibraries: [
        { id: 'libc', version: '2.39', target: 'native-linux-x86_64', linkage: 'shared', artifact: 'libc-so' },
        { id: 'sample', version: '1.0.0', target: 'native-linux-x86_64', linkage: 'static', artifact: 'libsample-a' },
      ],
      sysroots: [{ id: 'linux-sysroot', target: 'native-linux-x86_64', sha256: '6'.repeat(64) }],
      staticArtifacts: [{ id: 'libsample-a', target: 'native-linux-x86_64', sha256: '7'.repeat(64), size: 128 }],
      sharedArtifacts: [{ id: 'libc-so', target: 'native-linux-x86_64', sha256: '8'.repeat(64), size: 256 }],
      runtimeDependencies: [{ id: 'libc-runtime', target: 'native-linux-x86_64', sharedArtifact: 'libc-so' }],
    },
    compilation: {
      steps: [{
        id: 'compile-sample',
        tool: 'clang',
        target: 'native-linux-x86_64',
        inputs: [{
          kind: 'source',
          graphRole: 'target/runtime',
          package: structuredClone(document.sourcePlan.rootPackage),
          path: 'src/main.zry',
        }],
        arguments: ['-c', 'sample.c'],
        output: 'sample-object',
      }],
    },
    linking: {
      tool: 'lld',
      target: 'native-linux-x86_64',
      inputs: [
        { kind: 'object', id: 'sample-object' },
        { kind: 'static', id: 'libsample-a' },
        { kind: 'shared', id: 'libc-so' },
        { kind: 'sysroot', id: 'linux-sysroot' },
      ],
      arguments: ['--build-id=none'],
      output: 'native/app.elf',
    },
  };
  return withCache(document);
}

test('source-only v0 plan validates and canonical replay is stable', async () => {
  const { document, schema } = await loadBuildPlan();
  const packageFixture = JSON.parse(await readFile(path.join(workspaceRoot, 'tests/package-release-v1/valid.json'), 'utf8'));
  assert.equal(document.sourcePlan.packageLockSha256, packageFixture.release.lockSha256);
  assert.deepEqual(document.sourcePlan.packages.map(({ package: identity }) => identity),
    packageFixture.lock.packages.map(({ id, sourceSha256 }) => ({ id, sourceSha256 })));
  assert.deepEqual(validateBuildPlan(document, schema), { cacheKey: document.cacheKey, native: false });
  const wire = canonicalBytes(document);
  assert.deepEqual(validateBuildPlanBytes(wire, schema), { cacheKey: document.cacheKey, native: false });
  assert.deepEqual(validateBuildPlanBytes(wire, schema), { cacheKey: document.cacheKey, native: false });
  assert.throws(() => validateBuildPlanBytes(Buffer.from(` ${wire}`), schema), /P361-WIRE/);
});

test('cache identity binds source, dependency, compiler, runtime, profile, target and host-tool inputs', async () => {
  const { document } = await loadBuildPlan();
  const changes = [
    (plan) => { plan.sourcePlan.sources[0].sha256 = '0'.repeat(64); },
    (plan) => { plan.sourcePlan.packageLockSha256 = '0'.repeat(64); },
    (plan) => { plan.sourcePlan.packages[0].package.id = '0'.repeat(64); },
    (plan) => { plan.sourcePlan.packages[0].package.sourceSha256 = '0'.repeat(64); },
    (plan) => { plan.sourcePlan.packages[0].dependencies[0].package.id = '3'.repeat(64); },
    (plan) => { plan.sourcePlan.packages[0].graphRole = 'host/build'; },
    (plan) => { plan.sourcePlan.compiler.version = '0.1.1'; },
    (plan) => { plan.sourcePlan.profile.configurationSha256 = '0'.repeat(64); },
    (plan) => { plan.sourcePlan.executionPolicy.configurationSha256 = '0'.repeat(64); },
    (plan) => { plan.sourcePlan.host.environment[0].value = 'en_US.UTF-8'; },
    (plan) => { plan.sourcePlan.targets[0].runtime.version = '2'; },
    (plan) => { plan.sourcePlan.targets[0].composition.hostPolicySha256 = '2'.repeat(64); },
    (plan) => { plan.sourcePlan.targets[0].triple = 'ecmascript2024-unknown-unknown'; },
    (plan) => { plan.sourcePlan.hostTools[0].sha256 = '0'.repeat(64); },
  ];
  for (const change of changes) {
    const changed = structuredClone(document);
    change(changed);
    assert.notEqual(deriveCacheKey(changed), document.cacheKey);
  }
});

test('source material repeat succeeds while missing and stale bytes fail closed', async () => {
  const { document, schema } = await loadBuildPlan();
  const materialKey = (source) =>
    `${source.graphRole}:${source.package.id}:${source.package.sourceSha256}:${source.path}`;
  const materials = new Map(document.sourcePlan.sources.map((source) =>
    [materialKey(source), Buffer.from('synthetic source fixture\n')]));
  assert.equal(verifySourceMaterials(document, materials), true);
  assert.equal(verifySourceMaterials(document, materials), true);
  const missing = new Map(materials);
  missing.delete(materialKey(document.sourcePlan.sources[1]));
  assert.throws(() => verifySourceMaterials(document, missing), /P361-SOURCE: missing source material/);
  const stale = new Map(materials);
  stale.set(materialKey(document.sourcePlan.sources[0]), Buffer.from('changed'));
  assert.throws(() => verifySourceMaterials(document, stale), /P361-SOURCE: stale source material/);
  const mismatchedDigest = structuredClone(document);
  mismatchedDigest.sourcePlan.sources[0].sha256 = '0'.repeat(64);
  assert.throws(() => validateBuildPlan(withCache(mismatchedDigest), schema), /P361-SOURCE: .* source digest differs from #168/);
});

test('source inventory exact bound passes and the first extra entry rejects', async () => {
  const { document, schema } = await loadBuildPlan();
  const exact = structuredClone(document);
  const packageRows = Array.from({ length: 16 }, (_, packageIndex) => {
    const files = Array.from({ length: 16 }, (_, sourceIndex) => ({
      path: `src/${String(sourceIndex).padStart(2, '0')}.zry`,
      sha256: 'a'.repeat(64),
      size: 0,
    }));
    const packageIdentity = {
      id: String(packageIndex + 1).padStart(64, '0'),
      sourceSha256: packageDigest('source-files', files),
    };
    return { packageIdentity, files };
  });
  exact.sourcePlan.packages = packageRows.map(({ packageIdentity }) => ({
    graphRole: 'target/runtime',
    package: packageIdentity,
    dependencies: [],
  }));
  exact.sourcePlan.rootPackage = structuredClone(packageRows[0].packageIdentity);
  exact.sourcePlan.sources = packageRows.flatMap(({ packageIdentity, files }) =>
    files.map((file) => ({
      graphRole: 'target/runtime',
      package: packageIdentity,
      ...file,
    })));
  assert.doesNotThrow(() => validateBuildPlan(withCache(exact), schema));
  const extra = structuredClone(exact);
  extra.sourcePlan.sources.push({ ...extra.sourcePlan.sources[255], path: 'src/16.zry' });
  assert.throws(() => validateBuildPlan(withCache(extra), schema), /P361-SCHEMA/);
});

test('native draft separates acquisition, compilation and linking and rejects boundary confusion', async () => {
  const { document, schema } = await loadBuildPlan();
  const valid = nativePlan(document);
  assert.deepEqual(validateBuildPlan(valid, schema), { cacheKey: valid.cacheKey, native: true });
  const changedArtifact = structuredClone(valid);
  changedArtifact.nativeAppendix.acquisition.staticArtifacts[0].sha256 = '9'.repeat(64);
  assert.notEqual(deriveCacheKey(changedArtifact), valid.cacheKey);

  const wrongTarget = structuredClone(valid);
  wrongTarget.nativeAppendix.acquisition.staticArtifacts[0].target = 'javascript';
  assert.throws(() => validateBuildPlan(withCache(wrongTarget), schema), /P361-TARGET: libsample-a has the wrong target/);

  const wrongRuntime = structuredClone(valid);
  wrongRuntime.nativeAppendix.abi.runtime.sha256 = '0'.repeat(64);
  assert.throws(() => validateBuildPlan(withCache(wrongRuntime), schema), /P361-NATIVE: native ABI runtime identity differs/);

  const missingLibrary = structuredClone(valid);
  missingLibrary.nativeAppendix.acquisition.targetLibraries[1].artifact = 'missing-a';
  assert.throws(() => validateBuildPlan(withCache(missingLibrary), schema), /P361-NATIVE: sample is missing its static artifact/);

  const undeclaredTool = structuredClone(valid);
  undeclaredTool.nativeAppendix.compilation.steps[0].tool = 'ambient-cc';
  assert.throws(() => validateBuildPlan(withCache(undeclaredTool), schema), /P361-TOOLCHAIN: undeclared compilation tool/);
});

test('accepted #357 profile and target rows do not widen or cross target axes', async () => {
  const { document, schema } = await loadBuildPlan();
  const wrongRow = structuredClone(document);
  wrongRow.sourcePlan.targets[0].composition.row = 'U-WASM';
  assert.throws(() => validateBuildPlan(withCache(wrongRow), schema), /P361-TARGET: U-WASM is incompatible with javascript/);
  const unresolvedAll = structuredClone(document);
  unresolvedAll.sourcePlan.targets[0].id = 'all';
  assert.throws(() => validateBuildPlan(withCache(unresolvedAll), schema), /P361-SCHEMA/);
});

test('cache miss, hit, wrong target and stale output have distinct deterministic outcomes', async () => {
  const { document } = await loadBuildPlan();
  assert.deepEqual(validateCacheEntry(document, 'javascript', undefined, new Map()), { outcome: 'miss' });
  const bytes = Buffer.from('export function main(){return 1}\n');
  const entry = {
    key: deriveTargetCacheKey(document, 'javascript'),
    target: 'javascript',
    outputs: [{ path: 'javascript/app.mjs', size: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') }],
  };
  assert.deepEqual(validateCacheEntry(document, 'javascript', entry, new Map([['javascript/app.mjs', bytes]])), {
    outcome: 'hit',
    key: entry.key,
  });
  assert.throws(
    () => validateCacheEntry(document, 'javascript', { ...entry, target: 'native-linux-x86_64' }, new Map()),
    /P361-CACHE: cache entry identity is incompatible/,
  );
  assert.throws(
    () => validateCacheEntry(document, 'javascript', entry, new Map([['javascript/app.mjs', Buffer.from('stale')]])),
    /P361-CACHE: stale cached output/,
  );
});

test('interrupted and create-only publication preserve the destination boundary', () => {
  assert.deepEqual(validatePublicationObservation({
    destinationExistedBefore: false,
    commitAttempted: false,
    commitCompleted: false,
    destinationVisible: false,
    completeManifest: false,
    priorDestinationPreserved: false,
  }), { outcome: 'interrupted-hidden' });
  assert.deepEqual(validatePublicationObservation({
    destinationExistedBefore: true,
    commitAttempted: false,
    commitCompleted: false,
    destinationVisible: true,
    completeManifest: true,
    priorDestinationPreserved: true,
  }), { outcome: 'collision-preserved' });
  assert.throws(() => validatePublicationObservation({
    destinationExistedBefore: false,
    commitAttempted: true,
    commitCompleted: false,
    destinationVisible: true,
    completeManifest: false,
    priorDestinationPreserved: false,
  }), /P361-PUBLICATION: interrupted staging became visible/);
});

test('documentation keeps authority, exact identities and implementation stages explicit', async () => {
  const contract = await readFile(path.join(workspaceRoot, 'spec/package/RESOLVED_BUILD_PLAN_V0.md'), 'utf8');
  const normalized = contract.replace(/\s+/g, ' ');
  for (const phrase of [
    'Package and provenance authority remains with #168.',
    'The package identity is the exact ordered pair `(id, sourceSha256)`',
    'Each package occurrence, dependency edge, and qualified source carries a graph role',
    '`zryna.cross-target-profiles.v1`',
    'The driver alone owns compilation orchestration, tool validation, linking, cache materialization, and output publication.',
    'The native appendix is provisional pending the relevant accepted #364 ABI decisions.',
    'specified → implemented → conformance-passed → publicly supported',
  ]) assert.ok(normalized.includes(phrase), phrase);
});
