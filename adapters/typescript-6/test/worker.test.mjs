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
            path: 'src/add.uts',
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
