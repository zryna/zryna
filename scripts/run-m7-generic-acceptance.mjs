import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execute } from './stability-gates/process.mjs';
import { commandsFor, loadRegistry, platformName, REGISTRY_SHA256, verifyTestOutput } from './m7-generic-acceptance/commands.mjs';
import { createEvidence, retain, sourceIdentity, tools, writeReceipt } from './m7-generic-acceptance/evidence.mjs';

const SCRIPT = fileURLToPath(import.meta.url);
const ROOT = resolve(import.meta.dirname, '..');
// Existing trusted-command tree cleanup/log mechanism only; no stability/#418 registry or claims.
const COMMAND_TIMEOUT_MS = 25 * 60 * 1000;
function options(argv) {
  if (argv.length !== 4) throw new Error('usage: run-m7-generic-acceptance.mjs --evidence ABS --expected-source-revision FULL_SHA');
  const result = {};
  for (let i = 0; i < argv.length; i += 2) {
    if (!['--evidence', '--expected-source-revision'].includes(argv[i]) || argv[i] in result) {
      throw new Error('unknown or repeated M7 runner option');
    }
    result[argv[i]] = argv[i + 1];
  }
  return { evidence: result['--evidence'], revision: result['--expected-source-revision'] };
}
export async function runGenericAcceptance(config) {
  createEvidence(ROOT, config.evidence);
  const receipt = { format: 'zryna.m7-generic-acceptance.v1', registrySha256: REGISTRY_SHA256,
    status: 'failed', profile: 'bounded-generics-v1', host: { platform: process.platform, arch: process.arch },
    source: null, sourceAfter: null, postcheckError: null, pullRequestHead: process.env.ZRYNA_M7_PR_HEAD || null,
    workflow: { repository: process.env.GITHUB_REPOSITORY || null, runId: process.env.GITHUB_RUN_ID || null,
      attempt: process.env.GITHUB_RUN_ATTEMPT || null, job: process.env.GITHUB_JOB || null,
      precedingM0Outcome: process.env.ZRYNA_M7_UPSTREAM_M0_RESULT || null },
    inheritedCoverage: null, tools: [], commands: [], error: null,
    claims: 'This host packet only. Other platform, canonical M0 logs and final M0 aggregate/preflight remain separate required evidence.' };
  let failure;
  try {
    const registry = loadRegistry(ROOT);
    const platform = platformName();
    const commands = commandsFor(registry, platform);
    receipt.host.registryPlatform = platform;
    receipt.host.nativeExecutionSupported = platform === 'linux';
    if (receipt.workflow.precedingM0Outcome !== 'success') {
      throw new Error('the unchanged canonical M0 step must succeed first; supplied workflow metadata must be independently corroborated');
    }
    if (receipt.pullRequestHead !== null && !/^[a-f0-9]{40}$/u.test(receipt.pullRequestHead)) throw new Error('invalid distinct PR-head observation');
    receipt.source = sourceIdentity(ROOT, config.revision);
    receipt.tools = tools(ROOT, registry);
    receipt.inheritedCoverage = registry.inherited;
    for (const command of commands) {
      console.log(`[M7 ${platform}] ${command.id}`);
      const result = await execute(command.executable, command.args, { root: ROOT,
        timeoutMs: COMMAND_TIMEOUT_MS, env: { ...process.env, CARGO_TERM_COLOR: 'never',
          NODE: process.execPath, ZRYNA_TEST_NODE: process.execPath } });
      const row = { id: command.id, argv: [command.executable, ...command.args],
        exitCode: result.exitCode, signal: result.signal, error: result.error, elapsedMs: result.elapsedMs,
        stdout: retain(config.evidence, `logs/${command.id}.stdout.log`, result.stdout),
        stderr: retain(config.evidence, `logs/${command.id}.stderr.log`, result.stderr), tests: null };
      receipt.commands.push(row);
      process.stdout.write(result.stdout); process.stderr.write(result.stderr);
      if (result.error || result.signal || result.exitCode !== 0) throw new Error(`${command.id}: actual command failed`);
      row.tests = verifyTestOutput(result.stdout.toString('utf8'), result.stderr.toString('utf8'), command, platform);
    }
  } catch (error) {
    failure = error;
    receipt.error = error instanceof Error ? error.message : String(error);
  }
  if (receipt.source !== null) {
    try {
      receipt.sourceAfter = sourceIdentity(ROOT, config.revision);
      if (JSON.stringify(receipt.sourceAfter) !== JSON.stringify(receipt.source)) throw new Error('source changed during M7 packet');
      const registry = loadRegistry(ROOT);
      tools(ROOT, registry);
    } catch (error) {
      failure ??= error;
      receipt.postcheckError = error instanceof Error ? error.message : String(error);
      receipt.error ??= receipt.postcheckError;
    }
  }
  if (!failure) receipt.status = 'host-packet-passed';
  writeReceipt(config.evidence, receipt);
  if (failure) throw failure;
  const count = receipt.commands.reduce((n, row) => n + row.tests.executed, 0);
  console.log(`M7 actual ${receipt.host.platform}/${receipt.host.arch}: ${count} executed, 0 failed, 0 ignored. Other platform and inherited gates require separate evidence.`);
  return receipt;
}
if (process.argv[1] && resolve(process.argv[1]) === SCRIPT) {
  runGenericAcceptance(options(process.argv.slice(2))).catch(error => {
    console.error(error instanceof Error ? error.message : String(error)); process.exitCode = 1;
  });
}
