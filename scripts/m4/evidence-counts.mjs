import assert from 'node:assert/strict';
import { reject } from './evidence-input.mjs';
export { requireNodeTests } from './evidence-node.mjs';

const row = /^test ([^\n]+) \.\.\. (ok|FAILED|ignored(?:, [^\n]*)?)$/gm;
const summary = /^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out; finished in [^\n]+$/gm;
export function rustAccounting(output, selector = null) {
  const rows = [...output.matchAll(row)].map(match => ({ name: match[1], outcome: match[2] }));
  const summaries = [...output.matchAll(summary)].map(match => ({ status: match[1],
    passed: Number(match[2]), failed: Number(match[3]), ignored: Number(match[4]), measured: Number(match[5]) }));
  if (!summaries.length || !rows.length) reject('missing actual named Rust rows or summaries');
  if ((output.match(/^test result:.*$/gm) ?? []).length !== summaries.length) reject('unparsed or truncated Rust summary');
  const declared = [...output.matchAll(/^running (\d+) tests?$/gm)].map(match => Number(match[1]));
  if (declared.length !== summaries.length || summaries.some((item, index) =>
    declared[index] !== item.passed + item.failed + item.ignored + item.measured)) reject('Rust running block/footer count mismatch');
  if (summaries.some(item => item.status !== 'ok' || item.failed || item.measured)) reject('failed or measured Rust suite');
  const total = key => summaries.reduce((sum, item) => sum + item[key], 0);
  assert.equal(rows.filter(item => item.outcome === 'ok').length, total('passed'), 'Rust passing row accounting');
  assert.equal(rows.filter(item => item.outcome.startsWith('ignored')).length, total('ignored'), 'Rust ignored row accounting');
  if (!total('passed') || rows.some(item => item.outcome === 'FAILED')) reject('zero passing or failed named Rust rows');
  if (selector !== null) {
    assert.equal(summaries.length, 1, 'exact Rust proof summary count');
    assert.equal(rows.length, 1, 'exact Rust proof named row count');
    assert.deepEqual(rows[0], { name: selector, outcome: 'ok' }, 'exact named Rust proof');
    assert.equal(total('passed'), 1, 'exact passing count');
    assert.equal(total('ignored'), 0, 'selected ignored test actually executed');
    assert.deepEqual(output.match(/^running \d+ tests?$/gm), ['running 1 test'], 'Rust selected test count');
  }
  return { passed: total('passed'), ignored: total('ignored'), names: rows.filter(item => item.outcome === 'ok').map(item => item.name) };
}
