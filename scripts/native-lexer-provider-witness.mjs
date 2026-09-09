import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
import process from 'node:process';
import { resolve } from 'node:path';

const requireFromAdapter = createRequire(
  new URL('../adapters/typescript-6/package.json', import.meta.url),
);
const ts = requireFromAdapter('@typescript/typescript6');

const ROOT = resolve(import.meta.dirname, '..');
const WORKER = resolve(ROOT, 'adapters/typescript-6/src/worker-v4.mjs');
const EXPECTED_PROVIDER_VERSION = '6.0.3';
const MAX_INPUT_BYTES = 9 * 1024 * 1024;

assert.equal(ts.version, EXPECTED_PROVIDER_VERSION, 'exact TypeScript provider scanner version');

const canonicalKinds = new Map([
  [ts.SyntaxKind.Identifier, 'identifier'],
  [ts.SyntaxKind.NumericLiteral, 'decimal-integer'],
  [ts.SyntaxKind.StringLiteral, 'string-literal'],
  [ts.SyntaxKind.OpenBraceToken, 'open-brace'],
  [ts.SyntaxKind.CloseBraceToken, 'close-brace'],
  [ts.SyntaxKind.OpenBracketToken, 'open-bracket'],
  [ts.SyntaxKind.CloseBracketToken, 'close-bracket'],
  [ts.SyntaxKind.OpenParenToken, 'open-paren'],
  [ts.SyntaxKind.CloseParenToken, 'close-paren'],
  [ts.SyntaxKind.ColonToken, 'colon'],
  [ts.SyntaxKind.SemicolonToken, 'semicolon'],
  [ts.SyntaxKind.CommaToken, 'comma'],
  [ts.SyntaxKind.DotToken, 'dot'],
  [ts.SyntaxKind.LessThanToken, 'less-than'],
  [ts.SyntaxKind.LessThanEqualsToken, 'less-equal'],
  [ts.SyntaxKind.GreaterThanToken, 'greater-than'],
  [ts.SyntaxKind.GreaterThanEqualsToken, 'greater-equal'],
  [ts.SyntaxKind.EqualsToken, 'equals'],
  [ts.SyntaxKind.EqualsGreaterThanToken, 'fat-arrow'],
  [ts.SyntaxKind.EqualsEqualsEqualsToken, 'strict-equal'],
  [ts.SyntaxKind.ExclamationEqualsEqualsToken, 'strict-not-equal'],
  [ts.SyntaxKind.PlusToken, 'plus'],
  [ts.SyntaxKind.MinusToken, 'minus'],
  [ts.SyntaxKind.AsteriskToken, 'asterisk'],
]);

for (const [spelling, kind] of [
  ['as', ts.SyntaxKind.AsKeyword],
  ['const', ts.SyntaxKind.ConstKeyword],
  ['else', ts.SyntaxKind.ElseKeyword],
  ['export', ts.SyntaxKind.ExportKeyword],
  ['extends', ts.SyntaxKind.ExtendsKeyword],
  ['false', ts.SyntaxKind.FalseKeyword],
  ['from', ts.SyntaxKind.FromKeyword],
  ['function', ts.SyntaxKind.FunctionKeyword],
  ['if', ts.SyntaxKind.IfKeyword],
  ['import', ts.SyntaxKind.ImportKeyword],
  ['interface', ts.SyntaxKind.InterfaceKeyword],
  ['let', ts.SyntaxKind.LetKeyword],
  ['return', ts.SyntaxKind.ReturnKeyword],
  ['true', ts.SyntaxKind.TrueKeyword],
  ['while', ts.SyntaxKind.WhileKeyword],
]) canonicalKinds.set(kind, `keyword-${spelling}`);

function exactKeys(value, keys, label) {
  assert(value !== null && typeof value === 'object' && !Array.isArray(value), `${label} object`);
  assert.deepEqual(Object.keys(value).sort(), [...keys].sort(), `${label} fields`);
}

function byteOffsets(text) {
  const offsets = new Uint32Array(text.length + 1);
  let bytes = 0;
  for (let index = 0; index < text.length; index += 1) {
    offsets[index] = bytes;
    const codeUnit = text.charCodeAt(index);
    if (codeUnit <= 0x7f) bytes += 1;
    else if (codeUnit <= 0x7ff) bytes += 2;
    else if (codeUnit >= 0xd800 && codeUnit <= 0xdbff) {
      offsets[index + 1] = 0xffff_ffff;
      index += 1;
      bytes += 4;
    } else bytes += 3;
  }
  offsets[text.length] = bytes;
  return offsets;
}

function scan(file) {
  const offsets = byteOffsets(file.text);
  const scanner = ts.createScanner(
    ts.ScriptTarget.Latest,
    true,
    ts.LanguageVariant.Standard,
    file.text,
  );
  const tokens = [];
  for (let kind = scanner.scan(); kind !== ts.SyntaxKind.EndOfFileToken; kind = scanner.scan()) {
    const start = scanner.getTokenPos();
    const end = scanner.getTextPos();
    assert.notEqual(offsets[start], 0xffff_ffff, 'provider token start splits a surrogate pair');
    assert.notEqual(offsets[end], 0xffff_ffff, 'provider token end splits a surrogate pair');
    tokens.push({
      raw_provider_kind: ts.SyntaxKind[kind],
      canonical_kind: canonicalKinds.get(kind) ?? 'invalid',
      start: offsets[start],
      end: offsets[end],
      text: file.text.slice(start, end),
    });
  }
  return tokens;
}

function workerSession(files) {
  const input = `${JSON.stringify({ id: 1, method: 'handshake' })}\n${JSON.stringify({
    id: 2,
    method: 'analyze',
    params: { schema_version: 4, files },
  })}\n`;
  const result = spawnSync(process.execPath, [WORKER], {
    cwd: ROOT,
    input,
    encoding: 'utf8',
    shell: false,
    windowsHide: true,
    maxBuffer: 16 * 1024 * 1024,
  });
  assert.equal(result.error, undefined, 'provider spawn');
  assert.equal(result.signal, null, 'provider signal');
  assert.equal(result.status, 0, `provider status: ${result.stderr}`);
  assert.equal(result.stderr, '', 'provider stderr');
  const lines = result.stdout.trimEnd().split('\n');
  assert.equal(lines.length, 2, 'provider response count');
  const handshake = JSON.parse(lines[0]);
  assert.equal(handshake.result.provider, 'typescript-6');
  assert.equal(handshake.result.provider_version, EXPECTED_PROVIDER_VERSION);
  assert.equal(handshake.result.protocol_version, 4);
  return JSON.parse(lines[1]);
}

function analysisIdentity(response) {
  if (response.error) {
    exactKeys(response, ['error', 'id'], 'provider diagnostic response');
    exactKeys(response.error, ['code', 'message'], 'provider diagnostic');
    const match = /file ([0-9]+) bytes ([0-9]+)\.\.([0-9]+)/.exec(response.error.message);
    assert(match, 'provider diagnostic must retain its exact source range');
    return {
      kind: 'diagnostic',
      code: response.error.code,
      file: Number.parseInt(match[1], 10),
      start: Number.parseInt(match[2], 10),
      end: Number.parseInt(match[3], 10),
    };
  }
  exactKeys(response, ['id', 'result'], 'provider snapshot response');
  return {
    kind: 'snapshot',
    file_order: response.result.files.map((file) => ({ id: file.id, path: file.path })),
    diagnostic_codes: response.result.diagnostics.map((diagnostic) => diagnostic.code),
  };
}

function main(bytes) {
  assert(bytes.length <= MAX_INPUT_BYTES, 'bounded differential request');
  const request = JSON.parse(bytes.toString('utf8'));
  exactKeys(request, ['files'], 'differential request');
  assert(Array.isArray(request.files) && request.files.length > 0, 'nonempty source files');
  const files = request.files.map((file) => {
    exactKeys(file, ['path', 'text'], 'differential source');
    assert(typeof file.path === 'string' && typeof file.text === 'string', 'source strings');
    assert(file.text.isWellFormed(), 'well-formed provider input');
    return file;
  }).toSorted((left, right) => left.path < right.path ? -1 : left.path > right.path ? 1 : 0);
  const first = workerSession(files);
  const second = workerSession(files);
  assert.deepEqual(second, first, 'provider diagnostic/snapshot replay');
  return {
    provider: { id: 'typescript-6', version: EXPECTED_PROVIDER_VERSION },
    files: files.map((file) => ({ path: file.path, text: file.text, tokens: scan(file) })),
    analysis: analysisIdentity(first),
  };
}

try {
  const chunks = [];
  for await (const chunk of process.stdin) chunks.push(chunk);
  process.stdout.write(`${JSON.stringify(main(Buffer.concat(chunks)))}\n`);
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
