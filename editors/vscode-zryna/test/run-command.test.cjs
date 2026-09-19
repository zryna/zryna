'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');

function fixture(mode) {
  const commands = {};
  const errors = [];
  const runs = [];
  let storageReads = 0;
  const disposable = () => ({ dispose() {} });
  const bytes = Buffer.from('export function main():i32{return 1;}');
  const document = { languageId: 'zryna', uri: { scheme: 'file', fsPath: '/project/main.zry' }, version: 1,
    isDirty: mode === 'dirty', isClosed: mode === 'closed' };
  const vscode = {
    ProgressLocation: { Notification: 1 },
    workspace: { isTrusted: mode !== 'untrusted', getWorkspaceFolder: () => ({}),
      onDidChangeTextDocument: disposable, onDidCloseTextDocument: disposable, onDidChangeConfiguration: disposable,
      getConfiguration: () => ({ inspect: () => ({ globalValue: '/trusted/compiler', workspaceValue: '/untrusted/compiler' }) }) },
    window: { onDidChangeActiveTextEditor: disposable,
      createOutputChannel: () => ({ appendLine() {}, show() {}, clear() {} }),
      showErrorMessage: async message => errors.push(message), showInformationMessage: async () => undefined,
      withProgress: async (_, action) => action({}, { isCancellationRequested: mode === 'cancel-progress' }) },
    commands: { registerCommand: (name, fn) => { commands[name] = fn; return { dispose() {} }; } },
    languages: { createDiagnosticCollection: disposable, registerDocumentFormattingEditProvider: disposable,
      registerDocumentRangeFormattingEditProvider: disposable, registerDefinitionProvider: disposable },
  };
  const dependencies = {
    'node:fs/promises': {}, 'node:path': path,
    './run-process.cjs': { stopRuns() {} },
    './run-input.cjs': { sourceText: bytes => bytes.toString(), selectRun: async () => {
      if (mode === 'cancel') return;
      if (mode === 'stale') document.version++;
      return { name: 'main', args: [], target: 'javascript' };
    } },
    './run-project.cjs': { regularFile: async () => bytes, runProject: async input => {
      if (input.token.isCancellationRequested) throw new Error('Run cancelled.');
      runs.push(input); return { target: 'javascript', folder: '/storage/output', file: '/storage/output/main.mjs' };
    } },
  };
  const sandbox = { module: { exports: {} }, require: name => dependencies[name] };
  vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../src/run-command.cjs'), 'utf8'), sandbox);
  const extension = { module: { exports: {} }, require: name => name === 'vscode' ? vscode
    : name === './run-command.cjs' ? sandbox.module.exports : {} };
  vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../src/extension.cjs'), 'utf8'), extension);
  extension.module.exports.activate(Object.freeze({ subscriptions: [],
    extensionUri: { scheme: mode === 'remote-extension' ? 'vscode-remote' : 'file' },
    get globalStorageUri() {
      storageReads++;
      if (mode === 'missing-storage') return undefined;
      return { scheme: ['userdata', 'remote-extension', 'authority'].includes(mode) ? 'vscode-userdata'
        : mode === 'foreign-storage' ? 'https' : 'file',
      authority: mode === 'authority' ? 'remote' : '', fsPath: mode === 'relative-storage' ? 'storage' : '/storage' };
    },
  }));
  vscode.window.activeTextEditor = { document };
  return { commands, errors, runs, get storageReads() { return storageReads; } };
}

test('registration never runs; trust, unsaved, stale and cancelled input never executes', async () => {
  for (const mode of ['untrusted', 'dirty', 'closed', 'stale', 'cancel', 'cancel-progress']) {
    const f = fixture(mode);
    assert.equal(f.runs.length, 0);
    await f.commands['zryna.run']();
    assert.equal(f.runs.length, 0, mode);
    if (mode !== 'cancel') assert.equal(f.errors.length, 1, mode);
  }
});

test('explicit command uses only user executable and exact saved bytes', async () => {
  const f = fixture('ok');
  await f.commands['zryna.run']();
  assert.equal(f.errors.length, 0);
  assert.equal(f.runs.length, 1);
  assert.equal(f.runs[0].compiler, '/trusted/compiler');
  assert.equal(f.runs[0].bytes.toString(), 'export function main():i32{return 1;}');
});

test('activation forwards desktop user-data storage from the host context', async () => {
  const f = fixture('userdata');
  assert.equal(f.storageReads, 0);
  assert.equal(f.runs.length, 0);
  await f.commands['zryna.run']();
  assert.deepEqual(f.errors, []);
  assert.equal(f.storageReads, 1);
  assert.equal(f.runs[0].storage, path.join('/storage', 'runs'));
});

test('nonlocal or unavailable host storage never executes', async () => {
  for (const mode of ['remote-extension', 'authority', 'foreign-storage', 'relative-storage', 'missing-storage']) {
    const f = fixture(mode);
    await f.commands['zryna.run']();
    assert.deepEqual(f.errors, ['Local extension storage is unavailable.'], mode);
    assert.equal(f.runs.length, 0, mode);
  }
});
