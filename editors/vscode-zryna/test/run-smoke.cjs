'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');
const vscode = require('vscode');

// Isolated host test: real editor APIs/compiler, deterministic responses to the interactive prompts.
async function runSmoke(extension, config, document) {
  const { registerRun } = require(path.join(extension.extensionPath, 'src/run-command.cjs'));
  const callbacks = {};
  const errors = [];
  const lines = [];
  const opened = [];
  const revealed = [];
  let target = 'javascript';
  let argument = 0;
  const api = {
    Uri: vscode.Uri, ProgressLocation: vscode.ProgressLocation, workspace: vscode.workspace,
    window: {
      get activeTextEditor() { return vscode.window.activeTextEditor; },
      createOutputChannel: () => ({ appendLine: text => lines.push(text), clear() {}, show() {}, dispose() {} }),
      showQuickPick: async (items, options) => options.title.includes('profile') ? 'i32-v1'
        : typeof items[0] === 'string' ? target : items.find(item => item.label === 'add'),
      showInputBox: async () => ['13', '-4'][argument++],
      showErrorMessage: async text => errors.push(text), showInformationMessage: async () => undefined,
      withProgress: (options, task) => vscode.window.withProgress(options, task),
      showTextDocument: async doc => { opened.push(doc.uri.fsPath); return vscode.window.showTextDocument(doc); },
    },
    commands: {
      registerCommand: (id, fn) => { callbacks[id] = fn; return { dispose() {} }; },
      executeCommand: async (id, value) => {
        if (id === 'zryna.selectEditorProfile') { assert.equal(value, 'i32-v1'); return; }
        assert.equal(id, 'revealFileInOS'); revealed.push(value.fsPath);
      },
    },
  };
  const context = { subscriptions: [], globalStorageUri: vscode.Uri.file(config.runStorage) };
  registerRun(api, context);
  await document.save();
  for (const selected of ['javascript', 'webassembly']) {
    target = selected;
    argument = 0;
    await vscode.window.showTextDocument(document);
    await callbacks['zryna.run']();
    assert.deepEqual(errors, []);
    assert.ok(lines.some(line => line.includes(`${selected}: i32 9`)));
    if (selected === 'javascript') {
      await callbacks['zryna.openGeneratedJavaScript']();
      assert.equal(opened.length, 1);
      assert.ok(opened[0].endsWith('result.mjs'));
      assert.ok(vscode.window.activeTextEditor.document.getText().includes('export'));
    }
    await callbacks['zryna.revealRunOutput']();
    assert.ok(revealed.at(-1).endsWith(selected === 'javascript' ? '.mjs' : '.wasm'));
  }
  await vscode.window.showTextDocument(document);
  context.subscriptions.forEach(item => item.dispose());
  return { javascriptResult: 9, webassemblyResult: 9, opened, revealed };
}

module.exports = { runSmoke };
