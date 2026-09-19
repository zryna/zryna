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
  const bytes = Buffer.from('export function main():i32{return 1;}');
  const document = { languageId: 'zryna', uri: { scheme: 'file', fsPath: '/project/main.zry' }, version: 1,
    isDirty: mode === 'dirty', isClosed: mode === 'closed' };
  const vscode = {
    ProgressLocation: { Notification: 1 },
    workspace: { isTrusted: mode !== 'untrusted', getWorkspaceFolder: () => ({}),
      getConfiguration: () => ({ inspect: () => ({ globalValue: '/trusted/compiler', workspaceValue: '/untrusted/compiler' }) }) },
    window: { activeTextEditor: { document }, createOutputChannel: () => ({ appendLine() {}, show() {}, clear() {} }),
      showErrorMessage: async message => errors.push(message), showInformationMessage: async () => undefined,
      withProgress: async (_, action) => action({}, { isCancellationRequested: mode === 'cancel-progress' }) },
    commands: { registerCommand: (name, fn) => { commands[name] = fn; return { dispose() {} }; } },
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
  sandbox.module.exports.registerRun(vscode, { subscriptions: [], globalStorageUri: { scheme: 'file', fsPath: '/storage' } });
  return { commands, errors, runs };
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
