import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

import ts from '@typescript/typescript6';
import Ajv2020 from 'ajv/dist/2020.js';

const adapterRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const schema = JSON.parse(
  await readFile(new URL('../../../schemas/zryna-syntax-v2.schema.json', import.meta.url)),
);
const validateSnapshot = new Ajv2020({ allErrors: true, strict: true }).compile(schema);

async function exchangeRaw(input, options = {}) {
  const child = spawn(process.execPath, ['src/worker.mjs'], {
    cwd: adapterRoot,
    stdio: ['pipe', 'pipe', 'pipe'],
    windowsHide: true,
    env: { ...process.env, ...options.env },
  });
  const stdout = [];
  const stderr = [];
  child.stdout.on('data', (chunk) => stdout.push(chunk));
  child.stderr.on('data', (chunk) => stderr.push(chunk));
  child.stdin.end(input);
  const [code] = await once(child, 'exit');
  assert.equal(code, 0, Buffer.concat(stderr).toString('utf8'));
  assert.equal(Buffer.concat(stderr).length, 0);
  return Buffer.concat(stdout)
    .toString('utf8')
    .split('\n')
    .filter(Boolean)
    .map((line) => JSON.parse(line));
}

async function exchange(requests, options) {
  return exchangeRaw(
    `${requests.map((request) => JSON.stringify(request)).join('\n')}\n`,
    options,
  );
}

function analyze(id, files) {
  return { id, method: 'analyze', params: { schema_version: 2, files } };
}

function assertValidSnapshot(response) {
  assert.equal(response.error, undefined);
  assert.equal(validateSnapshot(response.result), true, JSON.stringify(validateSnapshot.errors));
}

test('handshake and supported function bodies emit canonical protocol v2', async () => {
  const text =
    'export function add(a: i32, b: i32): i32 { return a + b; return -12; return true; }';
  const responses = await exchange([
    { id: 1, method: 'handshake' },
    analyze(2, [{ path: 'src/add.zry', text }]),
  ]);

  assert.deepEqual(responses[0], {
    id: 1,
    result: {
      provider: 'typescript-6',
      provider_version: ts.version,
      protocol_version: 2,
      capabilities: { module_resolution: false, semantic_diagnostics: false },
    },
  });
  assert.equal(ts.version, '6.0.3');
  assertValidSnapshot(responses[1]);
  const functionSyntax = responses[1].result.files[0].functions[0];
  assert.deepEqual(
    functionSyntax.parameters.map((parameter) => [parameter.name.text, parameter.type_syntax.kind]),
    [
      ['a', { kind: 'named', name: 'i32' }],
      ['b', { kind: 'named', name: 'i32' }],
    ],
  );
  assert.deepEqual(
    functionSyntax.body.expressions.map((expression) => expression.kind.kind),
    ['reference', 'reference', 'addition', 'i32-literal', 'bool-literal'],
  );
  assert.deepEqual(
    functionSyntax.body.statements.map((statement) => statement.kind.value),
    [2, 3, 4],
  );
  assert.equal(functionSyntax.body.expressions[3].kind.spelling, '-12');
});

test('checked adapter fixture stays byte-stable and schema-valid', async () => {
  const request = JSON.parse(
    await readFile(new URL('../../../tests/fixtures/typescript-adapter-v2-request.json', import.meta.url)),
  );
  const expected = (
    await readFile(new URL('../../../tests/fixtures/typescript-adapter-v2-result.json', import.meta.url), 'utf8')
  ).trim();
  const [response] = await exchange([request]);

  assertValidSnapshot(response);
  assert.equal(JSON.stringify(response.result), expected);
});

test('missing annotations and explicit any remain syntax for Zryna semantics', async () => {
  const text = 'export function identity(value: any) { return value; }';
  const [response] = await exchange([analyze(1, [{ path: 'src/types.zry', text }])]);

  assertValidSnapshot(response);
  const functionSyntax = response.result.files[0].functions[0];
  assert.deepEqual(functionSyntax.parameters[0].type_syntax.kind, {
    kind: 'named',
    name: 'any',
  });
  assert.deepEqual(functionSyntax.result_type.kind, { kind: 'missing' });
  assert.equal(functionSyntax.result_type.span.start, functionSyntax.result_type.span.end);
  assert.deepEqual(response.result.diagnostics, []);
});

test('every emitted span uses authoritative UTF-8 byte offsets', async () => {
  const prefix = '// 😀 cafe\u0301\r\n';
  const text = `${prefix}export function add(a: i32, b: i32): i32 { return a + b; }`;
  const [response] = await exchange([analyze(1, [{ path: 'src/unicode.zry', text }])]);

  assertValidSnapshot(response);
  const functionSyntax = response.result.files[0].functions[0];
  assert.equal(functionSyntax.span.start, Buffer.byteLength(prefix, 'utf8'));
  assert.equal(functionSyntax.span.end, Buffer.byteLength(text, 'utf8'));
  assert.equal(
    Buffer.from(text).subarray(functionSyntax.name.span.start, functionSyntax.name.span.end).toString(),
    'add',
  );
  for (const expression of functionSyntax.body.expressions) {
    assert.ok(expression.span.start <= expression.span.end);
    assert.ok(expression.span.end <= Buffer.byteLength(text, 'utf8'));
  }
});

test('file identifiers and output bytes are deterministic under shuffled input', async () => {
  const source = 'export function value(): i32 { return 1; }';
  const files = [
    { path: 'src/z.zry', text: source },
    { path: 'src/a.zry', text: source },
  ];
  const [first, second] = await exchange([
    analyze(1, files),
    analyze(2, [...files].reverse()),
  ]);

  assertValidSnapshot(first);
  assertValidSnapshot(second);
  assert.deepEqual(first.result, second.result);
  assert.deepEqual(
    first.result.files.map((file) => [file.id, file.path]),
    [
      [0, 'src/a.zry'],
      [1, 'src/z.zry'],
    ],
  );
});

test('TypeScript parse failures are located and never normalize recovery ASTs', async () => {
  const text = 'export function broken(: i32): i32 { return 1; }';
  const [response] = await exchange([analyze(1, [{ path: 'src/parse.zry', text }])]);

  assertValidSnapshot(response);
  assert.deepEqual(response.result.files[0].functions, []);
  assert.match(response.result.diagnostics[0].code, /^TS[0-9]+$/);
  assert.equal(response.result.diagnostics[0].severity, 'error');
  assert.equal(response.result.diagnostics[0].location.kind, 'source');
});

test('unsupported TypeScript syntax is exhaustive, located, and fail closed', async () => {
  const cases = [
    ['hidden', 'function hidden(): i32 { return 1; }'],
    ['default', 'export default function value(): i32 { return 1; }'],
    ['async', 'export async function value(): i32 { return 1; }'],
    ['generator', 'export function* value(): i32 { return 1; }'],
    ['class', 'export class Value {}'],
    ['variable', 'export const value = 1;'],
    ['destructure', 'export function value({ x }: any): i32 { return 1; }'],
    ['this-param', 'export function value(this: i32): i32 { return 1; }'],
    ['rest', 'export function value(...x: any): i32 { return 1; }'],
    ['optional', 'export function value(x?: i32): i32 { return 1; }'],
    ['default-param', 'export function value(x: i32 = 1): i32 { return x; }'],
    ['generic-function', 'export function value<T>(x: T): T { return x; }'],
    ['union-type', 'export function value(x: i32 | bool): i32 { return 1; }'],
    ['empty-return', 'export function value(): i32 { return; }'],
    ['if', 'export function value(): i32 { if (true) return 1; return 2; }'],
    ['call', 'export function value(): i32 { return value(); }'],
    ['multiply', 'export function value(a: i32): i32 { return a * 2; }'],
    ['property', 'export function value(a: any): any { return a.value; }'],
    ['parenthesized', 'export function value(a: i32): i32 { return (a); }'],
    ['escaped-name', 'export function \\u0076alue(): i32 { return 1; }'],
    ['arrow', 'export const value = () => 1;'],
  ];
  const files = cases.map(([name, text]) => ({ path: `src/${name}.zry`, text }));
  const [response] = await exchange([analyze(1, files)]);

  assertValidSnapshot(response);
  assert.ok(response.result.diagnostics.length >= cases.length);
  assert.ok(response.result.diagnostics.every((diagnostic) => diagnostic.code === 'ZRYNA-F2002'));
  assert.ok(response.result.diagnostics.every((diagnostic) => diagnostic.location.kind === 'source'));
  assert.ok(response.result.files.every((file) => file.functions.length === 0));
});

test('unsupported expression rollback leaves no orphan arena nodes', async () => {
  const text = 'export function bad(a: i32): i32 { return a + bad(); }';
  const [response] = await exchange([analyze(1, [{ path: 'src/orphan.zry', text }])]);

  assertValidSnapshot(response);
  assert.deepEqual(response.result.files[0].functions, []);
  assert.equal(response.result.diagnostics.length, 1);
  assert.match(response.result.diagnostics[0].message, /CallExpression/);
});

test('malformed requests, duplicate fields, paths, and source text are rejected', async () => {
  const invalidRequests = [
    { id: 1, method: 'handshake', extra: true },
    analyze(2, [{ path: 'src/a.zry', text: '', extra: true }]),
    { id: -1, method: 'handshake' },
    { id: 3, method: 'analyze', params: { schema_version: 1, files: [] } },
    analyze(4, [
      { path: 'src/A.zry', text: '' },
      { path: 'src/a.zry', text: '' },
    ]),
    analyze(5, [{ path: 'src/CON.zry', text: '' }]),
    analyze(6, [{ path: 'src/bad.zry', text: '\ud800' }]),
  ];
  const responses = await exchange(invalidRequests);
  assert.equal(responses.length, invalidRequests.length);
  assert.ok(responses.every((response) => response.error.code === 'ZRYNA-F1001'));
  assert.equal(responses[2].id, null);

  const duplicates = [
    '{"id":7,"method":"handshake","method":"analyze","params":{"schema_version":2,"files":[]}}',
    '{"id":8,"method":"analyze","params":{"schema_version":2,"schema_version":2,"files":[]}}',
  ];
  const duplicateResponses = await exchangeRaw(`${duplicates.join('\n')}\n`);
  assert.ok(duplicateResponses.every((response) => response.error.code === 'ZRYNA-F1001'));
  assert.ok(
    duplicateResponses.every((response) => /duplicate object field/.test(response.error.message)),
  );

  const [utf8Response] = await exchangeRaw(Buffer.from([0xff, 0x0a]));
  assert.equal(utf8Response.error.code, 'ZRYNA-F1001');
});

test('source-byte and CR/LF line budgets fail closed', async () => {
  const tooLarge = ' '.repeat(2 * 1024 * 1024 + 1);
  const tooManyCrLines = '\r'.repeat(100_000);
  const [bytesResponse, linesResponse] = await exchange([
    analyze(1, [{ path: 'src/bytes.zry', text: tooLarge }]),
    analyze(2, [{ path: 'src/lines.zry', text: tooManyCrLines }]),
  ]);

  assert.equal(bytesResponse.error.code, 'ZRYNA-F1002');
  assert.match(bytesResponse.error.message, /byte limit/);
  assert.equal(linesResponse.error.code, 'ZRYNA-F1002');
  assert.match(linesResponse.error.message, /line limit/);
});

test('transport and file-count limits reject +1 then recover', async () => {
  const [requestError, requestRecovery] = await exchangeRaw(
    `${' '.repeat(65)}\n{"id":1,"method":"handshake"}\n`,
    { env: { NODE_ENV: 'test', ZRYNA_TEST_REQUEST_BYTES: '64' } },
  );
  assert.equal(requestError.error.code, 'ZRYNA-F1002');
  assert.equal(requestRecovery.result.protocol_version, 2);

  const empty = (name) => ({ path: `src/${name}.zry`, text: '' });
  const [exact, overflow, recovery] = await exchange(
    [
      analyze(2, [empty('a'), empty('b')]),
      analyze(3, [empty('a'), empty('b'), empty('c')]),
      { id: 4, method: 'handshake' },
    ],
    { env: { NODE_ENV: 'test', ZRYNA_TEST_FILES: '2' } },
  );
  assertValidSnapshot(exact);
  assert.equal(exact.result.files.length, 2);
  assert.equal(overflow.error.code, 'ZRYNA-F1002');
  assert.equal(recovery.result.protocol_version, 2);
});

test('per-file and project source-byte limits enforce exact and +1 boundaries', async () => {
  const [fileExact, fileOverflow, fileRecovery] = await exchange(
    [
      analyze(1, [{ path: 'src/exact.zry', text: ' '.repeat(20) }]),
      analyze(2, [{ path: 'src/overflow.zry', text: ' '.repeat(21) }]),
      { id: 3, method: 'handshake' },
    ],
    { env: { NODE_ENV: 'test', ZRYNA_TEST_SOURCE_FILE_BYTES: '20' } },
  );
  assertValidSnapshot(fileExact);
  assert.equal(fileOverflow.error.code, 'ZRYNA-F1002');
  assert.equal(fileRecovery.result.protocol_version, 2);

  const [projectExact, projectOverflow] = await exchange(
    [
      analyze(4, [
        { path: 'src/a.zry', text: ' '.repeat(15) },
        { path: 'src/b.zry', text: ' '.repeat(15) },
      ]),
      analyze(5, [
        { path: 'src/a.zry', text: ' '.repeat(16) },
        { path: 'src/b.zry', text: ' '.repeat(15) },
      ]),
    ],
    { env: { NODE_ENV: 'test', ZRYNA_TEST_SOURCE_BYTES: '30' } },
  );
  assertValidSnapshot(projectExact);
  assert.equal(projectOverflow.error.code, 'ZRYNA-F1002');
});

test('per-file and project line limits enforce exact and +1 boundaries', async () => {
  const [fileExact, fileOverflow] = await exchange(
    [
      analyze(1, [{ path: 'src/exact.zry', text: '\n\n' }]),
      analyze(2, [{ path: 'src/overflow.zry', text: '\n\n\n' }]),
    ],
    { env: { NODE_ENV: 'test', ZRYNA_TEST_LINES_PER_FILE: '3' } },
  );
  assertValidSnapshot(fileExact);
  assert.equal(fileOverflow.error.code, 'ZRYNA-F1002');

  const [projectExact, projectOverflow] = await exchange(
    [
      analyze(3, [
        { path: 'src/a.zry', text: '\n' },
        { path: 'src/b.zry', text: '\n' },
      ]),
      analyze(4, [
        { path: 'src/a.zry', text: '\n\n' },
        { path: 'src/b.zry', text: '\n' },
      ]),
    ],
    { env: { NODE_ENV: 'test', ZRYNA_TEST_LINES_PER_PROJECT: '4' } },
  );
  assertValidSnapshot(projectExact);
  assert.equal(projectOverflow.error.code, 'ZRYNA-F1002');
});

test('per-file and project syntax-node limits enforce exact and +1 boundaries', async () => {
  const [fileExact, fileOverflow] = await exchange(
    [
      analyze(1, [{ path: 'src/exact.zry', text: '' }]),
      analyze(2, [{ path: 'src/overflow.zry', text: '1' }]),
    ],
    { env: { NODE_ENV: 'test', ZRYNA_TEST_NODES_PER_FILE: '2' } },
  );
  assertValidSnapshot(fileExact);
  assert.equal(fileOverflow.error.code, 'ZRYNA-F1002');

  const empty = (name) => ({ path: `src/${name}.zry`, text: '' });
  const [projectExact, projectOverflow, recovery] = await exchange(
    [
      analyze(3, [empty('a'), empty('b')]),
      analyze(4, [empty('a'), empty('b'), empty('c')]),
      { id: 5, method: 'handshake' },
    ],
    { env: { NODE_ENV: 'test', ZRYNA_TEST_NODES_PER_PROJECT: '4' } },
  );
  assertValidSnapshot(projectExact);
  assert.equal(projectOverflow.error.code, 'ZRYNA-F1002');
  assert.equal(recovery.result.protocol_version, 2);
});

test('response-byte overflow is replaced by a bounded error and the worker recovers', async () => {
  const [overflow, recovery] = await exchange(
    [
      analyze(1, [
        {
          path: 'src/value.zry',
          text: 'export function value(): i32 { return 1; }',
        },
      ]),
      { id: 2, method: 'handshake' },
    ],
    { env: { NODE_ENV: 'test', ZRYNA_TEST_RESPONSE_BYTES: '190' } },
  );
  assert.equal(overflow.error.code, 'ZRYNA-F1002');
  assert.equal(recovery.result.protocol_version, 2);
});

test('literal, Unicode-line, and parser-nesting limits fail closed', async () => {
  const longInteger = '1'.repeat(65);
  const manyUnicodeLines = '\u2028'.repeat(100_000);
  const deepParentheses = '('.repeat(513) + '1' + ')'.repeat(513);
  const [integerResponse, lineResponse, nestingResponse] = await exchange([
    analyze(1, [
      {
        path: 'src/integer.zry',
        text: `export function value(): i32 { return ${longInteger}; }`,
      },
    ]),
    analyze(2, [{ path: 'src/unicode-lines.zry', text: manyUnicodeLines }]),
    analyze(3, [
      {
        path: 'src/nesting.zry',
        text: `export function value(): i32 { return ${deepParentheses}; }`,
      },
    ]),
  ]);

  assertValidSnapshot(integerResponse);
  assert.deepEqual(integerResponse.result.files[0].functions, []);
  assert.match(integerResponse.result.diagnostics[0].message, /integer literal/);
  assert.equal(lineResponse.error.code, 'ZRYNA-F1002');
  assert.match(lineResponse.error.message, /line limit/);
  assert.equal(nestingResponse.error.code, 'ZRYNA-F1002');
  assert.match(nestingResponse.error.message, /parser nesting limit/);
});

test('generic parser nesting and JSON structure budgets recover for the next request', async () => {
  const nestedType = `${'A<'.repeat(600)}i32${'>'.repeat(600)}`;
  const nestedSource = `export function value(input: ${nestedType}): i32 { return 1; }`;
  const [nestingResponse, handshakeResponse] = await exchange([
    analyze(1, [{ path: 'src/generic-nesting.zry', text: nestedSource }]),
    { id: 2, method: 'handshake' },
  ]);
  assert.equal(nestingResponse.error.code, 'ZRYNA-F1002');
  assert.match(nestingResponse.error.message, /parser capacity/);
  assert.equal(handshakeResponse.result.protocol_version, 2);

  const deepJson = `${'['.repeat(9)}0${']'.repeat(9)}`;
  const manyTokens = `[${'0,'.repeat(25_001)}0]`;
  const raw = `${deepJson}\n${manyTokens}\n{"id":3,"method":"handshake"}\n`;
  const [depthResponse, tokenResponse, recovered] = await exchangeRaw(raw);
  assert.equal(depthResponse.error.code, 'ZRYNA-F1002');
  assert.match(depthResponse.error.message, /JSON depth limit/);
  assert.equal(tokenResponse.error.code, 'ZRYNA-F1002');
  assert.match(tokenResponse.error.message, /JSON token limit/);
  assert.equal(recovered.result.protocol_version, 2);
});

test('large string fields and sequential less-than syntax do not consume JSON or nesting depth', async () => {
  const denseSource = `//${'x'.repeat(60_000)}\nexport function value(): i32 { return 1; }`;
  const comparisons = Array.from(
    { length: 600 },
    (_, index) => `export function less${index}(a: any, b: any): bool { return a < b; }`,
  ).join('\n');
  const [denseResponse, comparisonResponse] = await exchange([
    analyze(1, [{ path: 'src/dense.zry', text: denseSource }]),
    analyze(2, [{ path: 'src/comparisons.zry', text: comparisons }]),
  ]);

  assertValidSnapshot(denseResponse);
  assert.equal(denseResponse.result.files[0].functions.length, 1);
  assertValidSnapshot(comparisonResponse);
  assert.equal(comparisonResponse.result.files[0].functions.length, 0);
  assert.equal(comparisonResponse.result.diagnostics.at(-1).code, 'ZRYNA-F2003');
});

test('diagnostics retain deterministic top-K entries and one terminal marker', async () => {
  const text = Array.from({ length: 300 }, (_, index) => `class Unsupported${index} {}`).join('\n');
  const [first, second] = await exchange([
    analyze(1, [{ path: 'src/many.zry', text }]),
    analyze(2, [{ path: 'src/many.zry', text }]),
  ]);

  assertValidSnapshot(first);
  assert.deepEqual(first.result, second.result);
  assert.equal(first.result.diagnostics.length, 256);
  assert.equal(first.result.diagnostics.at(-1).code, 'ZRYNA-F2003');
});

test('expression-depth exhaustion produces a stable provider error', async () => {
  const expression = Array.from({ length: 130 }, () => '1 + ').join('') + '1';
  const text = `export function deep(): i32 { return ${expression}; }`;
  const [response] = await exchange([analyze(1, [{ path: 'src/deep.zry', text }])]);

  assertValidSnapshot(response);
  assert.deepEqual(response.result.files[0].functions, []);
  assert.ok(response.result.diagnostics.some((diagnostic) => /expression depth/.test(diagnostic.message)));
});
