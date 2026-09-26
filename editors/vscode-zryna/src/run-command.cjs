'use strict';

const fs = require('node:fs/promises');
const { isAbsolute, join } = require('node:path');
const { selectRun, SOURCE_LIMITS } = require('./run-input.cjs');
const { runProject, regularFile, verifyOutput } = require('./run-project.cjs');
const { stopRuns } = require('./run-process.cjs');
const { configuredInstallation } = require('./installation.cjs');

function registerRun(vscode, context) {
  const output = vscode.window.createOutputChannel('Zryna Run');
  let busy = false;
  let disposed = false;
  let last;
  function checkDocument(document) {
    if (!vscode.workspace.isTrusted || !document || document.isClosed || document.uri.scheme !== 'file'
      || document.languageId !== 'zryna' || !vscode.workspace.getWorkspaceFolder(document.uri)) {
      throw new Error('Run requires an open Zryna file in a trusted local workspace.');
    }
    if (document.isDirty || document.isUntitled) throw new Error('Save the source file before Run. Unsaved edits are never executed.');
  }
  async function inspect(kind, result = last) {
    if (!vscode.workspace.isTrusted) throw new Error('A trusted workspace is required.');
    if (!result) throw new Error('Complete a Zryna Run first.');
    if (kind === 'javascript' && result.target !== 'javascript') throw new Error('The last run emitted WebAssembly. Run with the JavaScript target to open generated source.');
    await verifyOutput(result.project, result.selection, result.hash);
    const stat = await fs.lstat(result.file);
    if (!stat.isFile() || stat.isSymbolicLink()) throw new Error('The last output is unavailable. Run again.');
    if (kind === 'javascript') await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(vscode.Uri.file(result.file)));
    else {
      await vscode.commands.executeCommand('workbench.action.focusActiveEditorGroup');
      await vscode.commands.executeCommand('revealFileInOS', vscode.Uri.file(result.file));
    }
  }
  const handle = action => async () => {
    try { await action(); } catch (error) {
      output.appendLine(error.message);
      output.show(true);
      await vscode.window.showErrorMessage(error.message.slice(0, 500));
    }
  };
  context.subscriptions.push(output, { dispose() { disposed = true; stopRuns(); } },
    vscode.commands.registerCommand('zryna.run', handle(async () => {
      if (busy) throw new Error('A Zryna Run is already in progress. Cancel it or wait for completion.');
      busy = true;
      try {
        const document = vscode.window.activeTextEditor?.document;
        checkDocument(document);
        const version = document.version;
        const bytes = await regularFile(document.uri.fsPath, SOURCE_LIMITS['control-flow-v1']);
        const selection = await selectRun(vscode.window, bytes);
        if (!selection) return;
        checkDocument(document);
        if (document.version !== version || !(await regularFile(document.uri.fsPath, SOURCE_LIMITS['control-flow-v1'])).equals(bytes)) {
          throw new Error('Source changed during selection. Run again to use the new saved version.');
        }
        const compiler = configuredInstallation(vscode)?.compilerPath
          ?? vscode.workspace.getConfiguration('zryna').inspect('compilerPath')?.globalValue;
        const storage = context.globalStorageUri;
        const localUserData = storage?.scheme === 'vscode-userdata'
          && context.extensionUri?.scheme === 'file' && !context.extensionUri.authority;
        if (!storage || (storage.scheme !== 'file' && !localUserData) || storage.authority
          || typeof storage.fsPath !== 'string' || !isAbsolute(storage.fsPath)) {
          throw new Error('Local extension storage is unavailable.');
        }
        output.clear();
        output.show(true);
        output.appendLine(`Saved file: ${document.uri.fsPath}\n${selection.profile}: ${selection.name}(${selection.args.map(argument => `${argument.type}:${argument.value}`).join(', ')}) — ${selection.target}`);
        last = undefined;
        await vscode.window.withProgress({ location: vscode.ProgressLocation.Notification,
          title: 'Zryna: Running saved source', cancellable: true }, async (_, token) => {
          checkDocument(document);
          const lifetime = { get isCancellationRequested() { return disposed || token.isCancellationRequested; },
            onCancellationRequested: callback => token.onCancellationRequested(callback) };
          if (lifetime.isCancellationRequested) throw new Error('Run cancelled.');
          await vscode.commands.executeCommand('zryna.selectEditorProfile', selection.profile);
          checkDocument(document);
          if (lifetime.isCancellationRequested) throw new Error('Run cancelled.');
          if (document.version !== version
            || !(await regularFile(document.uri.fsPath, SOURCE_LIMITS['control-flow-v1'])).equals(bytes)) {
            throw new Error('Source changed during selection. Run again to use the new saved version.');
          }
          last = await runProject({ compiler, storage: join(storage.fsPath, 'runs'), bytes,
            selection, token: lifetime, report: text => output.appendLine(text) });
        });
        output.appendLine(`Output folder: ${last.folder}\nArtifact: ${last.file}`);
        if (document.version !== version || document.isDirty) output.appendLine('Editor changed during Run; results refer to the saved snapshot above.');
        const completed = last;
        void vscode.window.showInformationMessage('Zryna Run completed. See Zryna Run output for results.',
          ...(completed.target === 'javascript' ? ['Open JavaScript'] : []), 'Reveal Output').then(action => {
          if (action) return handle(() => inspect(action === 'Open JavaScript' ? 'javascript' : 'folder', completed))();
        });
      } finally { busy = false; }
    })),
    vscode.commands.registerCommand('zryna.openGeneratedJavaScript', handle(() => inspect('javascript'))),
    vscode.commands.registerCommand('zryna.revealRunOutput', handle(() => inspect('folder'))),
  );
}

module.exports = { registerRun };
