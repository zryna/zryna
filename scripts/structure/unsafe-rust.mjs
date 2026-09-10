import { fail } from './policy.mjs';
import { readSafe } from './repository.mjs';

const CONTRACT = 'zryna.workspace.json';
const ROOT_MANIFEST = 'Cargo.toml';
const EXCEPTION_ID = 'zryna-windows-filesystem';
const EXCEPTION_ROOT = 'crates/zryna-windows-filesystem';
const EXCEPTION_MODULE = `${EXCEPTION_ROOT}/src/windows.rs`;

function parseKey(text, context) {
  const parts = [];
  let index = 0;
  const space = () => { while (/\s/.test(text[index] ?? '')) index += 1; };
  while (index < text.length) {
    space();
    const quote = text[index];
    let part = '';
    if (quote === '"' || quote === "'") {
      index += 1;
      while (index < text.length && text[index] !== quote) {
        if (text[index] === '\\') fail(`${context} uses a noncanonical escaped TOML key`);
        part += text[index++];
      }
      if (text[index++] !== quote) fail(`${context} contains an unterminated TOML key`);
    } else {
      const match = text.slice(index).match(/^[A-Za-z0-9_-]+/);
      if (!match) fail(`${context} contains an invalid TOML key`);
      part = match[0];
      index += part.length;
    }
    parts.push(part);
    space();
    if (index === text.length) break;
    if (text[index++] !== '.') fail(`${context} contains an invalid dotted TOML key`);
  }
  if (parts.length === 0) fail(`${context} contains an empty TOML key`);
  return parts;
}

function assignmentIndex(line) {
  let quote;
  for (let index = 0; index < line.length; index += 1) {
    const character = line[index];
    if (quote) {
      if (character === '\\' && quote === '"') index += 1;
      else if (character === quote) quote = undefined;
    } else if (character === '"' || character === "'") quote = character;
    else if (character === '=') return index;
    else if (character === '#') return -1;
  }
  return -1;
}

function isLintPath(path) {
  return path[0] === 'lints' || (path[0] === 'workspace' && path[1] === 'lints');
}

function lintTables(document, manifest) {
  const tables = new Map();
  let current = [];
  for (const rawLine of document.split(/\r?\n/)) {
    const trimmed = rawLine.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    if (trimmed.startsWith('[')) {
      const comment = trimmed.indexOf('#');
      const header = (comment < 0 ? trimmed : trimmed.slice(0, comment)).trim();
      const array = header.startsWith('[[');
      const open = array ? 2 : 1;
      const close = array ? 2 : 1;
      if (!header.endsWith(']'.repeat(close))) fail(`${manifest} contains an invalid TOML table`);
      current = parseKey(header.slice(open, -close).trim(), `${manifest} table`);
      if (isLintPath(current)) {
        const canonical = `[${current.join('.')}]`;
        if (array || header !== canonical) {
          fail(`${manifest} lint tables must use canonical unquoted dotted spelling`);
        }
        if (tables.has(current.join('.'))) fail(`${manifest} contains a duplicate lint table`);
        tables.set(current.join('.'), new Map());
      }
      continue;
    }
    const equals = assignmentIndex(rawLine);
    if (equals < 0) continue;
    const keyText = rawLine.slice(0, equals).trim();
    const key = parseKey(keyText, `${manifest} assignment`);
    const fullPath = [...current, ...key];
    if (!isLintPath(fullPath)) continue;
    const tableName = current.join('.');
    const values = tables.get(tableName);
    if (!values || key.length !== 1 || keyText !== key[0]) {
      fail(`${manifest} lint declarations must use canonical table and key spelling`);
    }
    const value = rawLine.slice(equals + 1).replace(/\s+#.*$/, '').trim();
    if (!value || values.has(key[0])) fail(`${manifest} contains an invalid or duplicate lint`);
    values.set(key[0], value);
  }
  return tables;
}

function sameTable(actual, expected, label) {
  const ordered = table => [...(table ?? new Map())]
    .sort(([left], [right]) => left.localeCompare(right));
  if (JSON.stringify(ordered(actual)) !== JSON.stringify(ordered(expected))) {
    fail(`${label} must preserve the workspace lint contract exactly`);
  }
}

function requireInheritedLints(document, manifest) {
  const tables = lintTables(document, manifest);
  const inherited = tables.get('lints');
  if (tables.size !== 1 || inherited?.size !== 1 || inherited.get('workspace') !== 'true') {
    fail(`${manifest} must inherit workspace lints without override`);
  }
}

function validateExceptionManifest(document, workspaceRust, workspaceClippy) {
  const manifest = `${EXCEPTION_ROOT}/Cargo.toml`;
  const tables = lintTables(document, manifest);
  if (tables.size !== 2) fail(`${manifest} must contain only the two exact local lint tables`);
  const expectedRust = new Map(workspaceRust);
  expectedRust.set('unsafe_code', '"deny"');
  sameTable(tables.get('lints.rust'), expectedRust, `${manifest} Rust lints`);
  sameTable(tables.get('lints.clippy'), workspaceClippy, `${manifest} Clippy lints`);
}

function skipQuoted(source, start, quote) {
  let index = start + 1;
  while (index < source.length) {
    if (source[index] === '\\') index += 2;
    else if (source[index++] === quote) return index;
  }
  fail('Rust source contains an unterminated quoted literal');
}

function rawStringEnd(source, start) {
  const match = source.slice(start).match(/^(?:br|cr|r)(#*)"/);
  if (!match) return undefined;
  const terminator = `"${match[1]}`;
  const end = source.indexOf(terminator, start + match[0].length);
  if (end < 0) fail('Rust source contains an unterminated raw string');
  return end + terminator.length;
}

function charEnd(source, start) {
  let index = start + 1;
  if (source[index] === '\\') {
    const escape = source[index + 1];
    if ('\\\'"nrt0'.includes(escape)) index += 2;
    else if (escape === 'x' && /^[0-9A-Fa-f]{2}$/.test(source.slice(index + 2, index + 4))) {
      index += 4;
    } else if (escape === 'u' && source[index + 2] === '{') {
      const close = source.indexOf('}', index + 3);
      if (close < 0) fail('Rust source contains an unterminated character escape');
      index = close + 1;
    } else return undefined;
  } else {
    const point = source.codePointAt(index);
    if (point === undefined || point === 10 || point === 13) return undefined;
    index += point > 0xffff ? 2 : 1;
  }
  return source[index] === "'" ? index + 1 : undefined;
}

function rustTokens(source) {
  const tokens = [];
  for (let index = 0; index < source.length;) {
    if (/\s/.test(source[index])) { index += 1; continue; }
    if (source.startsWith('//', index)) {
      const end = source.indexOf('\n', index + 2);
      index = end < 0 ? source.length : end + 1;
      continue;
    }
    if (source.startsWith('/*', index)) {
      let depth = 1;
      index += 2;
      while (index < source.length && depth > 0) {
        if (source.startsWith('/*', index)) { depth += 1; index += 2; }
        else if (source.startsWith('*/', index)) { depth -= 1; index += 2; }
        else index += 1;
      }
      if (depth > 0) fail('Rust source contains an unterminated block comment');
      continue;
    }
    const rawEnd = rawStringEnd(source, index);
    if (rawEnd !== undefined) { index = rawEnd; continue; }
    if (source[index] === '"') { index = skipQuoted(source, index, '"'); continue; }
    if ((source[index] === 'b' || source[index] === 'c') && source[index + 1] === '"') {
      index = skipQuoted(source, index + 1, '"');
      continue;
    }
    const byteChar = source[index] === 'b' && source[index + 1] === "'";
    const character = byteChar ? charEnd(source, index + 1)
      : source[index] === "'" ? charEnd(source, index) : undefined;
    if (character !== undefined) { index = character; continue; }
    const identifier = source.slice(index).match(/^[A-Za-z_][A-Za-z0-9_]*/);
    if (identifier) { tokens.push(identifier[0]); index += identifier[0].length; continue; }
    tokens.push(source[index++]);
  }
  return tokens;
}

function weakeningAttributes(tokens) {
  let count = 0;
  for (let index = 0; index < tokens.length; index += 1) {
    if (tokens[index] !== '#') continue;
    let cursor = index + 1;
    if (tokens[cursor] === '!') cursor += 1;
    if (tokens[cursor] !== '[') continue;
    let depth = 1;
    const contents = [];
    while (++cursor < tokens.length && depth > 0) {
      if (tokens[cursor] === '[') depth += 1;
      else if (tokens[cursor] === ']') depth -= 1;
      if (depth > 0) contents.push(tokens[cursor]);
    }
    for (let item = 0; item < contents.length; item += 1) {
      if (!['allow', 'expect'].includes(contents[item]) || contents[item + 1] !== '(') continue;
      let parentheses = 1;
      let hasUnsafeCode = false;
      while (++item < contents.length && parentheses > 0) {
        if (contents[item] === '(') parentheses += 1;
        else if (contents[item] === ')') parentheses -= 1;
        else if (contents[item] === 'unsafe_code') hasUnsafeCode = true;
      }
      if (hasUnsafeCode) count += 1;
    }
  }
  return count;
}

function validateRust(path, document) {
  const tokens = rustTokens(document);
  const weakening = weakeningAttributes(tokens);
  if (path === EXCEPTION_MODULE) {
    if (weakening !== 1 || !document.startsWith('#![allow(unsafe_code)]\n')) {
      fail(`${EXCEPTION_MODULE} must contain the one exact unsafe-code allowance`);
    }
  } else if (weakening > 0 || tokens.includes('unsafe')) {
    fail(`unsafe Rust is confined to ${EXCEPTION_MODULE}; found weakening or unsafe code in ${path}`);
  }
}

/** Validates the complete unsafe-Rust exception from an exact repository file map. */
export function validateUnsafeRustDocuments(files) {
  const rootManifest = files.get(ROOT_MANIFEST);
  const contractText = files.get(CONTRACT);
  if (rootManifest === undefined || contractText === undefined) {
    fail('unsafe-Rust enforcement requires Cargo.toml and zryna.workspace.json');
  }
  const rootTables = lintTables(rootManifest, ROOT_MANIFEST);
  const workspaceRust = rootTables.get('workspace.lints.rust');
  const workspaceClippy = rootTables.get('workspace.lints.clippy');
  if (workspaceRust?.get('unsafe_code') !== '"forbid"' || !workspaceClippy) {
    fail('workspace lint defaults and unsafe_code forbid must remain declared');
  }

  let contract;
  try { contract = JSON.parse(contractText); }
  catch { fail('zryna.workspace.json must be valid JSON for unsafe-Rust enforcement'); }
  if (!Array.isArray(contract.members)) fail('unsafe-Rust enforcement requires workspace members');
  const exceptions = contract.members.filter(member => member.id === EXCEPTION_ID);
  if (exceptions.length !== 1 || exceptions[0].root !== EXCEPTION_ROOT) {
    fail('unsafe-Rust exception component must have its exact registered identity and root');
  }

  const registered = new Set();
  for (const member of contract.members) {
    const manifest = `${member.root}/Cargo.toml`;
    registered.add(manifest);
    const document = files.get(manifest);
    if (document === undefined) fail(`registered Rust component manifest is absent: ${manifest}`);
    if (member.id === EXCEPTION_ID) validateExceptionManifest(document, workspaceRust, workspaceClippy);
    else requireInheritedLints(document, manifest);
  }
  for (const [path, document] of files) {
    const lower = path.toLowerCase();
    if (lower.endsWith('/cargo.toml') && path !== ROOT_MANIFEST && !registered.has(path)) {
      requireInheritedLints(document, path);
    }
    if (lower.endsWith('.rs')) validateRust(path, document);
  }
  if (!files.has(EXCEPTION_MODULE)) fail(`approved unsafe module is absent: ${EXCEPTION_MODULE}`);
}

export function validateUnsafeRustWorkspace(root, paths) {
  const pathSet = new Set(paths);
  if (!pathSet.has(ROOT_MANIFEST) && !pathSet.has(CONTRACT)) return;
  const files = new Map();
  for (const path of paths) {
    const lower = path.toLowerCase();
    if (path === ROOT_MANIFEST || path === CONTRACT || lower.endsWith('/cargo.toml') || lower.endsWith('.rs')) {
      const document = readSafe(root, path, true);
      if (document !== undefined) files.set(path, document);
    }
  }
  validateUnsafeRustDocuments(files);
}
