'use strict';

const MAX_SOURCE_BYTES = 1024;

function sourceText(bytes) {
  if (bytes.length > MAX_SOURCE_BYTES) throw new Error('Standalone Run supports at most 1024 UTF-8 source bytes.');
  return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
}

// Lexical discovery is a picker aid only. The compiler validates the complete saved source.
function exportsIn(text) {
  const tokens = text.match(/\/\*[\s\S]*?\*\/|\/\/[^\r\n]*|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|`(?:\\.|[^`\\])*`|[A-Za-z_$][\w$]*|\S/g) ?? [];
  const clean = tokens.filter(token => !token.startsWith('//') && !token.startsWith('/*'));
  const found = [];
  let depth = 0;
  for (let i = 0; i < clean.length; i++) {
    if (clean[i] === '{') depth++;
    if (clean[i] === '}') depth--;
    if (depth !== 0 || clean[i] !== 'export' || clean[i + 1] !== 'function') continue;
    const name = clean[i + 2];
    let cursor = i + 3;
    if (!/^[A-Za-z_][A-Za-z_0-9]*$/.test(name) || clean[cursor++] !== '(') continue;
    const parameters = [];
    let valid = true;
    while (cursor < clean.length && clean[cursor] !== ')') {
      const parameter = clean[cursor++];
      if (!/^[A-Za-z_][A-Za-z_0-9]*$/.test(parameter) || clean[cursor++] !== ':' || clean[cursor++] !== 'i32') {
        valid = false;
        break;
      }
      parameters.push(parameter);
      if (clean[cursor] === ')') break;
      if (clean[cursor++] !== ',') { valid = false; break; }
    }
    if (valid && clean[cursor++] === ')' && clean[cursor++] === ':' && clean[cursor++] === 'i32'
      && clean[cursor] === '{') found.push({ name, parameters });
  }
  if (new Set(found.map(item => item.name)).size !== found.length) throw new Error('Duplicate exported function names.');
  if (!found.length) throw new Error('No exported function with explicit i32 parameters and i32 result. Run supports the standalone i32-v1 profile.');
  return found;
}

function argumentError(value) {
  if (!/^(?:0|-?[1-9][0-9]*)$/.test(value) || !Number.isInteger(Number(value))
    || Number(value) < -2147483648 || Number(value) > 2147483647) {
    return 'Enter an i32 decimal integer from -2147483648 to 2147483647 (no spaces or leading zeros).';
  }
  return undefined;
}

async function selectRun(window, bytes) {
  const choices = exportsIn(sourceText(bytes));
  const selected = await window.showQuickPick(choices.map(item => ({
    label: item.name, description: `(${item.parameters.map(name => `${name}: i32`).join(', ')}): i32`, item,
  })), { title: 'Zryna: Run saved source — select export', ignoreFocusOut: true });
  if (!selected) return;
  const target = await window.showQuickPick(['javascript', 'webassembly'], {
    title: 'Zryna: Select execution target', ignoreFocusOut: true,
  });
  if (!target) return;
  const args = [];
  for (const parameter of selected.item.parameters) {
    const value = await window.showInputBox({ title: `Zryna: ${selected.item.name} — ${parameter}: i32`,
      prompt: 'Required signed 32-bit decimal integer', validateInput: argumentError, ignoreFocusOut: true });
    if (value === undefined) return;
    const error = argumentError(value);
    if (error) throw new Error(error);
    args.push(value);
  }
  return { name: selected.item.name, target, args };
}

module.exports = { sourceText, exportsIn, argumentError, selectRun };
