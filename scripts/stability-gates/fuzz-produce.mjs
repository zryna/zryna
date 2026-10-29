import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { readSafe } from './input.mjs';
import { createInterruptionScope } from './interruption.mjs';
import { execute } from './process.mjs';
import { hostName } from './registry.mjs';
import { snapshot, unchanged } from './source.mjs';
import { loadFuzzRegistry, REGISTRY_SHA256, ROOT } from './fuzz-registry.mjs';
import { childEnvironment, descriptor, document, inspectWithoutTokens, prepare, retain, successful } from './fuzz-files.mjs';
import { checkObservation, namedProof } from './fuzz-replay.mjs';
import { replayWitness } from './fuzz-witness.mjs';
import { resolvePnpmScript } from '../m4/observe-materials.mjs';

async function observe(output, gate, context) {
  const { root, run, interruption } = context;
  document(output, `started-${gate.id}.json`, { id: gate.id, executable: gate.executable, args: gate.args,
    witness: gate.test ? `${gate.id}.jsonl` : null });
  const env = childEnvironment(process.env, gate.test ? resolve(output, `${gate.id}.jsonl`) : undefined);
  const result = await run(gate.executable, gate.args, { root, timeoutMs: gate.timeoutMs, env, interruption });
  const { stdout, stderr, ...attempt } = result;
  const record = { id: gate.id, executable: gate.executable, args: gate.args, attempt,
    stdout: retain(output, `${gate.id}.stdout`, stdout),
    stderr: retain(output, `${gate.id}.stderr`, stderr), witness: null, totals: null };
  if (gate.test) {
    try {
      const bytes = readSafe(output, `${gate.id}.jsonl`, 16 * 1024 * 1024);
      record.witness = descriptor(`${gate.id}.jsonl`, bytes);
    } catch (error) { if (error.code !== 'ENOENT') throw error; }
  }
  document(output, `observed-${gate.id}.json`, record);
  successful(attempt, gate.timeoutMs);
  if (gate.test) record.totals = replayWitness(readSafe(output, record.witness.path, 16 * 1024 * 1024), gate);
  return record;
}

async function tools(output, registry, context) {
  const observations = [];
  for (const name of Object.keys(registry.toolchains).sort()) {
    assert.equal(context.interruption.signal, null, 'interrupted before tool observation');
    const executable = name === 'pnpm' ? process.execPath : name;
    const args = name === 'pnpm' ? [context.resolvePnpmScript(), '--version'] : ['--version'];
    const result = await context.run(executable, args, { root: context.root, timeoutMs: 30000,
      env: childEnvironment(), interruption: context.interruption });
    const { stdout, stderr, ...attempt } = result;
    const text = stdout.toString('utf8').trim();
    const version = ['rustc', 'cargo'].includes(name) ? text.split(' ')[1] : text;
    const record = { name, executable, args, version: version ?? null, attempt,
      stdout: retain(output, `tool-${name}.stdout`, stdout), stderr: retain(output, `tool-${name}.stderr`, stderr) };
    document(output, `observed-tool-${name}.json`, record);
    successful(attempt, 30000);
    assert(stdout.length <= 65536 && stderr.length <= 65536, 'bounded version observation');
    assert.equal(version, registry.toolchains[name], 'pinned tool version');
    observations.push(record);
  }
  return observations;
}

export async function collectFuzz(output, expectedCommit, options = {}) {
  const root = options.root ?? ROOT;
  const inspect = options.inspect ?? snapshot;
  const interruption = options.interruption ?? createInterruptionScope();
  const context = { root, interruption, run: options.run ?? execute,
    resolvePnpmScript: options.resolvePnpmScript ?? resolvePnpmScript };
  const results = [];
  let created = false;
  let stage = 'preparation';
  try {
    assert.match(expectedCommit, /^[a-f0-9]{40}$/);
    prepare(root, output);
    created = true;
    document(output, 'started.json', { expectedCommit, attempt: 1 });
    const { registry, bytes } = loadFuzzRegistry(root);
    retain(output, 'registry.json', bytes);
    const source = inspectWithoutTokens(inspect, root);
    assert.equal(source.commit, expectedCommit, 'selected source commit');
    document(output, 'source.json', source);
    const host = hostName();
    assert(registry.hosts.includes(host));
    stage = 'tools';
    const observedTools = await tools(output, registry, context);
    const gates = [...registry.campaigns, registry.fullOwner];
    for (const gate of gates) {
      stage = gate.id;
      assert.equal(interruption.signal, null, 'interrupted before gate');
      unchanged(source, inspectWithoutTokens(inspect, root));
      const record = await observe(output, gate, context);
      const stdout = checkObservation(output, record, gate);
      if (!gate.test) for (const name of registry.namedBoundaries) namedProof(stdout.toString('utf8'), name);
      results.push(record);
      document(output, `result-${gate.id}.json`, record);
      unchanged(source, inspectWithoutTokens(inspect, root));
    }
    assert.equal(interruption.signal, null, 'interrupted before receipt');
    const finalSource = inspectWithoutTokens(inspect, root);
    unchanged(source, finalSource);
    document(output, 'source-final.json', finalSource);
    const receipt = { format: 'zryna.m7.fuzz-campaign.v2', registrySha256: REGISTRY_SHA256,
      source, host, runtime: { nodePath: process.execPath, nodeVersion: process.version,
        platform: process.platform, architecture: process.arch }, tools: observedTools, results,
      closureStatus: 'blocked', globalM7: false };
    document(output, 'receipt.json', receipt);
    return receipt;
  } catch (error) {
    if (created) document(output, 'failure.json', { format: 'zryna.m7.fuzz-campaign-failure.v2', stage,
      expectedCommit, signal: interruption.signal, error: String(error.message).slice(0, 4096),
      completed: results.map(result => result.id), closureStatus: 'blocked', globalM7: false });
    error.signal = interruption.signal;
    throw error;
  } finally { if (!options.interruption) interruption.dispose(); }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    assert.equal(process.argv.length, 4, 'usage: fuzz-produce.mjs /absolute/new-output full-commit');
    await collectFuzz(process.argv[2], process.argv[3]);
  } catch (error) { console.error(error.message); process.exitCode = error.signal === 'SIGINT' ? 130 : error.signal === 'SIGTERM' ? 143 : 1; }
}
