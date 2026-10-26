// Mandatory trusted-checkout checks, outside every measured interval.
import assert from 'node:assert/strict';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { canonical, sha256 } from './input.mjs';
import { execute } from './process.mjs';
import { captureSourceCensus, sourceUnchanged } from './source-census-v2.mjs';

function save(output, name, value) {
  writeFileSync(join(output, name), canonical(value), { flag: 'wx', mode: 0o600 });
}
export async function runArchitectureCheck({ root, output, cli, environment }, run = execute) {
  const args = ['architecture', 'check'];
  const command = { executable: cli, args, cwd: root, timeoutMs: 60000 };
  save(output, 'architecture.command.json', command);
  const result = await run(cli, args, { root, env: environment, timeoutMs: 60000 });
  const logs = {};
  for (const stream of ['stdout', 'stderr']) {
    assert(Buffer.isBuffer(result[stream]));
    writeFileSync(join(output, `architecture.${stream}.raw`), result[stream], { flag: 'wx', mode: 0o600 });
    logs[stream] = { bytes: result[stream].length, sha256: sha256(result[stream]) };
  }
  save(output, 'architecture.result.json', { command, logs, exitCode: result.exitCode,
    signal: result.signal, error: result.error, elapsedMs: result.elapsedMs });
  assert.equal(result.error, null, 'architecture process completed without timeout/output/cleanup failure');
  assert.equal(result.signal, null); assert.equal(result.exitCode, 0, 'actual release CLI architecture check');
}
export function verifySourceAfterCollection({ root, output, tools, environment, before }, read = captureSourceCensus) {
  const after = read({ root, directory: join(output, 'source-after-round'), executable: tools.git, env: environment });
  save(output, 'source-after-round.json', after); // Retain mismatches before rejecting them.
  sourceUnchanged(before, after);
  return after;
}
