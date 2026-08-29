import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import path from 'node:path';
import readline from 'node:readline';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const adapterRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

async function exchange(requests) {
  const child = spawn(process.execPath, ['src/worker.mjs'], {
    cwd: adapterRoot,
    stdio: ['pipe', 'pipe', 'inherit'],
  });
  const lines = readline.createInterface({ input: child.stdout, crlfDelay: Infinity });
  const responses = [];
  lines.on('line', (line) => responses.push(JSON.parse(line)));
  child.stdin.end(`${requests.map((request) => JSON.stringify(request)).join('\n')}\n`);
  await once(child, 'exit');
  return responses;
}

test('handshake and normalized function snapshot stay provider-neutral', async () => {
  const responses = await exchange([
    { id: 1, method: 'handshake' },
    {
      id: 2,
      method: 'analyze',
      params: {
        schema_version: 1,
        files: [
          {
            path: 'src/add.zry',
            text: 'export function add(a: i32, b: i32): i32 { return a + b; }',
          },
        ],
      },
    },
  ]);

  assert.equal(responses[0].result.provider, 'typescript-6');
  assert.equal(responses[0].result.protocol_version, 1);
  assert.deepEqual(responses[1].result.files[0].functions[0].parameters, [
    ['a', { named: 'i32' }],
    ['b', { named: 'i32' }],
  ]);
  assert.deepEqual(responses[1].result.files[0].functions[0].return_type, { named: 'i32' });
  assert.equal('kind' in responses[1].result.files[0].functions[0], false);
});

test('converts TypeScript UTF-16 positions into authoritative UTF-8 byte offsets', async () => {
  const text = '// 😀 café\nexport function add(a: i32, b: i32): i32 { return a + b; }';
  const [response] = await exchange([
    {
      id: 1,
      method: 'analyze',
      params: { schema_version: 1, files: [{ path: 'src/unicode.zry', text }] },
    },
  ]);

  const span = response.result.files[0].functions[0].span;
  assert.deepEqual(span, {
    file: 0,
    start: 14,
    end: Buffer.byteLength(text, 'utf8'),
  });
  assert.equal(text.slice(0, 11), '// 😀 café\n');
});

test('assigns file identifiers by stable path order', async () => {
  const source = 'export function value(): i32 { return 1; }';
  const [response] = await exchange([
    {
      id: 1,
      method: 'analyze',
      params: {
        schema_version: 1,
        files: [
          { path: 'src/z.zry', text: source },
          { path: 'src/a.zry', text: source },
        ],
      },
    },
  ]);

  assert.deepEqual(
    response.result.files.map((file) => [file.id, file.path]),
    [
      [0, 'src/a.zry'],
      [1, 'src/z.zry'],
    ],
  );
});

test('rejects source text containing an unpaired surrogate', async () => {
  const [response] = await exchange([
    {
      id: 1,
      method: 'analyze',
      params: {
        schema_version: 1,
        files: [{ path: 'src/bad.zry', text: '\ud800' }],
      },
    },
  ]);

  assert.equal(response.error.code, 'ZRYNA-F1001');
  assert.match(response.error.message, /unpaired UTF-16 surrogate/);
});
