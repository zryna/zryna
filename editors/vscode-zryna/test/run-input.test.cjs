'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const { sourceText, exportsIn, argumentError, selectRun } = require('../src/run-input.cjs');

const source = Buffer.from('export function add(a:i32,b:i32):i32{return a+b;}\nexport function one():i32{return 1;}');

test('export discovery excludes comments, strings, nested functions and profile-incompatible signatures', () => {
  const text = '// export function fake():i32{}\n"export function fake():i32{}";\n'
    + 'function outer(){export function nested():i32{}}\nexport function flag(x:bool):bool{}\n' + source;
  assert.deepEqual(exportsIn(text), [
    { name: 'add', parameters: [{ name: 'a', type: 'i32' }, { name: 'b', type: 'i32' }], resultType: 'i32' },
    { name: 'one', parameters: [], resultType: 'i32' },
  ]);
  assert.deepEqual(exportsIn('export function flag(x:bool):bool{}', 'control-flow-v1'), [
    { name: 'flag', parameters: [{ name: 'x', type: 'bool' }], resultType: 'bool' },
  ]);
  assert.throws(() => exportsIn('export function f():i32{} export function f():i32{}'), /Duplicate/);
  assert.throws(() => exportsIn('export function f(x:bool):bool{}'), /No exported/);
});

test('UTF-8 byte bound and canonical i32 bounds reject malformed input', () => {
  assert.equal(sourceText(Buffer.alloc(1024, 32)).length, 1024);
  assert.throws(() => sourceText(Buffer.alloc(1025)), /1024/);
  assert.equal(sourceText(Buffer.alloc(1025, 32), 'control-flow-v1').length, 1025);
  assert.throws(() => sourceText(Buffer.alloc(2 * 1024 * 1024 + 1), 'control-flow-v1'), /2097152/);
  assert.throws(() => sourceText(Buffer.from([255])), /encoded/);
  for (const value of ['0', '-1', '-2147483648', '2147483647']) assert.equal(argumentError(value), undefined);
  for (const value of ['', '-0', '01', '+1', ' 1', '1.0', '1e2', '2147483648', '-2147483649', '7;echo x']) assert.ok(argumentError(value));
  for (const value of ['true', 'false']) assert.equal(argumentError(value, 'bool'), undefined);
  for (const value of ['True', '0', '1', 'false;echo x']) assert.ok(argumentError(value, 'bool'));
});

test('every picker cancellation returns without an invocation; arguments have no defaults', async () => {
  for (let stop = 0; stop < 5; stop++) {
    let step = 0;
    const window = {
      async showQuickPick(items) { return step++ === stop ? undefined : items[0]; },
      async showInputBox(options) { assert.equal(options.value, undefined); return step++ === stop ? undefined : '3'; },
    };
    assert.equal(await selectRun(window, source), undefined);
  }
  const selected = await selectRun({ showQuickPick: async items => items[0], showInputBox: async () => '9' }, source);
  assert.deepEqual(selected, { profile: 'i32-v1', name: 'add', target: 'javascript',
    args: [{ type: 'i32', value: '9' }, { type: 'i32', value: '9' }], resultType: 'i32' });
});

test('control-flow picker binds bool and i32 arguments without defaults', async () => {
  const bytes = Buffer.from('export function choose(yes:bool, value:i32):bool{return yes;}');
  let prompt = 0;
  const selected = await selectRun({
    showQuickPick: async (items, options) => options.title.includes('profile') ? items[1] : items[0],
    showInputBox: async options => { assert.equal(options.value, undefined); return ['false', '-8'][prompt++]; },
  }, bytes);
  assert.deepEqual(selected, { profile: 'control-flow-v1', name: 'choose', target: 'javascript',
    args: [{ type: 'bool', value: 'false' }, { type: 'i32', value: '-8' }], resultType: 'bool' });
});
