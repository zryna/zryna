// Public generated-loader execution for one genuine source-emitted component byte boundary.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { verifyBrowser } from '../scalar-host/browser-fixture.mjs';

const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const exactSha = '128b13f47a9195a2e927bf6cdea34588a7194656ad8b265b8e346c8c43d93c97';
const extraSha = 'c8345ef73d90facd7f49cbe520588e146c830cd034430504e448a1c29dd90cd6';
const [loaderPath, componentPath, ordinaryLoaderPath, ordinaryComponentPath, browserRoot] = process.argv.slice(2);
assert([loaderPath, componentPath, ordinaryLoaderPath, ordinaryComponentPath].every(Boolean));
const loader = await readFile(loaderPath, 'utf8');
const ordinaryLoader = await readFile(ordinaryLoaderPath, 'utf8');
const bytes = new Uint8Array(await readFile(componentPath));
const ordinaryBytes = new Uint8Array(await readFile(ordinaryComponentPath));
assert.equal(bytes.length, 1048576);
assert.equal(sha256(bytes), exactSha);

function leb(value) {
  const bytes = [];
  do { bytes.push((value & 127) | (value >= 128 ? 128 : 0)); value = Math.floor(value / 128); } while (value);
  return bytes;
}
function readLeb(bytes, cursor) {
  let value = 0;
  let scale = 1;
  for (let count = 0; count < 5; count++) {
    const byte = bytes[cursor.offset++];
    assert.notEqual(byte, undefined);
    value += (byte & 127) * scale;
    if (byte < 128) return value;
    scale *= 128;
  }
  throw new Error('fixture LEB exceeds u32');
}
function sections(bytes) {
  const cursor = { offset: 8 };
  const result = [];
  while (cursor.offset < bytes.length) {
    const id = bytes[cursor.offset++];
    const size = readLeb(bytes, cursor);
    const start = cursor.offset;
    assert(start + size <= bytes.length);
    result.push({ id, data: bytes.slice(start, start + size) });
    cursor.offset += size;
  }
  assert.equal(cursor.offset, bytes.length);
  return result;
}
function rebuild(bytes, entries) {
  const size = 8 + entries.reduce((n, entry) => n + 1 + leb(entry.data.length).length + entry.data.length, 0);
  const result = new Uint8Array(size);
  result.set(bytes.subarray(0, 8));
  let offset = 8;
  for (const entry of entries) {
    result[offset++] = entry.id;
    const length = leb(entry.data.length);
    result.set(length, offset); offset += length.length;
    result.set(entry.data, offset); offset += entry.data.length;
  }
  return result;
}
function firstExtra(exact) {
  // Replace precisely function 106's canonical `i32.const 0` with `i32.const 64`.
  // This is the next authored source literal, with body/code/core/component lengths
  // reencoded. The independent source-authenticated Rust factory pins every byte.
  const outer = sections(exact);
  const modules = outer.filter(entry => entry.id === 1);
  assert.equal(modules.length, 1);
  const core = modules[0].data;
  const inner = sections(core);
  const codes = inner.filter(entry => entry.id === 10);
  assert.equal(codes.length, 1);
  const code = codes[0].data;
  const cursor = { offset: 0 };
  assert.equal(readLeb(code, cursor), 1863);
  let changed;
  for (let index = 0; index <= 106; index++) {
    const start = cursor.offset;
    const size = readLeb(code, cursor);
    const end = cursor.offset + size;
    if (index === 106) {
      assert.deepEqual(Array.from(code.subarray(cursor.offset, end)), [0, 65, 0, 11]);
      const replacement = new Uint8Array([...leb(5), 0, 65, 192, 0, 11]);
      changed = new Uint8Array(code.length + 1);
      changed.set(code.subarray(0, start));
      changed.set(replacement, start);
      changed.set(code.subarray(end), start + replacement.length);
    }
    cursor.offset = end;
  }
  codes[0].data = changed;
  modules[0].data = rebuild(core, inner);
  assert(WebAssembly.validate(modules[0].data), 'next source has a valid ordinary scalar core');
  const extra = rebuild(exact, outer);
  assert.equal(extra.length, 1048577);
  assert.equal(sha256(extra), extraSha, 'independent canonical source factory byte identity');
  return extra;
}
const extra = firstExtra(bytes);

async function corpus(instantiate, exact, extra, ordinaryInstantiate, ordinary) {
  const instance = await instantiate(exact);
  if (!Object.isFrozen(instance) || Object.keys(instance).length !== 1863) throw new Error('fixture interface differs');
  for (let index = 0; index < 1863; index++) {
    const name = `f${index.toString(16).padStart(4, '0')}${'x'.repeat(123)}`;
    if (instance[name]() !== (index < 106 ? 64 : 0)) throw new Error('source return differs');
  }
  let rejection;
  try { await instantiate(extra); } catch (error) { rejection = error.message; }
  if (rejection !== 'ZRYNA-B3991') throw new Error('first extra was not rejected at byte admission');
  const recovered = await ordinaryInstantiate(ordinary);
  const recovery = recovered.add(20, 22);
  if (recovery !== 42) throw new Error('ordinary loader did not recover');
  return { executedExports: 1863, wideReturns: 106, narrowReturns: 1757,
    firstExtraRejection: rejection, ordinaryRecovery: recovery };
}
const module = await import(pathToFileURL(loaderPath));
const ordinaryModule = await import(pathToFileURL(ordinaryLoaderPath));
assert.equal(module.browserComponentIdentity.componentSha256, exactSha);
assert.equal(module.browserComponentIdentity.world, 'zryna:capability-profiles/browser@0.1.0');
const node = await corpus(module.instantiateBrowserComponent, bytes, extra,
  ordinaryModule.instantiateBrowserComponent, ordinaryBytes);
let browserResult = null;
if (browserRoot) {
  const require = createRequire(import.meta.url);
  const pin = JSON.parse(await readFile(new URL('../scalar-host/browser-pin.json', import.meta.url)));
  const executablePath = await verifyBrowser(browserRoot);
  assert.equal(require('playwright-core/package.json').version, pin.runnerVersion);
  const { chromium } = require('playwright-core');
  let browser;
  let context;
  let page;
  let unexpected = 0;
  try {
    browser = await chromium.launch({ executablePath, headless: true, chromiumSandbox: true,
      timeout: 10000, args: ['--disable-background-networking'] });
    assert.equal(browser.version(), pin.browserVersion);
    context = await browser.newContext({ serviceWorkers: 'block' });
    await context.route('**/*', route => {
      if (route.request().isNavigationRequest() && route.request().url() === 'https://browser.invalid/') {
        return route.fulfill({ contentType: 'text/html', body: '<!doctype html><title>Canonical byte boundary</title>' });
      }
      unexpected++;
      return route.abort();
    });
    page = await context.newPage();
    page.on('worker', () => { unexpected++; });
    await page.goto('https://browser.invalid/');
    browserResult = await page.evaluate(async data => {
      const urls = [data.loader, data.ordinaryLoader, `export ${data.corpus}`]
        .map(source => URL.createObjectURL(new Blob([source], { type: 'text/javascript' })));
      try {
        const [loaded, ordinary, run] = await Promise.all(urls.map(url => import(url)));
        if (loaded.browserComponentIdentity.componentSha256 !== data.exactSha ||
            loaded.browserComponentIdentity.world !== 'zryna:capability-profiles/browser@0.1.0') {
          throw new Error('browser sealed identity differs');
        }
        return await run.corpus(loaded.instantiateBrowserComponent, new Uint8Array(data.exact),
          new Uint8Array(data.extra), ordinary.instantiateBrowserComponent, new Uint8Array(data.ordinary));
      } finally { for (const url of urls) URL.revokeObjectURL(url); }
    }, { loader, ordinaryLoader, corpus: corpus.toString(), exactSha,
      exact: Array.from(bytes), extra: Array.from(extra), ordinary: Array.from(ordinaryBytes) });
    assert.deepEqual(browserResult, node);
    assert.equal(unexpected, 0);
  } finally {
    const failures = [];
    for (const resource of [page, context, browser]) if (resource) {
      try { await resource.close(); } catch (error) { failures.push(error); }
    }
    if (failures.length) throw new Error('browser byte-boundary cleanup could not be confirmed');
  }
  await verifyBrowser(browserRoot);
}
console.log(JSON.stringify({ componentBytes: bytes.length, componentSha256: sha256(bytes),
  firstExtraComponentBytes: extra.length, firstExtraComponentSha256: sha256(extra),
  node, browser: browserResult }));
