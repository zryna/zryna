'use strict';

// Invoked only by the editor's isolated extension-test host, never by extension activation.
const vscode = require('vscode');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');

async function run() {
  const config = JSON.parse(await fs.readFile(process.env.ZRYNA_EDITOR_SMOKE_CONFIG, 'utf8'));
  const settings = vscode.workspace.getConfiguration('zryna');
  for (const key of ['serverPath', 'compilerRoot', 'nodePath', ...(config.compilerPath ? ['compilerPath'] : [])]) {
    await settings.update(key, config[key], vscode.ConfigurationTarget.Global);
  }
  const extension = vscode.extensions.getExtension('zryna.zryna');
  assert.ok(extension, 'installed extension is visible');
  await extension.activate();
  const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders[0].uri, 'main.zry');
  const original = 'export function add(x:i32,y:i32):i32{return x+y;}\n';
  await vscode.workspace.fs.writeFile(uri, Buffer.from(original));
  const document = await vscode.workspace.openTextDocument(uri);
  await vscode.window.showTextDocument(document);
  const options = { tabSize: 2, insertSpaces: true };
  const edits = await vscode.commands.executeCommand('vscode.executeFormatDocumentProvider', uri, options);
  assert.ok(edits.length > 0); // VS Code may minimize the server's whole-document edit.
  const change = new vscode.WorkspaceEdit();
  change.set(uri, edits);
  assert.equal(await vscode.workspace.applyEdit(change), true);
  assert.equal(document.getText(), 'export function add(x: i32, y: i32): i32 {\n  return x + y;\n}\n');
  const again = await vscode.commands.executeCommand('vscode.executeFormatDocumentProvider', uri, options);
  assert.equal(again?.length ?? 0, 0);
  const definitions = await vscode.commands.executeCommand('vscode.executeDefinitionProvider', uri, new vscode.Position(1, 9));
  assert.equal(definitions.length, 1);
  const selection = new vscode.Range(0, 0, document.lineCount - 1, 0);
  const rangeEdits = await vscode.commands.executeCommand('vscode.executeFormatRangeProvider', uri, selection, options);
  assert.equal(rangeEdits?.length ?? 0, 0);
  const runResults = config.compilerPath
    ? await require('./run-smoke.cjs').runSmoke(extension, config, document) : undefined;
  const invalid = new vscode.WorkspaceEdit();
  invalid.replace(uri, selection, 'class Unsupported {}\n');
  await vscode.workspace.applyEdit(invalid);
  const before = document.getText();
  let refused = false;
  try {
    const result = await vscode.commands.executeCommand('vscode.executeFormatDocumentProvider', uri, options);
    refused = !result || result.length === 0;
  } catch { refused = true; }
  assert.ok(refused);
  assert.equal(document.getText(), before);
  await fs.writeFile(config.resultPath, JSON.stringify({
    installed: extension.id, version: extension.packageJSON.version,
    formatting: true, idempotence: true, definition: true, range: true, malformedPreserved: true,
    editorVersion: vscode.version, platform: process.platform, runResults,
  }, null, 2));
}

module.exports = { run };
