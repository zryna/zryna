import assert from 'node:assert/strict';
import { realpathSync } from 'node:fs';
import { parse, relative, resolve, sep } from 'node:path';
import { parseArgs } from 'node:util';
import { readSafe } from './input.mjs';
import { runNumericHostedRound } from './cli-performance-hosted-adapter.mjs';

const { values } = parseArgs({ options: { input: { type: 'string' }, output: { type: 'string' } }, strict: true });
assert(values.input && values.output, 'explicit input and fresh output required');
assert.equal(process.version, 'v22.22.1');
const path = realpathSync(values.input);
const base = parse(path).root;
const bytes = readSafe(base, relative(base, path).split(sep).join('/'));
const input = JSON.parse(bytes.toString('utf8'));
assert.deepEqual(Object.keys(input).sort(), ['calibrationDirectory', 'environment', 'expectedHead',
  'expectedTree', 'root', 'round', 'tools'].sort());
for (const key of ['expectedHead', 'expectedTree']) assert.match(input[key], /^[a-f0-9]{40}$/u);
await runNumericHostedRound(input, resolve(values.output));
