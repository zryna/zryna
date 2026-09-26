'use strict';

const vscode = require('vscode');

function activate() {
  void (async () => {
    try {
      await require('../host-acceptance.cjs').run();
    } catch (error) {
      console.error(error);
    } finally {
      try { await vscode.workspace.saveAll(); }
      finally { await vscode.commands.executeCommand('workbench.action.quit'); }
    }
  })();
}

module.exports = { activate };
