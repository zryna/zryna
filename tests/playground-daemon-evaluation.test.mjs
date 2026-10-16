import assert from 'node:assert/strict';
import { mkdtemp, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { admitEvaluation, decodeBase64Frame, readImageFile } from
  '../examples/playground/restricted/daemon/evaluation-input.mjs';
import { sha256 } from '../examples/playground/restricted/limits.mjs';

test('evaluation file reader rejects symlinks and the first extra byte, preserving the original file', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'zryna-evaluation-input-'));
  try {
    const file = path.join(root, 'input');
    const alias = path.join(root, 'alias');
    await writeFile(file, 'four');
    assert.equal(new TextDecoder().decode(await readImageFile(file, 4)), 'four');
    await assert.rejects(readImageFile(file, 3), /IMAGE-FILE/);
    await symlink(file, alias);
    await assert.rejects(readImageFile(alias, 4));
    assert.equal(new TextDecoder().decode(await readImageFile(file, 4)), 'four');
  } finally { await rm(root, { recursive: true, force: true }); }
});

test('evaluation framing rejects noncanonical encodings and a frame beyond the compiler boundary', () => {
  assert.deepEqual(decodeBase64Frame('AAAAAA=='), new Uint8Array(4));
  for (const value of ['AAAAAB==', 'AAAAAA', 'AAAAAA==\n', '____AA==', 'AA==', null, 3]) {
    assert.throws(() => decodeBase64Frame(value), /EVALUATION-FRAME/);
  }
  assert.throws(() => decodeBase64Frame(Buffer.alloc(1_277_957).toString('base64')), /EVALUATION-FRAME/);
});

test('image-owned compiler/template pins refuse response-selected authority before executable admission', async () => {
  const template = new TextEncoder().encode('fixed reviewed template');
  const templateHash = await sha256(template);
  const identity = { version: 1, compilerSha256: 'a'.repeat(64),
    bindingTemplateSha256: templateHash, witSha256: 'b'.repeat(64), browserArchiveSha256: 'c'.repeat(64) };
  const input = { version: 1, sourceRequest: { version: 1, revision: 1, source: '' },
    frameBase64: 'AAAAAA==', selection: {}, receipt: {}, authority: {
      compilerSha256: 'd'.repeat(64), toolkitSha256: 'e'.repeat(64),
      policySha256: 'f'.repeat(64), bindingTemplateSha256: templateHash } };
  const encode = value => new TextEncoder().encode(JSON.stringify(value));
  await assert.rejects(admitEvaluation(encode(input), encode(identity), template), /IMAGE-AUTHORITY/);
  input.authority.compilerSha256 = identity.compilerSha256;
  await assert.rejects(admitEvaluation(encode(input), encode(identity), new TextEncoder().encode('substitute')),
    /IMAGE-AUTHORITY/);
  input.executable = '/untrusted';
  await assert.rejects(admitEvaluation(encode(input), encode(identity), template), /SHAPE/);
  const duplicate = encode(input).slice(1);
  await assert.rejects(admitEvaluation(new TextEncoder().encode(`{"version":1,${new TextDecoder().decode(duplicate)}`),
    encode(identity), template), /DUPLICATE/);
});
