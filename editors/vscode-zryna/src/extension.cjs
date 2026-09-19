'use strict';

const vscode = require('vscode');
const { Connection } = require('./connection.cjs');
const { registerRun } = require('./run-command.cjs');

let active;
let starting = Promise.resolve();
let diagnostics;

function configuration() {
  const config = vscode.workspace.getConfiguration('zryna');
  // Executables are selected only through explicit user configuration, never workspace settings.
  return Object.fromEntries(['serverPath', 'compilerRoot', 'nodePath'].map(key =>
    [key, config.inspect(key)?.globalValue ?? '']));
}

function range(value, document) {
  if (!value || !validPosition(value.start, document) || !validPosition(value.end, document)) {
    throw new Error('Invalid Zryna source range.');
  }
  const result = new vscode.Range(value.start.line, value.start.character, value.end.line, value.end.character);
  if (result.start.line !== value.start.line || result.start.character !== value.start.character) {
    throw new Error('Reversed Zryna source range.');
  }
  return result;
}

function validPosition(position, document) {
  return position && Number.isSafeInteger(position.line) && Number.isSafeInteger(position.character)
    && position.line >= 0 && position.line < document.lineCount && position.character >= 0
    && position.character <= document.lineAt(position.line).text.length;
}

function publish(state, method, params) {
  if (method !== 'textDocument/publishDiagnostics' || active !== state
    || params?.uri !== state.uri || params.version !== state.document.version) return;
  if (!Array.isArray(params.diagnostics) || params.diagnostics.length > 10000) return;
  try {
    const values = params.diagnostics.map(item => {
      if (typeof item.message !== 'string') throw new Error('Invalid diagnostic.');
      const diagnostic = new vscode.Diagnostic(range(item.range, state.document), item.message,
        [undefined, vscode.DiagnosticSeverity.Error, vscode.DiagnosticSeverity.Warning,
          vscode.DiagnosticSeverity.Information, vscode.DiagnosticSeverity.Hint][item.severity]
          ?? vscode.DiagnosticSeverity.Error);
      diagnostic.source = 'Zryna';
      diagnostic.code = item.code;
      return diagnostic;
    });
    diagnostics.set(state.document.uri, values);
  } catch { diagnostics.delete(state.document.uri); }
}

async function connect(document) {
  if (document.isClosed || !vscode.workspace.isTrusted || document.uri.scheme !== 'file' || document.languageId !== 'zryna') {
    throw new Error('Zryna requires a trusted local file workspace.');
  }
  const folder = vscode.workspace.getWorkspaceFolder(document.uri);
  if (!folder) throw new Error('Open the Zryna project folder first.');
  if (active?.uri === document.uri.toString() && !active.connection.closed) return active;
  await disconnect();
  const state = { document, uri: document.uri.toString() };
  state.connection = new Connection(configuration(), (method, params) => publish(state, method, params), () => {
    diagnostics.delete(document.uri);
  });
  active = state;
  try {
    const result = await state.connection.request('initialize', {
      rootUri: folder.uri.toString(), capabilities: { general: { positionEncodings: ['utf-16'] } },
    });
    const cap = result?.capabilities;
    if (result?.serverInfo?.name !== 'zryna-language-server' || result.serverInfo.version !== '0.2.3'
      || cap?.experimental?.zrynaFormattingProfile !== 'scalar-format-v1'
      || cap.positionEncoding !== 'utf-16' || !cap.documentFormattingProvider || !cap.documentRangeFormattingProvider) {
      throw new Error('This extension needs the matching source build with scalar-format-v1; released v0.2.3 is incompatible.');
    }
    state.connection.notify('initialized', {});
    state.connection.notify('textDocument/didOpen', { textDocument: {
      uri: state.uri, languageId: 'zryna', version: document.version, text: document.getText(),
    } });
    state.ready = true;
    return state;
  } catch (error) {
    state.connection.fail(error);
    if (active === state) active = undefined;
    throw error;
  }
}

function ensure(document) {
  const next = starting.catch(() => {}).then(() => connect(document));
  starting = next;
  return next;
}

async function query(document, method, params, token) {
  const state = await ensure(document);
  const version = document.version;
  const result = await state.connection.request(method, { textDocument: { uri: state.uri }, ...params }, token);
  if (active !== state || document.version !== version || document.isClosed || token?.isCancellationRequested) {
    throw new Error('The document changed before the Zryna response.');
  }
  return result;
}

async function format(document, selection, options, token) {
  const result = await query(document, selection ? 'textDocument/rangeFormatting' : 'textDocument/formatting', {
    options: { tabSize: options.tabSize, insertSpaces: options.insertSpaces },
    ...(selection ? { range: { start: selection.start, end: selection.end } } : {}),
  }, token);
  if (!Array.isArray(result) || result.length > 10000) throw new Error('Invalid Zryna edits.');
  let previousEnd = 0;
  return result.map(edit => {
    const selected = range(edit.range, document);
    if (typeof edit.newText !== 'string' || (selection && !selection.contains(selected))
      || document.offsetAt(selected.start) < previousEnd) throw new Error('Invalid Zryna edits.');
    previousEnd = document.offsetAt(selected.end);
    return vscode.TextEdit.replace(selected, edit.newText);
  });
}

async function disconnect() {
  const previous = active;
  active = undefined;
  if (previous) {
    diagnostics?.delete(previous.document.uri);
    await previous.connection.stop().catch(() => {});
  }
}

function activate(context) {
  registerRun(vscode, context);
  diagnostics = vscode.languages.createDiagnosticCollection('zryna');
  const selector = { scheme: 'file', language: 'zryna' };
  context.subscriptions.push(diagnostics,
    vscode.languages.registerDocumentFormattingEditProvider(selector, {
      provideDocumentFormattingEdits: (document, options, token) => format(document, null, options, token),
    }),
    vscode.languages.registerDocumentRangeFormattingEditProvider(selector, {
      provideDocumentRangeFormattingEdits: (document, selection, options, token) => format(document, selection, options, token),
    }),
    vscode.languages.registerDefinitionProvider(selector, {
      async provideDefinition(document, position, token) {
        const result = await query(document, 'textDocument/definition', { position }, token);
        if (result === null) return null;
        if (result?.uri !== document.uri.toString()) throw new Error('Foreign definition result.');
        return new vscode.Location(document.uri, range(result.range, document));
      },
    }),
    vscode.workspace.onDidChangeTextDocument(event => {
      if (active?.ready && active.document === event.document && event.contentChanges.length) {
        try {
          active.connection.notify('textDocument/didChange', {
            textDocument: { uri: active.uri, version: event.document.version },
            contentChanges: [{ text: event.document.getText() }],
          });
          diagnostics.delete(event.document.uri);
        } catch (error) { active.connection.fail(error); }
      }
    }),
    vscode.workspace.onDidCloseTextDocument(document => {
      if (active?.document === document) void disconnect();
    }),
    vscode.workspace.onDidChangeConfiguration(event => {
      if (event.affectsConfiguration('zryna')) void disconnect();
    }),
    vscode.window.onDidChangeActiveTextEditor(editor => {
      if (editor?.document.languageId === 'zryna') void ensure(editor.document).catch(() => {});
    }),
  );
  const document = vscode.window.activeTextEditor?.document;
  if (document?.languageId === 'zryna') {
    void ensure(document).catch(error => vscode.window.showErrorMessage(error.message));
  }
}

module.exports = { activate, deactivate: disconnect };
