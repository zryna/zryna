// Producers retain duplicate source facts; owning semantic rejection is tested independently.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const adapter = new URL('../', import.meta.url);
const fixture = new URL('../../../tests/m7-generic-catalogue-fixtures/duplicate-arm/', import.meta.url);
const source = await readFile(new URL('main.zry', fixture), 'utf8');
const reference = JSON.parse(await readFile(new URL('independent.json', fixture), 'utf8'));

function exchange(text) {
  const requests = [
    { id: 1, method: 'handshake' },
    { id: 2, method: 'analyze', params: {
      schema_version: 5, files: [{ path: 'main.zry', text }],
    } },
  ];
  const run = spawnSync(process.execPath, ['src/worker-v5.mjs'], {
    cwd: adapter, input: requests.map(JSON.stringify).join('\n') + '\n', encoding: 'utf8',
    timeout: 30_000, maxBuffer: 1024 * 1024, windowsHide: true,
  });
  assert.equal(run.error, undefined);
  assert.equal(run.status, 0, run.stderr);
  assert.equal(run.stderr, '');
  const responses = run.stdout.trim().split('\n').map(JSON.parse);
  assert.equal(responses.length, 2);
  assert.equal(responses[0].result.protocol_version, 5);
  assert.equal(responses[0].result.capabilities.semantic_diagnostics, false);
  assert.equal(responses[1].id, 2);
  return responses[1];
}

test('v5 duplicate match keys produce all original raw arms for the independent source verifier', () => {
  const response = exchange(source);
  assert.equal(response.error, undefined, JSON.stringify(response.error));
  assert.equal(response.result.schema_version, 5);
  assert.deepEqual(response.result, reference, 'exact complete independently authored DTO structure');
  assert.deepEqual(response.result.diagnostics, [], 'provider cannot issue owning M diagnostics');
  const arms = response.result.files[0].functions[0].body.expressions[4].kind.arms;
  assert.equal(arms.length, 3);
  assert.deepEqual(arms.map(arm => arm.variant.text), ['none', 'none', 'some']);
  assert.equal(arms[1].span.start, 81);
  assert.equal(Buffer.from(source).subarray(81, 94).toString(), '"Option.none"');
});

test('v5 duplicate keys cannot hide incomplete or malformed original source', () => {
  for (const malformed of [
    source.trimEnd().slice(0, -1),
    source.replace('"Option.none": () => 1', '"Option.none": () => '),
    source.replace('"Option.some": (item) => item', '"Option.some": (item) => '),
  ]) {
    const response = exchange(malformed);
    assert.equal(response.error?.code, 'ZRYNA-F2002');
    assert.equal(response.result, undefined, 'no partial raw DTO');
  }
});
