'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const vscode = require('vscode');
async function publish(path, value) {
  await fs.writeFile(path, `${JSON.stringify(value, null, 2)}\n`, { flag: 'wx' });
  await fs.writeFile(path + '.ready', '', { flag: 'wx' });
}

function activate(context) {
  void (async () => {
    let config;
    try {
      config = JSON.parse(await fs.readFile(process.env.ZRYNA_EDITOR_HOST_CONFIG, 'utf8'));
      assert.equal(vscode.version, '1.138.0');
      assert.equal(vscode.workspace.workspaceFolders?.length, 1);
      assert.equal(vscode.workspace.workspaceFolders[0].uri.fsPath, config.workspace);
      assert.equal(vscode.workspace.isTrusted, false);
      const extension = vscode.extensions.getExtension('zryna.zryna');
      assert(extension && extension.extensionPath === config.installedExtension);
      assert.equal(extension.isActive, false, 'Untrusted workspace must not activate Zryna.');
      const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders[0].uri, 'untrusted.zry');
      await vscode.workspace.fs.writeFile(uri,
        Buffer.from('export function main():i32{return missing;}\n'));
      const document = await vscode.workspace.openTextDocument(uri);
      await vscode.window.showTextDocument(document);
      assert.equal(document.languageId, 'zryna');
      const started = Date.now();
      let samples = 0;
      while (Date.now() - started < 1000) {
        assert.equal(vscode.workspace.isTrusted, false);
        assert.equal(extension.isActive, false, 'Actual untrusted language activation must be refused.');
        assert.equal(vscode.languages.getDiagnostics(uri).length, 0);
        samples++;
        await new Promise(resolve => setTimeout(resolve, 50));
      }
      const { verifyInstalled } = await import('./materials.mjs');
      await verifyInstalled(extension.extensionPath, config.payload, undefined, config.installer);
      await publish(config.initialPath, { initialTrusted: false,
        editorVersion: vscode.version, installedExtension: extension.extensionPath,
        untrustedLanguageAttempt: { uri: uri.toString(), samples, durationMs: Date.now() - started,
          extensionInactive: true, diagnosticsAbsent: true } });
      await new Promise((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error('Normal GUI trust action timed out.')), 120000);
        const disposable = vscode.workspace.onDidGrantWorkspaceTrust(() => {
          clearTimeout(timer); disposable.dispose(); resolve();
        });
        context.subscriptions.push(disposable);
      });
      assert.equal(vscode.workspace.isTrusted, true);
      const result = await require('./host-checks.cjs').run(config);
      assert.equal(result.checks.length, 9);
      assert.equal(result.passed, true);
      assert.equal(result.generated.length, 3);
      result.installedExtension = extension.extensionPath;
      result.installedInventory = await verifyInstalled(extension.extensionPath, config.payload, undefined, config.installer);
      await publish(config.resultPath, result);
    } catch (error) {
      if (config) await publish(config.failurePath, { failed: true, error: String(error.stack ?? error) });
      console.error(error);
    } finally {
      await vscode.workspace.saveAll();
      await vscode.commands.executeCommand('workbench.action.quit');
    }
  })();
}
module.exports = { activate };
