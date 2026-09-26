'use strict';

const SOURCE_LIMITS = Object.freeze({ 'i32-v1': 1024, 'control-flow-v1': 2 * 1024 * 1024 });

function sourceText(bytes, profile = 'i32-v1') {
  if (!Object.hasOwn(SOURCE_LIMITS, profile)) throw new Error('Unsupported Run profile.');
  const limit = SOURCE_LIMITS[profile];
  if (bytes.length > limit) throw new Error(`Run ${profile} supports at most ${limit} UTF-8 source bytes.`);
  return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
}

// Lexical discovery is a picker aid only. The compiler validates the complete saved source.
function exportsIn(text, profile = 'i32-v1') {
  if (!Object.hasOwn(SOURCE_LIMITS, profile)) throw new Error('Unsupported Run profile.');
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
      if (!/^[A-Za-z_][A-Za-z_0-9]*$/.test(parameter) || clean[cursor++] !== ':') {
        valid = false;
        break;
      }
      const type = clean[cursor++];
      if (type !== 'i32' && (profile !== 'control-flow-v1' || type !== 'bool')) {
        valid = false;
        break;
      }
      parameters.push({ name: parameter, type });
      if (clean[cursor] === ')') break;
      if (clean[cursor++] !== ',') { valid = false; break; }
    }
    if (!valid || clean[cursor++] !== ')' || clean[cursor++] !== ':') continue;
    const resultType = clean[cursor++];
    if ((resultType === 'i32' || (profile === 'control-flow-v1' && resultType === 'bool'))
      && clean[cursor] === '{') found.push({ name, parameters, resultType });
  }
  if (new Set(found.map(item => item.name)).size !== found.length) throw new Error('Duplicate exported function names.');
  if (!found.length) throw new Error(`No exported function with explicit ${profile === 'i32-v1' ? 'i32' : 'i32/bool'} parameters and result. Run supports the selected ${profile} profile.`);
  return found;
}

function argumentError(value, type = 'i32') {
  if (type === 'bool') return value === 'true' || value === 'false' ? undefined : 'Enter exactly true or false.';
  if (type !== 'i32' || typeof value !== 'string' || !/^(?:0|-?[1-9][0-9]*)$/.test(value)
    || !Number.isInteger(Number(value)) || Number(value) < -2147483648 || Number(value) > 2147483647) {
    return 'Enter an i32 decimal integer from -2147483648 to 2147483647 (no spaces or leading zeros).';
  }
  return undefined;
}

async function selectRun(window, bytes, requested) {
  if (requested !== undefined) {
    const { profile, name, target, args } = requested ?? {};
    if (!Object.hasOwn(SOURCE_LIMITS, profile) || !['javascript', 'webassembly'].includes(target)
      || typeof name !== 'string' || !Array.isArray(args)) throw new Error('Invalid Run selection.');
    const candidate = exportsIn(sourceText(bytes, profile), profile).find(item => item.name === name);
    if (!candidate || args.length !== candidate.parameters.length
      || args.some((argument, index) => argument?.type !== candidate.parameters[index].type
        || argumentError(argument.value, argument.type))) throw new Error('Invalid Run selection.');
    return { profile, name, target, args: args.map(argument => ({ type: argument.type, value: argument.value })),
      resultType: candidate.resultType };
  }
  const profile = await window.showQuickPick(['i32-v1', 'control-flow-v1'], {
    title: 'Zryna: Select Run profile', ignoreFocusOut: true,
  });
  if (!profile) return;
  const choices = exportsIn(sourceText(bytes, profile), profile);
  const selected = await window.showQuickPick(choices.map(item => ({
    label: item.name,
    description: `(${item.parameters.map(parameter => `${parameter.name}: ${parameter.type}`).join(', ')}): ${item.resultType}`,
    item,
  })), { title: 'Zryna: Run saved source — select export', ignoreFocusOut: true });
  if (!selected) return;
  const target = await window.showQuickPick(['javascript', 'webassembly'], {
    title: 'Zryna: Select execution target', ignoreFocusOut: true,
  });
  if (!target) return;
  const args = [];
  for (const parameter of selected.item.parameters) {
    const value = await window.showInputBox({ title: `Zryna: ${selected.item.name} — ${parameter.name}: ${parameter.type}`,
      prompt: parameter.type === 'bool' ? 'Required true or false' : 'Required signed 32-bit decimal integer',
      validateInput: input => argumentError(input, parameter.type), ignoreFocusOut: true });
    if (value === undefined) return;
    const error = argumentError(value, parameter.type);
    if (error) throw new Error(error);
    args.push({ type: parameter.type, value });
  }
  return { profile, name: selected.item.name, target, args, resultType: selected.item.resultType };
}

module.exports = { SOURCE_LIMITS, sourceText, exportsIn, argumentError, selectRun };
