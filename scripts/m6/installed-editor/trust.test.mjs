import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, writeFile, rm, realpath } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { performance } from 'node:perf_hooks';
import test from 'node:test';
import { Script, createContext } from 'node:vm';
import { waitJson, waitTrustDialog } from './trust.mjs';
import untrusted from './untrusted.cjs';
async function fixture(t) {
  const root = await mkdtemp(join(await realpath(tmpdir()), 'm6-trust-unit-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  return root;
}

test('trust dialog discovery waits for a new target and refuses duplicate exact dialogs', async () => {
  const page = matches => ({ url: () => 'vscode-file://fresh-workbench',
    getByRole: role => { assert.equal(role, 'dialog'); return {
      filter: ({ hasText }) => { assert.equal(hasText, 'exact-folder'); return { count: async () => matches }; },
    }; }, evaluate: async () => ({ title: 'fresh', bodyText: 'not ready yet', dialogs: [] }) });
  let scans = 0;
  const ready = page(1), observed = [];
  const selected = await waitTrustDialog(() => ++scans === 1 ? [] : scans === 2 ? [page(0)] : [ready],
    'exact-folder', performance.now() + 2000, () => null, row => observed.push(row));
  assert.equal(selected.page, ready); assert.equal(scans, 3);
  assert.equal(observed[1][0].matchingDialogs, 0);
  assert.equal(observed[1][0].dom.bodyText, 'not ready yet');
  for (const pages of [[page(2)], [page(1), page(1)]]) {
    await assert.rejects(waitTrustDialog(() => pages, 'exact-folder', performance.now() + 1000,
      () => null), /Ambiguous/);
  }
  await assert.rejects(waitTrustDialog(() => [page(0)], 'exact-folder', performance.now() + 1000,
    () => new Error('owned editor failed')), /owned editor failed/);
  await assert.rejects(waitTrustDialog(() => [], 'exact-folder', performance.now() - 1,
    () => null), /deadline/);
});

test('marked host failure interrupts initial/result waiting and preserves the real host error', async t => {
  const root = await fixture(t), failure = join(root, 'host-failure.json');
  const target = join(root, 'missing-result.json');
  await writeFile(failure, JSON.stringify({ failed: true, error: 'real host assertion' }));
  await writeFile(failure + '.ready', '');
  await assert.rejects(waitJson(target, performance.now() + 1000, () => null, failure),
    /real host assertion/);
  await writeFile(failure, '{invalid');
  await assert.rejects(waitJson(target, performance.now() + 1000, () => null, failure), SyntaxError);
});


test('unit host receives a grant delivered immediately when initial readiness is published', async () => {
  // Only event ordering is exercised. No editor/material/host acceptance is simulated.
  const config = { workspace: '/unit/workspace', installedExtension: '/unit/extension',
    initialPath: '/unit/initial', failurePath: '/unit/failure' };
  let grant, reached = false, finish;
  const done = new Promise(resolve => { finish = resolve; });
  const writes = new Map();
  const vscode = { version: '1.138.0',
    workspace: { workspaceFolders: [{ uri: { fsPath: config.workspace } }], isTrusted: false,
      fs: { writeFile: async () => {} }, openTextDocument: async () => ({ languageId: 'plaintext' }),
      onDidGrantWorkspaceTrust: callback => { grant = callback; return { dispose() {} }; },
      saveAll: async () => {} },
    window: { showTextDocument: async () => {} }, languages: { getDiagnostics: () => [] },
    extensions: { getExtension: () => undefined }, Uri: { joinPath: () => ({ toString: () => 'file:///unit/untrusted.zry' }) },
    commands: { executeCommand: async name => { assert.equal(name, 'workbench.action.quit'); finish(); } },
  };
  const fs = { realpath: async path => path, readFile: async () => JSON.stringify(config), writeFile: async (path, text) => {
    writes.set(path, text);
    if (path === config.initialPath + '.ready') {
      assert.equal(typeof grant, 'function', 'Listener must precede readiness publication.');
      vscode.workspace.isTrusted = true; grant();
    }
  } };
  const source = (await readFile(new URL('./extension.cjs', import.meta.url), 'utf8'))
    .replace("await import('./materials.mjs')", '({ verifyInstalled: async () => {} })');
  const module = { exports: {} };
  const context = createContext({ module, Buffer, setTimeout, clearTimeout,
    process: { env: { ZRYNA_EDITOR_HOST_CONFIG: '/unit/config' } }, console: { error() {} },
    require: name => {
      if (name === 'node:assert/strict') return assert;
      if (name === 'node:fs/promises') return fs;
      if (name === 'vscode') return vscode;
      if (name === './untrusted.cjs') return untrusted;
      if (name === './host-checks.cjs') return { run: async () => {
        reached = true; throw new Error('UNIT_GRANTED_EVENT_DELIVERED');
      } };
      throw new Error('Unexpected unit import: ' + name);
    } });
  new Script(source).runInContext(context);
  module.exports.activate({ subscriptions: [] });
  let timer;
  try { await Promise.race([done, new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error('Unit grant flow did not finish')), 5000);
  })]); } finally { clearTimeout(timer); }
  assert.equal(reached, true);
  assert.match(JSON.parse(writes.get(config.failurePath)).error, /UNIT_GRANTED_EVENT_DELIVERED/);
  assert.equal(writes.get(config.failurePath + '.ready'), '');
});


test('workspace binding compares actual canonical paths and rejects a different physical folder', async t => {
  const root = await fixture(t), selected = join(root, 'selected'), other = join(root, 'other');
  await mkdir(selected); await mkdir(other);
  const module = { exports: {} };
  const context = createContext({ module, require: name => {
    if (name === 'node:assert/strict') return assert;
    if (name === 'node:fs/promises') return { realpath };
    if (name === 'vscode') return {};
    if (name === './untrusted.cjs') return untrusted;
    throw new Error('Unexpected unit import: ' + name);
  } });
  new Script(await readFile(new URL('./extension.cjs', import.meta.url), 'utf8')).runInContext(context);
  const report = await module.exports.bindPhysicalDirectory(selected + '/../selected', selected);
  assert.equal(report.reportedFsPath, selected + '/../selected');
  assert.equal(report.reportedRealPath, await realpath(selected));
  assert.equal(report.selectedRealPath, report.reportedRealPath);
  await assert.rejects(module.exports.bindPhysicalDirectory(other, selected), /selected physical directory/);
});
