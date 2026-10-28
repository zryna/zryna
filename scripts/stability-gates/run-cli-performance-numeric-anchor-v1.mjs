// Fixed nonmeasurement entry point; a retained verification.json is an audit record, not a key.
import assert from 'node:assert/strict';
import { existsSync, mkdirSync, realpathSync, writeFileSync } from 'node:fs';
import { dirname, isAbsolute, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { canonical } from './input.mjs';
import { reconcileCompletedNumericCampaign, inspectCompletedNumericCampaign, closeCompletedNumericCampaign }
  from './cli-performance-completed-campaign-v1.mjs';
import { numericRunId } from './cli-performance-anchor-context-v1.mjs';
import { NUMERIC_ANCHOR_FILES } from './cli-performance-anchor-documents-v1.mjs';
import { buildNumericCampaignAnchor, verifyNumericCampaignAnchor, inspectNumericCampaignAnchor,
  closeNumericCampaignAnchor } from './cli-performance-campaign-anchor-v1.mjs';
export function numericAnchorArguments(args) {
  assert(Array.isArray(args));
  const build = args[1] === 'build-anchor', compare = args[1] === 'compare';
  assert.deepEqual(args.filter((_, index) => index % 2 === 0), build
    ? ['--mode', '--baseline-run-id', '--output']
    : compare ? ['--mode', '--anchor-run-id', '--heldout-run-id', '--output']
      : ['--mode', '--anchor-run-id', '--platform', '--output']);
  assert.equal(args.length, build ? 6 : 8);
  assert.equal(args[1], build ? 'build-anchor' : compare ? 'compare' : 'verify-anchor');
  const runId = numericRunId(args[3]), output = args.at(-1);
  assert(typeof output === 'string' && isAbsolute(output) && output === resolve(output));
  const platform = build || compare ? null : args[5];
  if (!build && !compare) assert(['linux-x86_64', 'windows-x86_64'].includes(platform));
  return { mode: args[1], runId, output, platform, ...(compare ? { heldoutRunId: numericRunId(args[5]) } : {}) };
}
export async function runNumericAnchor(args) {
  assert.equal(arguments.length, 1);
  const selected = numericAnchorArguments(args), root = realpathSync.native(process.cwd());
  const delta = relative(root, selected.output);
  assert(delta && (isAbsolute(delta) || delta.split(/[\\/]/u)[0] === '..'), 'fresh output outside selected source');
  assert.equal(realpathSync.native(dirname(selected.output)), dirname(selected.output), 'private real output parent');
  assert(!existsSync(selected.output), 'create-only anchor output');
  mkdirSync(selected.output, { mode: 0o700 });
  try {
    if (selected.mode === 'build-anchor') {
      const files = await buildNumericCampaignAnchor({ root, baselineRunId: selected.runId });
      assert.deepEqual([...files.keys()].sort(), NUMERIC_ANCHOR_FILES);
      for (const name of NUMERIC_ANCHOR_FILES)
        writeFileSync(join(selected.output, name), files.get(name), { flag: 'wx', mode: 0o600 });
    } else if (selected.mode === 'compare') {
      const key = await reconcileCompletedNumericCampaign({ root, anchorRunId: selected.runId,
        heldoutRunId: selected.heldoutRunId });
      try {
        writeFileSync(join(selected.output, 'comparison.json'), canonical(inspectCompletedNumericCampaign(key)),
          { flag: 'wx', mode: 0o600 });
      } finally { closeCompletedNumericCampaign(key); }
    } else {
      const key = await verifyNumericCampaignAnchor({ root, anchorRunId: selected.runId, platform: selected.platform });
      try {
        writeFileSync(join(selected.output, 'verification.json'), canonical(inspectNumericCampaignAnchor(key)),
          { flag: 'wx', mode: 0o600 });
      } finally { closeNumericCampaignAnchor(key); }
    }
  } catch (error) {
    if (error.numericComparison)
      writeFileSync(join(selected.output, 'numeric-comparison-failure.json'), canonical(error.numericComparison),
        { flag: 'wx', mode: 0o600 });
    const failure = { format: 'zryna.cli-performance-numeric-anchor-failure.v1',
      mode: selected.mode, runId: selected.runId, message: String(error.message).slice(0, 1024),
      sourceAuthenticated: false, proofQualified: false, performanceQualified: false,
      baselineAdmitted: false, thresholdAuthority: null, closureStatus: 'blocked' };
    writeFileSync(join(selected.output, 'failure.json'), canonical(failure), { flag: 'wx', mode: 0o600 });
    throw error;
  }
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  runNumericAnchor(process.argv.slice(2)).catch(error => {
    process.stderr.write(`Numeric anchor failed: ${String(error.message).slice(0, 1024)}\n`); process.exitCode = 1;
  });
}
