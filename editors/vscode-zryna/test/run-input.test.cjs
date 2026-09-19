'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const { sourceText, exportsIn, argumentError, selectRun } = require('../src/run-input.cjs');

const source = Buffer.from('export function add(a:i32,b:i32):i32{return a+b;}\nexport function one():i32{return 1;}');

test('export discovery excludes comments, strings, nested functions and unsupported signatures', () => {
  const text = '// export function fake():i32{}\n"export function fake():i32{}";\n'
    + 'function outer(){export function nested():i32{}}\nexport function flag(x:bool):bool{}\n' + source;
  assert.deepEqual(exportsIn(text), [{ name: 'add', parameters: ['a', 'b'] }, { name: 'one', parameters: [] }]);
  assert.throws(() => exportsIn('export function f():i32{} export function f():i32{}'), /Duplicate/);
  assert.throws(() => exportsIn('export function f(x:bool):bool{}'), /No exported/);
});

test('UTF-8 byte bound and canonical i32 bounds reject malformed input', () => {
  assert.equal(sourceText(Buffer.alloc(1024, 32)).length, 1024);
  assert.throws(() => sourceText(Buffer.alloc(1025)), /1024/);
  assert.throws(() => sourceText(Buffer.from([255])), /encoded/);
  for (const value of ['0', '-1', '-2147483648', '2147483647']) assert.equal(argumentError(value), undefined);
  for (const value of ['', '-0', '01', '+1', ' 1', '1.0', '1e2', '2147483648', '-2147483649', '7;echo x']) assert.ok(argumentError(value));
});

test('every picker cancellation returns without an invocation; arguments have no defaults', async () => {
  for (let stop = 0; stop < 4; stop++) {
    let step = 0;
    const window = {
      async showQuickPick(items) { return step++ === stop ? undefined : items[0]; },
      async showInputBox(options) { assert.equal(options.value, undefined); return step++ === stop ? undefined : '3'; },
    };
    assert.equal(await selectRun(window, source), undefined);
  }
  const selected = await selectRun({ showQuickPick: async items => items[0], showInputBox: async () => '9' }, source);
  assert.deepEqual(selected, { name: 'add', target: 'javascript', args: ['9', '9'] });
});
