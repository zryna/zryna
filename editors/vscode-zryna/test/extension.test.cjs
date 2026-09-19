'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { resolve } = require('node:path');
const vm = require('node:vm');

function fixture({ trusted = true, capability = 'scalar-format-v1', editResult = [] } = {}) {
  const launched = [];
  const sent = [];
  const providers = {};
  const document = {
    uri: { scheme: 'file', toString: () => 'file:///project/main.zry' }, languageId: 'zryna', version: 1,
    getText: () => 'export function f():i32{return 1;}', lineCount: 1,
    lineAt: () => ({ text: 'export function f():i32{return 1;}' }),
  };
  const disposable = () => ({ dispose() {} });
  const vscode = {
    workspace: {
      isTrusted: trusted,
      getWorkspaceFolder: () => ({ uri: { toString: () => 'file:///project' } }),
      getConfiguration: () => ({ inspect: key => ({ globalValue: resolve(`trusted-${key}`), workspaceValue: 'hostile-workspace-command' }) }),
      onDidChangeTextDocument: disposable, onDidCloseTextDocument: disposable, onDidChangeConfiguration: disposable,
    },
    window: { onDidChangeActiveTextEditor: disposable },
    languages: {
      createDiagnosticCollection: () => ({ ...disposable(), delete() {}, set() {} }),
      registerDocumentFormattingEditProvider: (_, provider) => { providers.format = provider; return disposable(); },
      registerDocumentRangeFormattingEditProvider: (_, provider) => { providers.range = provider; return disposable(); },
      registerDefinitionProvider: (_, provider) => { providers.definition = provider; return disposable(); },
    },
  };
  class Connection {
    constructor(config) { launched.push(config); }
    async request(method) {
      if (method === 'initialize') return {
        serverInfo: { name: 'zryna-language-server', version: '0.2.3' },
        capabilities: { positionEncoding: 'utf-16', documentFormattingProvider: true,
          documentRangeFormattingProvider: true, experimental: { zrynaFormattingProfile: capability } },
      };
      return typeof editResult === 'function' ? editResult(document) : editResult;
    }
    notify(method, params) { sent.push({ method, params }); }
    fail() { this.closed = true; }
    async stop() { this.closed = true; }
  }
  const sandbox = { module: { exports: {} }, require: name => name === 'vscode' ? vscode
    : name === './run-command.cjs' ? { registerRun() {} } : { Connection } };
  vm.runInNewContext(readFileSync(resolve(__dirname, '../src/extension.cjs'), 'utf8'), sandbox);
  sandbox.module.exports.activate({ subscriptions: [] });
  return { document, launched, sent, providers, deactivate: sandbox.module.exports.deactivate };
}

test('untrusted, virtual and closed documents cannot launch a compiler', async () => {
  for (const kind of ['untrusted', 'virtual', 'closed']) {
    const f = fixture({ trusted: kind !== 'untrusted' });
    if (kind === 'virtual') f.document.uri.scheme = 'https';
    if (kind === 'closed') f.document.isClosed = true;
    await assert.rejects(f.providers.format.provideDocumentFormattingEdits(f.document, {}), /trusted local/);
    assert.equal(f.launched.length, 0);
  }
});

test('workspace executable overrides are ignored and incompatible servers receive no document contents', async () => {
  const f = fixture({ capability: undefined });
  // Use an explicit incompatible value rather than the fixture's default capability.
  const old = fixture({ capability: 'old-scalar-server' });
  await assert.rejects(old.providers.format.provideDocumentFormattingEdits(old.document, {}), /released v0.2.3/);
  assert.equal(old.sent.length, 0);
  assert.equal(old.launched[0].serverPath, resolve('trusted-serverPath'));
  await f.deactivate();
});

test('matching server gets only the explicit document and stale versions cannot return edits', async () => {
  const f = fixture({ editResult: document => { document.version++; return []; } });
  await assert.rejects(f.providers.format.provideDocumentFormattingEdits(f.document, {}), /document changed/);
  assert.deepEqual(f.sent.map(item => item.method), ['initialized', 'textDocument/didOpen']);
  assert.equal(f.sent[1].params.textDocument.text, f.document.getText());
  await f.deactivate();
});

test('foreign definition locations cannot redirect the editor', async () => {
  const f = fixture({ editResult: { uri: 'file:///other/private.zry', range: {} } });
  await assert.rejects(f.providers.definition.provideDefinition(f.document, { line: 0, character: 0 }), /Foreign/);
  await f.deactivate();
});
