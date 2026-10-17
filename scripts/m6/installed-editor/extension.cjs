'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const vscode = require('vscode');
const { observeUntrustedExtension, observeUntrustedDocument } = require('./untrusted.cjs');
async function publish(path, value) {
  await fs.writeFile(path, `${JSON.stringify(value, null, 2)}\n`, { flag: 'wx' });
  await fs.writeFile(path + '.ready', '', { flag: 'wx' });
}

async function bindPhysicalDirectory(reportedFsPath, selectedPath) {
  const [reportedRealPath, selectedRealPath] = await Promise.all([
    fs.realpath(reportedFsPath), fs.realpath(selectedPath),
  ]);
  const [reported, selected] = await Promise.all([
    fs.stat(reportedRealPath, { bigint: true }), fs.stat(selectedRealPath, { bigint: true }),
  ]);
  assert(reported.isDirectory() && selected.isDirectory(), 'Selected physical directory required.');
  assert.notEqual(reported.ino, 0n);
  assert.equal(reported.dev, selected.dev, 'Editor must use the selected physical directory.');
  assert.equal(reported.ino, selected.ino, 'Editor must use the selected physical directory.');
  return { reportedFsPath, reportedRealPath, selectedRealPath,
    physicalIdentity: { device: String(reported.dev), inode: String(reported.ino) } };
}

function activate(context) {
  void (async () => {
    let config, trustTimer, trustDisposable;
    let lastUntrustedRegistry = null;
    try {
      config = JSON.parse(await fs.readFile(process.env.ZRYNA_EDITOR_HOST_CONFIG, 'utf8'));
      assert.equal(vscode.version, '1.138.0');
      assert.equal(vscode.workspace.workspaceFolders?.length, 1);
      const workspaceBinding = await bindPhysicalDirectory(
        vscode.workspace.workspaceFolders[0].uri.fsPath, config.workspace);
      assert.equal(vscode.workspace.isTrusted, false);
      const sampleExtension = async () => {
        const extension = vscode.extensions.getExtension('zryna.zryna');
        const reportedPath = extension === undefined ? config.installedExtension : extension?.extensionPath;
        lastUntrustedRegistry = observeUntrustedExtension(extension, reportedPath);
        const binding = extension === undefined ? null
          : await bindPhysicalDirectory(reportedPath, config.installedExtension);
        return { extension, reportedPath, binding };
      };
      const initialExtension = await sampleExtension();
      const initialRegistry = observeUntrustedExtension(initialExtension.extension, initialExtension.reportedPath);
      const uri = vscode.Uri.joinPath(vscode.workspace.workspaceFolders[0].uri, 'untrusted.zry');
      await vscode.workspace.fs.writeFile(uri,
        Buffer.from('export function main():i32{return missing;}\n'));
      const document = await vscode.workspace.openTextDocument(uri);
      await vscode.window.showTextDocument(document);
      const observeDocument = async () => {
        const sample = await sampleExtension();
        const observed = observeUntrustedDocument(sample.extension, sample.reportedPath, document.languageId);
        lastUntrustedRegistry = observed.registry;
        return { ...observed, physicalBinding: sample.binding };
      };
      const initialDocument = await observeDocument();
      const started = Date.now();
      let samples = 0;
      const registryObservations = [];
      const documentObservations = [];
      while (Date.now() - started < 1000) {
        assert.equal(vscode.workspace.isTrusted, false);
        assert(registryObservations.length < 64, 'Untrusted registry sample limit exceeded.');
        const observed = await observeDocument();
        registryObservations.push(observed.registry);
        documentObservations.push(observed);
        assert.equal(vscode.languages.getDiagnostics(uri).length, 0);
        samples++;
        await new Promise(resolve => setTimeout(resolve, 50));
      }
      const { verifyInstalled } = await import('./materials.mjs');
      await verifyInstalled(config.installedExtension, config.payload, undefined, config.installer);
      const granted = new Promise((resolve, reject) => {
        trustTimer = setTimeout(() => reject(new Error('Normal GUI trust action timed out.')), 120000);
        trustDisposable = vscode.workspace.onDidGrantWorkspaceTrust(() => {
          clearTimeout(trustTimer); trustDisposable.dispose(); resolve();
        });
        context.subscriptions.push(trustDisposable);
      });
      await publish(config.initialPath, { initialTrusted: false,
        editorVersion: vscode.version, workspaceBinding, installedExtension: config.installedExtension, initialRegistry,
        initialRegistryBinding: initialExtension.binding,
        untrustedLanguageAttempt: { kind: 'opened-zry-file', forcedLanguageSelection: false,
          uri: uri.toString(), initialDocument, samples, durationMs: Date.now() - started,
          registryObservations, documentObservations, diagnosticsAbsent: true } });
      await granted;
      assert.equal(vscode.workspace.isTrusted, true);
      const result = await require('./host-checks.cjs').run(config);
      assert.equal(result.checks.length, 9);
      assert.equal(result.passed, true);
      assert.equal(result.generated.length, 3);
      const extension = vscode.extensions.getExtension('zryna.zryna');
      assert(extension);
      const extensionBinding = await bindPhysicalDirectory(extension.extensionPath, config.installedExtension);
      assert.equal(extension.isActive, true);
      result.installedExtension = extensionBinding.selectedRealPath;
      result.installedExtensionRawPath = extension.extensionPath;
      result.installedExtensionPhysicalBinding = extensionBinding;
      result.installedInventory = await verifyInstalled(extensionBinding.selectedRealPath, config.payload, undefined, config.installer);
      await publish(config.resultPath, result);
    } catch (error) {
      if (config) await publish(config.failurePath, { failed: true, error: String(error.stack ?? error),
        untrustedRegistry: error.untrustedRegistryObservation ?? lastUntrustedRegistry,
        untrustedDocument: error.untrustedLanguageObservation ?? null });
      console.error(error);
    } finally {
      clearTimeout(trustTimer);
      trustDisposable?.dispose();
      await vscode.workspace.saveAll();
      await vscode.commands.executeCommand('workbench.action.quit');
    }
  })();
}
module.exports = { activate, bindPhysicalDirectory };
