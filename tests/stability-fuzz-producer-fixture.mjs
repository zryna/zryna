import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { collectFuzz } from '../scripts/stability-gates/fuzz-produce.mjs';
import { ROOT } from '../scripts/stability-gates/fuzz-registry.mjs';
import { encode, registry, rustLog, witnessRecords } from './stability-fuzz-fixture.mjs';

export const commit = 'a'.repeat(40);
export const source = { repository: 'https://github.com/zryna/zryna', commit, tree: 'b'.repeat(40),
  inventorySha256: 'c'.repeat(64), files: 1, bytes: 1 };

export function fixture(t, failure) {
  const parent = mkdtempSync(resolve(tmpdir(), 'zryna-fuzz-validator-'));
  t.after(() => rmSync(parent, { recursive: true, force: true }));
  const output = resolve(parent, 'original');
  const calls = [];
  const interruption = { signal: null };
  const run = async (executable, args, options) => {
    calls.push({ executable, args, witness: options.env.ZRYNA_M7_FUZZ_WITNESS });
    assert(!Object.keys(options.env).some(key => /^(GH_TOKEN|GITHUB_TOKEN)$/i.test(key)));
    let stdout;
    let error = null;
    let signal = null;
    if (args.at(-1) === '--version') {
      const name = executable === process.execPath ? 'pnpm' : executable;
      const version = registry.toolchains[name];
      stdout = Buffer.from(['cargo', 'rustc'].includes(name) ? `${name} ${version}\n` : `${version}\n`);
    } else {
      const gate = [...registry.campaigns, registry.fullOwner].find(value => value.executable === executable &&
        JSON.stringify(value.args) === JSON.stringify(args));
      assert(gate, 'only immutable registered command');
      const applies = failure && (typeof failure === 'string' || failure.gate === gate.id);
      if (gate.test) {
        const records = witnessRecords(gate);
        if (applies && typeof failure === 'object') failure.mutate?.(records);
        writeFileSync(options.env.ZRYNA_M7_FUZZ_WITNESS,
          encode(applies ? records.slice(0, typeof failure === 'object' ? failure.rows : 2) : records), { flag: 'wx' });
      } else assert.equal(options.env.ZRYNA_M7_FUZZ_WITNESS, undefined);
      stdout = rustLog(gate);
      if (applies && typeof failure === 'object') return { exitCode: 101, signal: null, error: null,
        elapsedMs: 1, stdout: Buffer.from('synthetic original oracle panicked\n'), stderr: Buffer.alloc(0) };
      if (applies) {
        error = failure;
        if (failure === 'ECANCELED') { interruption.signal = 'SIGTERM'; signal = 'SIGTERM'; }
      }
    }
    return { exitCode: error ? null : 0, signal, error, elapsedMs: 1, stdout, stderr: Buffer.from('') };
  };
  return { output, parent, calls, interruption,
    collect: () => collectFuzz(output, commit, { root: ROOT, inspect: () => structuredClone(source), run, interruption,
      resolvePnpmScript: () => resolve(parent, 'bin/pnpm.cjs') }) };
}
