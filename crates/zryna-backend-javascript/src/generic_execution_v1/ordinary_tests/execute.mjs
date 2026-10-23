import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';
const key = Symbol.for('zryna.generic.ownership.runtime.v1');
Object.defineProperty(globalThis, key, { get() { throw new Error('PRIVATE-INJECTION-IS-NOT-ORDINARY-AUTHORITY'); } });
const module = await import(pathToFileURL(process.argv[2]).href);
assert.deepEqual(Object.keys(module), ['entry']);
for (let retry=0; retry<3; retry++) assert.equal(module.entry(),Number(process.argv[3]));
assert.throws(() => module.entry(1));
assert.equal(module.entry(),Number(process.argv[3]));
