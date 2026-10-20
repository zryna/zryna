import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { PURE_H1_SOURCE } from './command-h1.mjs';

test('installed pure H1 uses exactly the existing registered bool program', () => {
  assert.equal(PURE_H1_SOURCE,
    readFileSync(new URL('../../examples/wasi-command/pure.zry', import.meta.url), 'utf8'));
});
