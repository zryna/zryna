import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { parseDocument } from 'yaml';

import {
  PREFLIGHT_COMMANDS,
  preflightCommandDigest,
  runPreflight,
  validatePreflightCommands,
} from '../scripts/run-preflight.mjs';

function workflowJob(workflow, jobId) {
  const lines = workflow.split(/\r?\n/);
  const start = lines.findIndex((line) => line === `  ${jobId}:`);
  assert.notEqual(start, -1, `missing ${jobId} job`);
  const next = lines.findIndex((line, index) => index > start && /^  [a-zA-Z0-9_-]+:$/.test(line));
  return lines.slice(start, next === -1 ? undefined : next).join('\n');
}

test('preflight has one frozen portable command order', () => {
  assert.deepEqual(
    PREFLIGHT_COMMANDS.map(({ id, executable }) => [id, executable]),
    [
      ['portable-contract-tests', 'node'],
      ['rust-format', 'cargo'],
      ['m2-semantic-driver-tests', 'cargo'],
      ['m3-layout-tests', 'cargo'],
      ['m3-ownership-runtime-abi-tests', 'cargo'],
      ['m3-data-ir-tests', 'cargo'],
      ['m3-aggregate-semantics-tests', 'cargo'],
      ['rust-workspace-check', 'cargo'],
      ['frontend-syntax-tests', 'cargo'],
    ],
  );
  assert.ok(PREFLIGHT_COMMANDS.every(({ args }) => Object.isFrozen(args)));
  assert.ok(Object.isFrozen(PREFLIGHT_COMMANDS));
  assert.equal(preflightCommandDigest(), '333e0f2538ce879c5399dfde520baa2be025cd3671cedb628d33d33bf3b6a866');
  assert.doesNotThrow(() => validatePreflightCommands());

  for (const mutate of [
    (commands) => commands[0].args.pop(),
    (commands) => { commands[1].args[0] = 'check'; },
    (commands) => commands[2].args.splice(commands[2].args.indexOf('--locked'), 1),
    (commands) => commands[3].args.pop(),
    (commands) => commands[4].args.pop(),
    (commands) => commands[5].args.pop(),
    (commands) => commands[6].args.pop(),
    (commands) => commands[7].args.pop(),
    (commands) => commands[8].args.pop(),
  ]) {
    const changed = structuredClone(PREFLIGHT_COMMANDS);
    mutate(changed);
    assert.throws(() => validatePreflightCommands(changed), /differ from the frozen command set/);
  }
});

test('preflight stops at the first failure', () => {
  const started = [];
  const commands = [
    { id: 'first', executable: 'node', args: ['first'] },
    { id: 'fails', executable: 'node', args: ['fails'] },
    { id: 'must-not-run', executable: 'node', args: ['must-not-run'] },
  ];
  const spawn = (_executable, args, options) => {
    started.push(args[0]);
    assert.equal(options.shell, false);
    return { status: args[0] === 'fails' ? 7 : 0 };
  };

  assert.throws(() => runPreflight(commands, spawn), /fails failed with exit status 7/);
  assert.deepEqual(started, ['first', 'fails']);
});

test('pull-request platform jobs wait for preflight and the aggregate requires every gate', async () => {
  const workflow = await readFile(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8');
  const document = parseDocument(workflow);
  assert.deepEqual(document.errors, []);
  const parsed = document.toJS();
  const preflight = workflowJob(workflow, 'preflight');
  const rust = workflowJob(workflow, 'rust');
  const adapterPlatform = workflowJob(workflow, 'adapter-platform');
  const aggregate = workflowJob(workflow, 'm0');

  assert.match(preflight, /name: preflight/);
  assert.match(preflight, /run: pnpm preflight/);
  assert.match(rust, /name: Fetch locked Rust dependencies\s+run: cargo fetch --locked/);
  assert.match(
    rust,
    /name: Verify M2 semantics\s+run: cargo test --locked -p zryna-semantics --lib control_flow_v1/,
  );
  assert.match(rust, /run: cargo test --locked -p zryna-driver --lib/);
  assert.doesNotMatch(rust, /module_closure_tests::/);
  assert.match(rust, /needs: preflight/);
  assert.match(adapterPlatform, /needs: preflight/);
  assert.match(aggregate, /needs: \[preflight, rust, adapter\]/);
  assert.match(aggregate, /PREFLIGHT_RESULT: \$\{\{ needs\.preflight\.result \}\}/);
  assert.match(aggregate, /test "\$PREFLIGHT_RESULT" = success/);

  assert.equal(parsed.jobs.rust.needs, 'preflight');
  assert.equal(parsed.jobs['adapter-platform'].needs, 'preflight');
  assert.equal(parsed.jobs.adapter.needs, 'adapter-platform');
  assert.deepEqual(parsed.jobs.m0.needs, ['preflight', 'rust', 'adapter']);

  const controlledResults = { preflight: 'failure' };
  controlledResults.rust = controlledResults.preflight === 'success' ? 'success' : 'skipped';
  controlledResults.adapterPlatform = controlledResults.preflight === 'success' ? 'success' : 'skipped';
  controlledResults.adapter = controlledResults.adapterPlatform === 'success' ? 'success' : 'failure';
  controlledResults.m0 = Object.values(controlledResults).every((result) => result === 'success')
    ? 'success'
    : 'failure';
  assert.deepEqual(controlledResults, {
    preflight: 'failure',
    rust: 'skipped',
    adapterPlatform: 'skipped',
    adapter: 'failure',
    m0: 'failure',
  });
});

test('package exposes the exact documented preflight entrypoint', async () => {
  const packageDocument = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'));
  assert.equal(packageDocument.scripts.preflight, 'node scripts/run-preflight.mjs');
  assert.equal(
    packageDocument.scripts['m2:quick'],
    'node scripts/run-m2-quick.mjs',
  );
  assert.equal(
    packageDocument.scripts['m3:data:quick'],
    'cargo test --locked -p zryna-semantics data_ownership_v1 -- --skip authenticated_v4_derived_value_budget_is_exact_and_plus_one_fails_m3201',
  );
  assert.equal(
    packageDocument.scripts['m3:runtime-abi:quick'],
    'cargo test --locked -p zryna-ownership-runtime-abi',
  );
});
