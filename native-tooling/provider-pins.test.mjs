// SOURCE ONLY, UNRUN. Inert retained owning-source fixture, no Rust/native execution.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import { providerPins } from './provider-pins.mjs';

test('actual owning capture declarations select four exact provider byte roles; malformed declarations refuse', async () => {
  const source = await readFile(new URL('./provider-capture.fixture.rs', import.meta.url), 'utf8');
  const selected = providerPins(source);
  assert.equal(selected.length, 4);
  assert.equal(selected[0].components.join('/'), 'node_modules/.pnpm/@typescript+typescript6@6.0.2/node_modules/@typescript/typescript6/package.json');
  assert.equal(selected[3].components.join('/'), 'node_modules/.pnpm/typescript@6.0.3/node_modules/typescript/lib/typescript.js');
  assert.equal(selected[3].sha256, '569177652966bd528c319171c7dd22860dbf72bde116cbc4f644f1d02bb12e39');
  for (const changed of [source.replace('"node_modules",', '"..",'),
    source.replace('"node_modules",', '"bad/segment",'),
    source.replace('"node_modules",', 'call(),'),
    source.replace('const WRAPPER_MANIFEST: &[&str]', 'const OTHER: &[&str]'),
    source.replace('const WRAPPER_SHA256:', 'const OTHER_SHA256:'),
    source + '\nconst WRAPPER: &[&str] = &["duplicate"];']) {
    assert.throws(() => providerPins(changed));
  }
});
