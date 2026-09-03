import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { parseDocument } from 'yaml';

import {
  expectedRegistrySha256,
  loadAndValidateM2Conformance,
  rustTestSelectorExists,
  validateM2Conformance,
} from '../scripts/check-m2-conformance.mjs';
import {
  M2_CONFORMANCE_COMMANDS,
  validateM2ConformanceCommands,
} from '../scripts/run-m2-conformance.mjs';
import {
  M2_QUICK_COMMANDS,
  runM2Quick,
  validateM2QuickCommands,
} from '../scripts/run-m2-quick.mjs';

const fixturePaths = loadAndValidateM2Conformance().fixtureFiles.map(({ path: fixturePath }) =>
  fixturePath);

function cloneRegistry() {
  return structuredClone(loadAndValidateM2Conformance());
}

test('Rust evidence requires the exact module-qualified registered test', () => {
  const selector = 'pipeline::tests::registered_evidence';
  assert.equal(rustTestSelectorExists('#[test]\nfn registered_evidence() {}\n', selector, 'pipeline::tests'), true);
  assert.equal(rustTestSelectorExists('fn registered_evidence() {}\n', selector, 'pipeline::tests'), false);
  assert.equal(rustTestSelectorExists('// #[test]\nfn registered_evidence() {}\n', selector, 'pipeline::tests'), false);
  assert.equal(rustTestSelectorExists('#[test]\nfn registered_evidence() {}\n', selector, 'other::tests'), false);
});

function workflowJob(workflow, jobId) {
  const lines = workflow.split(/\r?\n/);
  const start = lines.findIndex((line) => line === `  ${jobId}:`);
  assert.notEqual(start, -1, `missing ${jobId} job`);
  const next = lines.findIndex((line, index) =>
    index > start && /^  [a-zA-Z0-9_-]+:$/.test(line));
  return lines.slice(start, next === -1 ? undefined : next).join('\n');
}

test('authenticates the executable oracle separately from historical planning', () => {
  const registry = loadAndValidateM2Conformance();
  assert.equal(expectedRegistrySha256.length, 64);
  assert.equal(registry.profile, 'zryna-control-flow-fixed-oracle-v1');
  assert.deepEqual(registry.targetOrder, ['javascript', 'webassembly', 'native']);
  assert.equal(registry.validCases.length, 20);
  assert.equal(registry.invalidCases.length, 9);
  assert.equal(registry.validCases[14].id, 'argument-order-is-positional');
  assert.deepEqual(registry.validCases[14].expected, { type: 'i32', value: -2 });
});

test('rejects oracle, inventory, provenance, and evidence drift', () => {
  for (const mutate of [
    (registry) => { registry.profile = 'different'; },
    (registry) => { registry.targetOrder.reverse(); },
    (registry) => { registry.validCases.pop(); },
    (registry) => { registry.validCases[0].arguments[0].value = 0; },
    (registry) => { registry.validCases[0].expected.value = 0; },
    (registry) => { registry.validCases[14].export = 'commutative'; },
    (registry) => { registry.invalidCases[0].diagnosticCodes[0] = 'ZRYNA-M2001'; },
    (registry) => { registry.invalidCases[0].diagnostics[0].message = 'changed'; },
    (registry) => { registry.graph.sha256 = '0'.repeat(64); },
    (registry) => { registry.graph.sources[0].sha256 = '0'.repeat(64); },
    (registry) => { registry.graph.buildArtifacts[0].bytes += 1; },
    (registry) => { registry.fixtureFiles[0].sha256 = '0'.repeat(64); },
    (registry) => { registry.boundaryEvidence.irreducibleCfg.diagnosticCode = 'ZRYNA-I2001'; },
    (registry) => { registry.boundaryEvidence.atomicFailures.test = 'not-a-real-test'; },
    (registry) => { registry.boundaryEvidence.sourceRaces = { anything: true }; },
    (registry) => { registry.boundaryEvidence.resourceLimits.rows.pop(); },
    (registry) => { registry.boundaryEvidence.resourceLimits.rows[0].limit += 1; },
    (registry) => { registry.boundaryEvidence.resourceLimits.rows[0].commandId = 'missing'; },
    (registry) => { registry.boundaryEvidence.resourceLimits.rows[0].test = 'not-a-real-test'; },
    (registry) => { registry.determinism.compareArtifactBytes = false; },
    (registry) => { registry.unknown = true; },
  ]) {
    const registry = cloneRegistry();
    mutate(registry);
    assert.throws(
      () => validateM2Conformance(registry, fixturePaths),
      /invalid M2 executable conformance/,
    );
  }
});

test('rejects unlisted, missing, reordered, and case-colliding fixture inventory', () => {
  const registry = cloneRegistry();
  for (const paths of [
    fixturePaths.slice(1),
    [...fixturePaths, 'tests/m2-fixtures/unlisted.zry'],
    [...fixturePaths].reverse(),
  ]) {
    assert.throws(
      () => validateM2Conformance(registry, paths),
      /fixture inventory drifted/,
    );
  }
});

test('distinguishes registry authentication from canonical structure', async () => {
  const directory = await mkdtemp(path.join(tmpdir(), 'zryna-m2-conformance-'));
  try {
    const changed = cloneRegistry();
    changed.validCases[0].expected.value = 0;
    const changedPath = path.join(directory, 'changed.json');
    await writeFile(changedPath, `${JSON.stringify(changed, null, 2)}\n`);
    assert.throws(
      () => loadAndValidateM2Conformance(changedPath),
      /registry digest mismatch/,
    );

    const source = await readFile(new URL('./m2-conformance-v1.json', import.meta.url), 'utf8');
    const noncanonicalPath = path.join(directory, 'noncanonical.json');
    await writeFile(noncanonicalPath, JSON.stringify(JSON.parse(source)));
    assert.throws(
      () => loadAndValidateM2Conformance(noncanonicalPath, { verifyDigest: false }),
      /registry bytes are not canonical JSON/,
    );

    const oversizedPath = path.join(directory, 'oversized.json');
    await writeFile(oversizedPath, Buffer.alloc(1024 * 1024 + 1, 0x20));
    assert.throws(
      () => loadAndValidateM2Conformance(oversizedPath, { verifyDigest: false }),
      /registry exceeds its byte limit/,
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('freezes bounded no-shell command declarations', () => {
  assert.doesNotThrow(() => validateM2ConformanceCommands());
  assert.deepEqual(
    M2_CONFORMANCE_COMMANDS.map(({ id }) => id),
    ['registry-and-fixture-contract', 'public-fixed-oracle-corpus', 'internal-boundary-evidence'],
  );
  for (const command of M2_CONFORMANCE_COMMANDS) {
    assert.equal(typeof command.executable, 'string');
    assert.ok(command.args.every((argument) => typeof argument === 'string'));
    assert.ok(command.timeout >= 30_000 && command.timeout <= 10 * 60_000);
  }
  for (const mutate of [
    (commands) => { commands[0].executable = 'shell'; },
    (commands) => { commands[1].args.pop(); },
    (commands) => { commands[2].timeout += 1; },
  ]) {
    const commands = structuredClone(M2_CONFORMANCE_COMMANDS);
    mutate(commands);
    assert.throws(
      () => validateM2ConformanceCommands(commands),
      /commands differ from the frozen command set/,
    );
  }

  assert.doesNotThrow(() => validateM2QuickCommands());
  assert.equal(M2_QUICK_COMMANDS.length, 15);
  assert.deepEqual(
    M2_QUICK_COMMANDS.slice(0, 3).map(({ id, executable }) => [id, executable]),
    [
      ['portable-contracts', 'node'],
      ['adapter-boundaries', 'node'],
      ['public-control-flow-profile', 'cargo'],
    ],
  );
  assert.ok(M2_QUICK_COMMANDS.some(({ id }) => id === 'portable-ir'));
  const changedQuick = structuredClone(M2_QUICK_COMMANDS);
  changedQuick[1].args.pop();
  assert.throws(() => validateM2QuickCommands(changedQuick),
    /quick commands differ from the frozen command set/);
});

test('quick runner is shell-free, bounded, ordered, and fail-fast', () => {
  const started = [];
  const commands = [
    { id: 'first', executable: 'node', args: ['first'], timeout: 30_000 },
    { id: 'fails', executable: 'cargo', args: ['fails'], timeout: 30_000 },
    { id: 'must-not-run', executable: 'node', args: ['later'], timeout: 30_000 },
  ];
  const spawn = (executable, args, options) => {
    started.push([executable, args[0]]);
    assert.equal(options.shell, false);
    assert.equal(options.maxBuffer, 8 * 1024 * 1024);
    assert.equal(options.timeout, 30_000);
    return { status: args[0] === 'fails' ? 9 : 0, stdout: '', stderr: '' };
  };
  assert.throws(() => runM2Quick(commands, spawn), /fails failed with exit status 9/);
  assert.deepEqual(started, [['node', 'first'], ['cargo', 'fails']]);
});

test('package and CI expose one stable cross-platform M2 gate', async () => {
  const packageDocument = JSON.parse(
    await readFile(new URL('../package.json', import.meta.url), 'utf8'),
  );
  assert.equal(packageDocument.scripts['m2:registry'],
    'node scripts/check-m2-conformance.mjs && node --test tests/m2-conformance.test.mjs');
  assert.equal(packageDocument.scripts['m2:check'], 'node scripts/run-m2-conformance.mjs');
  assert.equal(packageDocument.scripts['m2:quick'], 'node scripts/run-m2-quick.mjs');

  const workflow = await readFile(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8');
  const document = parseDocument(workflow);
  assert.deepEqual(document.errors, []);
  const parsed = document.toJS();
  const platform = workflowJob(workflow, 'm2-platform');
  const aggregate = workflowJob(workflow, 'm2');
  const ciDigest = createHash('sha256').update(JSON.stringify({
    platform: parsed.jobs['m2-platform'],
    aggregate: parsed.jobs.m2,
  })).digest('hex');
  assert.equal(ciDigest, '8eeee67d331c528f713625dde310a7af6a6f42cc0f85efcb8bf551a0de13ddaf');

  assert.equal(parsed.jobs['m2-platform'].needs, undefined);
  assert.deepEqual(parsed.jobs['m2-platform'].strategy.matrix.os,
    ['ubuntu-latest', 'windows-latest']);
  assert.deepEqual(parsed.jobs.m2.needs, ['m0', 'm2-platform']);
  assert.equal(parsed.jobs.m2.if, 'always()');
  assert.match(platform, /name: Fetch locked Rust dependencies\s+run: cargo fetch --locked/);
  assert.match(platform, /run: pnpm m2:check/);
  assert.match(aggregate, /M0_RESULT: \$\{\{ needs\.m0\.result \}\}/);
  assert.match(aggregate, /PLATFORM_RESULT: \$\{\{ needs\.m2-platform\.result \}\}/);
  assert.match(aggregate,
    /test "\$M0_RESULT" = success && test "\$PLATFORM_RESULT" = success/);
});
