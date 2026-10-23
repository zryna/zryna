// Accepted #415 source rejections; provider error text is advisory and grants no syntax seal.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const adapter = new URL('../', import.meta.url);
const catalogue = JSON.parse(await readFile(
  new URL('../../../spec/language/generic-review-v1-fixtures.json', import.meta.url), 'utf8'));
const analyze = (text, id) => ({ id, method: 'analyze',
  params: { schema_version: 5, files: [{ path: 'main.zry', text }] } });

function exchange(requests) {
  const run = spawnSync(process.execPath, ['src/worker-v5.mjs'], {
    cwd: adapter, input: requests.map(JSON.stringify).join('\n') + '\n', encoding: 'utf8',
    timeout: 30_000, maxBuffer: 1024 * 1024, windowsHide: true,
  });
  assert.equal(run.error, undefined);
  assert.equal(run.status, 0, run.stderr);
  assert.equal(run.stderr, '');
  return run.stdout.trim().split('\n').map(JSON.parse);
}

for (const [id, wrapper, extra] of [
  ['three-parameters', false, ', U extends ZrynaValue'],
  ['three-arguments', true, ', String'],
]) {
  test(`v5 source rejection ${id} retains its catalogue code and UTF-8 primary token`, () => {
    const entry = catalogue.sourceCases.find(entry => entry.id === id);
    assert.equal(entry.parserErrors, 0);
    assert.equal(entry.primaryOccurrence, 0);
    for (const prefix of ['', '// π😀\r\n', '/* π😀 */\u2028']) {
      const body = wrapper ? `function probe(): i32 { ${entry.source} return 0; }` : entry.source;
      const text = prefix + body;
      const start = Buffer.byteLength(text.slice(0, text.indexOf(entry.primaryToken)));
      const end = start + Buffer.byteLength(entry.primaryToken);
      const responses = exchange([
        analyze(text, 2), analyze(text.replace(extra, ''), 3), { id: 4, method: 'handshake' },
      ]);
      assert.equal(responses[0].error?.code, entry.code, id);
      assert.equal(responses[0].result, undefined, 'no partial DTO or syntax seal');
      assert.match(responses[0].error.message, new RegExp(`file 0 bytes ${start}\\.\\.${end}(?:\\D|$)`));
      assert.equal(responses[1].error, undefined, JSON.stringify(responses[1].error));
      assert.equal(responses[1].result.schema_version, 5);
      assert.equal(responses[2].result.protocol_version, 5);
    }
  });
}

test('v5 reports the complete first-extra type argument and ignores later extra items', () => {
  for (const [text, code, primary] of [
    ['function f<T extends ZrynaValue, E extends ZrynaValue, U extends ZrynaValue, V extends ZrynaValue>(): i32 { return 0; }',
      'ZRYNA-D7001', 'U'],
    ['function f(): i32 { return select<i32, bool, String, i32>(7, true); }',
      'ZRYNA-M7001', 'String'],
    ['// π😀\r\nfunction f(): i32 { return select<i32, bool, Option<Result<i32,bool>>>(7, true); }',
      'ZRYNA-M7001', 'Option<Result<i32,bool>>'],
  ]) {
    const start = Buffer.byteLength(text.slice(0, text.indexOf(primary)));
    const end = start + Buffer.byteLength(primary);
    const [response] = exchange([analyze(text, 2)]);
    assert.equal(response.error?.code, code);
    assert.equal(response.result, undefined);
    assert.match(response.error.message, new RegExp(`file 0 bytes ${start}\\.\\.${end}(?:\\D|$)`));
  }
});

test('v5 outer arity rejects before nested arity or type-depth normalization', () => {
  const deep = 'Option<'.repeat(129) + 'i32' + '>'.repeat(129);
  for (const [first, second, third] of [
    ['i32', 'bool', 'Option<i32, bool, String>'],
    ['Option<i32, bool, String>', 'bool', 'String'],
    ['i32', 'Option<i32, bool, String>', 'String'],
    [deep, 'bool', 'String'],
    ['i32', deep, 'String'],
    ['i32', 'bool', deep],
  ]) {
    const prefix = `// π😀\r\nfunction f(): i32 { return select<${first}, ${second}, `;
    const text = `${prefix}${third}>(7, true); }`;
    const start = Buffer.byteLength(prefix);
    const end = start + Buffer.byteLength(third);
    const [rejected, recovered] = exchange([
      analyze(text, 2), analyze('function f(): i32 { return select<i32, bool>(7, true); }', 3),
    ]);
    assert.equal(rejected.error?.code, 'ZRYNA-M7001');
    assert.equal(rejected.result, undefined);
    assert.match(rejected.error.message, new RegExp(`file 0 bytes ${start}\\.\\.${end}(?:\\D|$)`));
    assert.equal(recovered.error, undefined, JSON.stringify(recovered.error));
    assert.equal(recovered.result.schema_version, 5);
  }
});

test('v5 malformed nonempty prefixes and absent outer close remain syntax errors', () => {
  for (const text of [
    'function f(): i32 { return select<i32 +, bool, String>(7, true); }',
    'function f(): i32 { return select<i32 foo, bool, String>(7, true); }',
    'function f<T ZrynaValue, E extends ZrynaValue, U extends ZrynaValue>(): i32 { return 0; }',
    'function f(value: Box<i32, bool, String, ): i32 { return 0; }',
    'function f(): i32 { return select<function<i32>, bool, String>(7, true); }',
    'function f(value: Box<function<i32>, bool, String>): i32 { return 0; }',
  ]) {
    const [response] = exchange([analyze(text, 2)]);
    assert.equal(response.error?.code, 'ZRYNA-F2002');
    assert.equal(response.result, undefined);
  }
});

test('v5 FixedArray outer cardinality precedes its element and length normalization', () => {
  for (const first of ['i32', 'Option<i32, bool, String>']) {
    const prefix = `// π😀\r\nfunction f(value: FixedArray<${first}, 2, `;
    const text = `${prefix}String>): i32 { return 0; }`;
    const start = Buffer.byteLength(prefix);
    const [response] = exchange([analyze(text, 2)]);
    assert.equal(response.error?.code, 'ZRYNA-M7001');
    assert.equal(response.result, undefined);
    assert.match(response.error.message,
      new RegExp(`file 0 bytes ${start}\\.\\.${start + 6}(?:\\D|$)`));
  }
});
