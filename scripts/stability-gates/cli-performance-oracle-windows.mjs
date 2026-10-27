// Unmeasured correctness still requires the qualified helper's complete Job cleanup predicate.
import assert from 'node:assert/strict';
import { join } from 'node:path';
import { MAX_LOG, readSafe } from './input.mjs';
import { execute } from './process.mjs';
import { flat, helperKeys } from './windows-job-metrics-v1/calibration/protocol.mjs';
import { checkedWindowsMetrics } from './cli-performance-windows-metrics.mjs';
export function validateWindowsOracleResult(result, call, readMetadata) {
  assert.equal(result.readError, null, 'complete helper sample files');
  assert.equal(result.outer.error, null); assert.equal(result.outer.signal, null); assert.equal(result.outer.exitCode, 0);
  assert.equal(result.outer.stdout.length, 0); assert.equal(result.outer.stderr.length, 0);
  // QPC/COMMIT values are validated using the original predicate, then discarded as measurements.
  checkedWindowsMetrics(readMetadata(), { hostWallGuardMs: 60000 }, call.args.length + 1,
    result.stdout, result.stderr);
}
export function windowsOracleRunner({ output, tools, root, environment }) {
  return {
    run: async (call, id) => {
      const sampleDirectory = join(output, `${id}.sample`);
      const args = ['--timeout-ms', '60000', '--output', sampleDirectory, '--', call.file, ...call.args];
      const outer = await execute(tools.helper, args, { root, env: environment, timeoutMs: 71000 });
      const result = { outer, sampleDirectory, outerCall: { file: tools.helper, args, timeoutMs: 71000 },
        stdout: Buffer.alloc(0), stderr: Buffer.alloc(0), metadata: Buffer.alloc(0), readError: null };
      // Preserve every readable stream independently even when another sample file is missing/oversize.
      const errors = [];
      for (const [key, name, limit] of [['metadata', 'result.json', 4096],
        ['stdout', 'stdout.raw', MAX_LOG], ['stderr', 'stderr.raw', MAX_LOG]]) {
        try { result[key] = readSafe(sampleDirectory, name, limit); }
        catch (error) { errors.push(`${key}: ${String(error.message).slice(0, 1024)}`); }
      }
      result.readError = errors.length ? errors.join('; ') : null;
      return result;
    },
    validateResult: (result, call) => validateWindowsOracleResult(result, call, () => {
      const parsed = flat(join(result.sampleDirectory, 'result.json'), 4096, helperKeys);
      assert(readSafe(result.sampleDirectory, 'result.json', 4096).equals(result.metadata), 'parsed retained helper bytes');
      return parsed;
    }),
  };
}
