import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {pathToFileURL} from 'node:url';
import {join} from 'node:path';
const [directory, name] = process.argv.slice(2);
const bytes = readFileSync(join(directory, 'SCALAR_EXPECTED.json'));
assert.equal(createHash('sha256').update(bytes).digest('hex'), '1d49c3c77c20d7a0e577852ff9d2fcf6b458b6de22757e2d563f685dbcf08db4');
const expected = JSON.parse(bytes).cases.find(c => c.case === name);
assert(expected);
Object.defineProperty(globalThis, Symbol.for('zryna.generic.ownership.runtime.v1'), {get() { throw new Error('ambient runtime cannot authorize ordinary output'); }});
const js = await import(pathToFileURL(join(directory, 'program.mjs')).href);
assert.deepEqual(Object.keys(js), ['entry']);
const wasm = new WebAssembly.Instance(new WebAssembly.Module(readFileSync(join(directory, 'program.wasm'))), {});
assert.deepEqual(Object.keys(wasm.exports), ['entry']);
const actual = [];
for (const [target, entry] of [['javascript', js.entry], ['webassembly', wasm.exports.entry]]) {
  for (const run of expected.runs) {
    for (let repeat = 0; repeat < 2; repeat++) {
      if (run.bounds_trap) {
        assert.throws(() => entry(...run.arguments), error => target === 'javascript'
          ? error instanceof Error && error.message === 'zryna.trap.bounds-v1'
          : error instanceof WebAssembly.RuntimeError && /unreachable/.test(error.message));
        actual.push({target, arguments: run.arguments, repeat, trap: 'bounds'});
        const pristine = expected.runs.find(r => !r.bounds_trap);
        assert.equal(entry(...pristine.arguments), pristine.result);
        actual.push({target, arguments: pristine.arguments, repeat, retry: true, returned: pristine.result});
      } else {
        const returned = entry(...run.arguments);
        assert.equal(returned, run.result);
        actual.push({target, arguments: run.arguments, repeat, returned});
      }
    }
  }
}
console.log(JSON.stringify({case: name, actual, pristineRetries: true}));
