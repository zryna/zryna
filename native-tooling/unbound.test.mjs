// SOURCE ONLY, UNRUN. Actual collector graph, native and filesystem effects denied.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { constants } from 'node:fs';
import { createHash } from 'node:crypto';
import * as path from 'node:path';
import { performance } from 'node:perf_hooks';
import { pathToFileURL, fileURLToPath } from 'node:url';
import { SourceTextModule, SyntheticModule, createContext } from 'node:vm';
import { test } from 'node:test';

test('actual collector refuses unselected source before filesystem/native effects', async () => {
  let effects = 0;
  const denied = () => { effects++; throw new Error('Forbidden unbound effect.'); };
  const context = createContext({ Buffer, TextDecoder, structuredClone,
    process: { argv: ['node', 'collect.mjs'], env: {} } });
  const builtins = new Map([
    ['node:assert/strict', { default: assert }], ['node:fs', { constants }],
    ['node:fs/promises', { access: denied, mkdir: denied, open: denied, realpath: denied,
      readdir: denied, lstat: denied, writeFile: denied }],
    ['node:child_process', { spawn: denied }], ['node:crypto', { createHash }],
    ['node:path', path], ['node:perf_hooks', { performance }],
    ['node:url', { pathToFileURL, fileURLToPath }],
  ]);
  const cache = new Map();
  async function load(name) {
    if (cache.has(name)) return cache.get(name);
    let module;
    if (builtins.has(name)) {
      const values = builtins.get(name), keys = Object.keys(values);
      module = new SyntheticModule(keys, function () {
        for (const key of keys) this.setExport(key, values[key]);
      }, { context });
    } else {
      assert(['collect.mjs', 'provider-pins.mjs', 'producer-pins.mjs', 'owner/process.mjs',
        'owner/owner-client.mjs', 'owner/files.mjs', 'binding-producer/recorder/capture.mjs',
        'binding-producer/recorder/catalogue.mjs', 'binding-producer/recorder/select-cargo.mjs'].includes(name));
      module = new SourceTextModule(await readFile(new URL(name, import.meta.url), 'utf8'),
        { context, identifier: name, importModuleDynamically: async () => { denied(); } });
    }
    cache.set(name, module); return module;
  }
  const module = await load('collect.mjs');
  await module.link((specifier, referrer) => load(specifier.startsWith('.')
    ? path.posix.normalize(path.posix.join(path.posix.dirname(referrer.identifier), specifier)) : specifier));
  await assert.rejects(module.evaluate(), /UNBOUND/); assert.equal(effects, 0);
});
