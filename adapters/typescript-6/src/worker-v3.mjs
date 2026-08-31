import process from 'node:process';
import { once } from 'node:events';
import { TextDecoder } from 'node:util';

import ts from '@typescript/typescript6';

function boundedTestLimit(name, productionLimit) {
  if (process.env.NODE_ENV !== 'test') return productionLimit;
  const configured = process.env[`ZRYNA_TEST_${name}`];
  if (configured === undefined) return productionLimit;
  const value = Number(configured);
  if (!Number.isSafeInteger(value) || value < 1 || value > productionLimit) {
    throw new Error(`invalid lowered test limit for ${name}`);
  }
  return value;
}

const protocolVersion = 3;
const expectedProviderVersion = '6.0.3';
const providerVersion = ts.version;
if (providerVersion !== expectedProviderVersion) {
  throw new Error(`the TypeScript provider must be exactly ${expectedProviderVersion}`);
}

const maxRequestBytes = boundedTestLimit('REQUEST_BYTES', 72 * 1024 * 1024);
const maxResponseBytes = boundedTestLimit('RESPONSE_BYTES', 64 * 1024 * 1024);
const maxFiles = boundedTestLimit('FILES', 4096);
const maxSourceFileBytes = boundedTestLimit('SOURCE_FILE_BYTES', 8 * 1024 * 1024);
const maxSourceBytes = boundedTestLimit('SOURCE_BYTES', 8 * 1024 * 1024);
const maxFunctionsPerFile = boundedTestLimit('FUNCTIONS_PER_FILE', 4096);
const maxFunctionsPerProject = boundedTestLimit('FUNCTIONS_PER_PROJECT', 16_384);
const maxImportsPerFile = boundedTestLimit('IMPORTS_PER_FILE', 4096);
const maxImportsPerProject = boundedTestLimit('IMPORTS_PER_PROJECT', 65_536);
const maxBindingsPerImport = boundedTestLimit('BINDINGS_PER_IMPORT', 256);
const maxBindingsPerProject = boundedTestLimit('BINDINGS_PER_PROJECT', 65_536);
const maxParametersPerFunction = boundedTestLimit('PARAMETERS_PER_FUNCTION', 256);
const maxParametersPerProject = boundedTestLimit('PARAMETERS_PER_PROJECT', 262_144);
const maxBlocksPerFunction = boundedTestLimit('BLOCKS_PER_FUNCTION', 4096);
const maxBlocksPerProject = boundedTestLimit('BLOCKS_PER_PROJECT', 65_536);
const maxStatementsPerFunction = boundedTestLimit('STATEMENTS_PER_FUNCTION', 4096);
const maxStatementsPerProject = boundedTestLimit('STATEMENTS_PER_PROJECT', 65_536);
const maxExpressionsPerFunction = boundedTestLimit('EXPRESSIONS_PER_FUNCTION', 16_384);
const maxExpressionsPerProject = boundedTestLimit('EXPRESSIONS_PER_PROJECT', 262_144);
const maxLocalsPerFunction = boundedTestLimit('LOCALS_PER_FUNCTION', 4096);
const maxLocalsPerProject = boundedTestLimit('LOCALS_PER_PROJECT', 65_536);
const maxNesting = boundedTestLimit('NESTING', 128);
const maxCallArguments = boundedTestLimit('CALL_ARGUMENTS', 256);
const maxDiagnostics = 256;
const maxDiagnosticCharacters = 4096;
const maxNameCharacters = 1024;
const maxIntegerSpellingBytes = 64;
const maxJsonDepth = 8;
const maxJsonContainers = maxFiles + 4;
const maxJsonFields = maxFiles * 2 + 8;

class AdapterError extends Error {
  constructor(code, message) {
    super(message);
    this.code = code;
  }
}

function failRequest(message) {
  throw new AdapterError('ZRYNA-F1001', message);
}

function failBudget(message) {
  throw new AdapterError('ZRYNA-F1002', message);
}

function failInvariant(message) {
  throw new AdapterError('ZRYNA-F1003', message);
}

function isRecord(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function requireExactKeys(value, allowed, label) {
  if (!isRecord(value)) failRequest(`${label} must be an object`);
  const actual = Object.keys(value).sort();
  const expected = [...allowed].sort();
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) {
    failRequest(`${label} contains unknown or missing fields`);
  }
}

function requireRequestId(value) {
  if (!Number.isSafeInteger(value) || value < 0 || value > 0xffff_ffff) {
    failRequest('request id must be an unsigned 32-bit integer');
  }
}

function isRequestId(value) {
  return Number.isSafeInteger(value) && value >= 0 && value <= 0xffff_ffff;
}

function rejectDuplicateObjectKeys(text) {
  const stack = [];
  let containers = 0;
  let fields = 0;
  for (let index = 0; index < text.length; index += 1) {
    const character = text[index];
    if (character === '{' || character === '[') {
      containers += 1;
      if (stack.length >= maxJsonDepth) failBudget('request exceeds the JSON depth limit');
      if (containers > maxJsonContainers) failBudget('request exceeds the JSON container limit');
      stack.push(character === '{' ? { kind: 'object', keys: new Set() } : { kind: 'array' });
      continue;
    }
    if (character === '}' || character === ']') {
      stack.pop();
      continue;
    }
    if (character !== '"') continue;
    const start = index;
    index += 1;
    let escaped = false;
    while (index < text.length) {
      const current = text[index];
      if (escaped) escaped = false;
      else if (current === '\\') escaped = true;
      else if (current === '"') break;
      index += 1;
    }
    if (index >= text.length) return;
    let next = index + 1;
    while (/\s/.test(text[next] ?? '')) next += 1;
    const frame = stack.at(-1);
    if (text[next] !== ':' || frame?.kind !== 'object') continue;
    fields += 1;
    if (fields > maxJsonFields) failBudget('request exceeds the JSON field limit');
    let key;
    try {
      key = JSON.parse(text.slice(start, index + 1));
    } catch {
      return;
    }
    if (frame.keys.has(key)) failRequest('request contains a duplicate object field');
    frame.keys.add(key);
  }
}

function validatePortablePath(path) {
  if (typeof path !== 'string' || path.length === 0) failRequest('source path must be a string');
  if (!/^[\x20-\x7e]+$/.test(path) || Buffer.byteLength(path, 'utf8') > 1024) {
    failRequest('source path must be bounded printable ASCII');
  }
  if (path.startsWith('/') || path.includes('\\')) {
    failRequest('source path must be workspace-relative and use forward slashes');
  }
  const components = path.split('/');
  if (components.length > 32) failRequest('source path exceeds the component limit');
  for (const component of components) {
    if (
      component.length === 0 || component === '.' || component === '..' ||
      component.length > 255 || component.endsWith('.') || component.endsWith(' ') ||
      /[<>:"|?*]/.test(component)
    ) failRequest('source path contains a non-portable component');
    const stem = component.split('.')[0].toLowerCase();
    if (/^(con|prn|aux|nul|com[1-9]|lpt[1-9])$/.test(stem)) {
      failRequest('source path contains a reserved device name');
    }
  }
  return path;
}

const utf8OffsetMaps = new WeakMap();

function buildUtf8OffsetMap(sourceFile) {
  const offsets = new Uint32Array(sourceFile.text.length + 1);
  let bytes = 0;
  for (let index = 0; index < sourceFile.text.length; index += 1) {
    offsets[index] = bytes;
    const codeUnit = sourceFile.text.charCodeAt(index);
    if (codeUnit <= 0x7f) bytes += 1;
    else if (codeUnit <= 0x7ff) bytes += 2;
    else if (codeUnit >= 0xd800 && codeUnit <= 0xdbff) {
      offsets[index + 1] = 0xffff_ffff;
      index += 1;
      bytes += 4;
    } else bytes += 3;
  }
  offsets[sourceFile.text.length] = bytes;
  utf8OffsetMaps.set(sourceFile, offsets);
}

function utf8ByteOffset(sourceFile, offset) {
  const offsets = utf8OffsetMaps.get(sourceFile);
  if (!offsets || !Number.isInteger(offset) || offset < 0 || offset > sourceFile.text.length) {
    failInvariant('TypeScript returned an invalid UTF-16 source offset');
  }
  if (offsets[offset] === 0xffff_ffff) failInvariant('TypeScript returned an offset inside a surrogate pair');
  return offsets[offset];
}

function spanFromOffsets(sourceFile, file, start, end) {
  return { file, start: utf8ByteOffset(sourceFile, start), end: utf8ByteOffset(sourceFile, end) };
}

function nodeSpan(node, sourceFile, file) {
  return spanFromOffsets(sourceFile, file, node.getStart(sourceFile), node.getEnd());
}

function compactText(value) {
  const text = String(value).replaceAll(/\s+/g, ' ').trim();
  return [...text].slice(0, maxDiagnosticCharacters).join('') || 'provider diagnostic';
}

function compareDiagnostics(left, right) {
  const a = left.location.kind === 'source' ? left.location.span : null;
  const b = right.location.kind === 'source' ? right.location.span : null;
  const ak = [a?.file ?? -1, a?.start ?? -1, a?.end ?? -1, left.code, left.message, left.guidance];
  const bk = [b?.file ?? -1, b?.start ?? -1, b?.end ?? -1, right.code, right.message, right.guidance];
  for (let index = 0; index < ak.length; index += 1) {
    if (ak[index] < bk[index]) return -1;
    if (ak[index] > bk[index]) return 1;
  }
  return 0;
}

class DiagnosticCollector {
  #diagnostics = [];
  #truncated = false;

  add(diagnostic) {
    if (this.#diagnostics.length < maxDiagnostics - 1) this.#diagnostics.push(diagnostic);
    else this.#truncated = true;
  }

  located(code, sourceFile, file, node, message, guidance) {
    this.add({
      code,
      severity: 'error',
      location: { kind: 'source', span: nodeSpan(node, sourceFile, file) },
      message: compactText(message),
      guidance: compactText(guidance),
    });
  }

  unsupported(node, sourceFile, file, context) {
    const kind = ts.SyntaxKind[node.kind] ?? `kind ${node.kind}`;
    const span = nodeSpan(node, sourceFile, file);
    throw new AdapterError(
      'ZRYNA-F2002',
      `${context} uses unsupported syntax '${kind}' at file ${span.file} bytes ${span.start}..${span.end}`,
    );
  }

  finish() {
    this.#diagnostics.sort(compareDiagnostics);
    if (this.#truncated) {
      this.#diagnostics.push({
        code: 'ZRYNA-F2003', severity: 'error', location: { kind: 'global' },
        message: 'frontend diagnostics exceeded the deterministic limit',
        guidance: 'reduce unsupported or malformed source before analysis',
      });
    }
    return this.#diagnostics;
  }
}

function normalizedIdentifier(node, sourceFile, file, collector, context) {
  const spelling = node.getText(sourceFile);
  if (spelling !== node.text || [...spelling].length > maxNameCharacters) {
    collector.unsupported(node, sourceFile, file, context);
    return null;
  }
  return { text: spelling, span: nodeSpan(node, sourceFile, file) };
}

function findToken(node, kind, sourceFile) {
  return node.getChildren(sourceFile).find((child) => child.kind === kind) ?? null;
}

function requiredToken(node, kind, sourceFile, label) {
  const token = findToken(node, kind, sourceFile);
  if (!token) failInvariant(`TypeScript omitted ${label}`);
  return token;
}

function enforceParserNesting(text) {
  const scanner = ts.createScanner(ts.ScriptTarget.Latest, true, ts.LanguageVariant.Standard, text);
  let depth = 0;
  for (let token = scanner.scan(); token !== ts.SyntaxKind.EndOfFileToken; token = scanner.scan()) {
    if (token === ts.SyntaxKind.OpenBraceToken || token === ts.SyntaxKind.OpenBracketToken || token === ts.SyntaxKind.OpenParenToken) {
      depth += 1;
      if (depth > maxNesting) failBudget('source exceeds the nesting limit');
    } else if (token === ts.SyntaxKind.CloseBraceToken || token === ts.SyntaxKind.CloseBracketToken || token === ts.SyntaxKind.CloseParenToken) {
      depth = Math.max(0, depth - 1);
    }
  }
}

function normalizeType(node, insertionOffset, sourceFile, file, collector, context) {
  if (!node) {
    return { span: spanFromOffsets(sourceFile, file, insertionOffset, insertionOffset), kind: { kind: 'missing' } };
  }
  let name = null;
  if (node.kind === ts.SyntaxKind.AnyKeyword) name = 'any';
  else if (ts.isTypeReferenceNode(node) && ts.isIdentifier(node.typeName) && !node.typeArguments?.length) {
    name = normalizedIdentifier(node.typeName, sourceFile, file, collector, context)?.text ?? null;
  }
  if (name === null || [...name].length > maxNameCharacters) {
    collector.unsupported(node, sourceFile, file, context);
    return null;
  }
  return { span: nodeSpan(node, sourceFile, file), kind: { kind: 'named', name } };
}

function normalizeParameter(parameter, sourceFile, file, collector, index) {
  let valid = true;
  let name = null;
  if (!ts.isIdentifier(parameter.name) || parameter.name.text === 'this') {
    collector.unsupported(parameter.name, sourceFile, file, `parameter ${index} name`);
    valid = false;
  } else {
    name = normalizedIdentifier(parameter.name, sourceFile, file, collector, `parameter ${index} name`);
    valid &&= name !== null;
  }
  for (const token of [parameter.dotDotDotToken, parameter.questionToken, parameter.initializer, ...(parameter.modifiers ?? [])]) {
    if (token) {
      collector.unsupported(token, sourceFile, file, `parameter ${index}`);
      valid = false;
    }
  }
  const typeSyntax = normalizeType(parameter.type, parameter.name.getEnd(), sourceFile, file, collector, `parameter ${index} annotation`);
  valid &&= typeSyntax !== null;
  return valid ? { span: nodeSpan(parameter, sourceFile, file), name, type_syntax: typeSyntax } : null;
}

function pushExpression(context, expression) {
  if (context.expressions.length >= maxExpressionsPerFunction) failBudget('function exceeds the expression limit');
  context.budgets.expressions += 1;
  if (context.budgets.expressions > maxExpressionsPerProject) failBudget('project exceeds the expression limit');
  const id = context.expressions.length;
  context.expressions.push(expression);
  return id;
}

const binaryKinds = new Map([
  [ts.SyntaxKind.PlusToken, 'addition'],
  [ts.SyntaxKind.MinusToken, 'subtraction'],
  [ts.SyntaxKind.AsteriskToken, 'multiplication'],
  [ts.SyntaxKind.EqualsEqualsEqualsToken, 'equal'],
  [ts.SyntaxKind.ExclamationEqualsEqualsToken, 'not-equal'],
  [ts.SyntaxKind.LessThanToken, 'less-than'],
  [ts.SyntaxKind.LessThanEqualsToken, 'less-equal'],
  [ts.SyntaxKind.GreaterThanToken, 'greater-than'],
  [ts.SyntaxKind.GreaterThanEqualsToken, 'greater-equal'],
]);

function normalizeExpression(node, context, depth = 1) {
  const { sourceFile, file, collector } = context;
  if (depth > maxNesting) {
    collector.unsupported(node, sourceFile, file, 'expression depth');
    return null;
  }
  if (ts.isIdentifier(node)) {
    const name = normalizedIdentifier(node, sourceFile, file, collector, 'reference');
    return name === null ? null : pushExpression(context, { span: nodeSpan(node, sourceFile, file), kind: { kind: 'reference', name } });
  }
  if (node.kind === ts.SyntaxKind.TrueKeyword || node.kind === ts.SyntaxKind.FalseKeyword) {
    return pushExpression(context, { span: nodeSpan(node, sourceFile, file), kind: { kind: 'bool-literal', value: node.kind === ts.SyntaxKind.TrueKeyword } });
  }
  if (ts.isNumericLiteral(node)) {
    const spelling = node.getText(sourceFile);
    if (Buffer.byteLength(spelling, 'utf8') <= maxIntegerSpellingBytes && /^(0|[1-9][0-9]*)$/.test(spelling)) {
      return pushExpression(context, { span: nodeSpan(node, sourceFile, file), kind: { kind: 'i32-literal', spelling } });
    }
  } else if (
    ts.isPrefixUnaryExpression(node) &&
    node.operator === ts.SyntaxKind.MinusToken &&
    ts.isNumericLiteral(node.operand) &&
    Buffer.byteLength(node.getText(sourceFile), 'utf8') <= maxIntegerSpellingBytes &&
    /^-[1-9][0-9]*$/.test(node.getText(sourceFile))
  ) {
    return pushExpression(context, {
      span: nodeSpan(node, sourceFile, file),
      kind: { kind: 'i32-literal', spelling: node.getText(sourceFile) },
    });
  } else if (ts.isPrefixUnaryExpression(node) && node.operator === ts.SyntaxKind.MinusToken) {
    const operand = normalizeExpression(node.operand, context, depth + 1);
    if (operand !== null) {
      const operator = node.getFirstToken(sourceFile);
      if (!operator || operator.kind !== ts.SyntaxKind.MinusToken) failInvariant('TypeScript omitted a unary minus token');
      return pushExpression(context, { span: nodeSpan(node, sourceFile, file), kind: { kind: 'negation', operator_span: nodeSpan(operator, sourceFile, file), operand } });
    }
    return null;
  } else if (ts.isBinaryExpression(node) && binaryKinds.has(node.operatorToken.kind)) {
    const lhs = normalizeExpression(node.left, context, depth + 1);
    const rhs = normalizeExpression(node.right, context, depth + 1);
    if (lhs !== null && rhs !== null) {
      return pushExpression(context, { span: nodeSpan(node, sourceFile, file), kind: { kind: binaryKinds.get(node.operatorToken.kind), operator_span: nodeSpan(node.operatorToken, sourceFile, file), lhs, rhs } });
    }
    return null;
  } else if (ts.isCallExpression(node) && ts.isIdentifier(node.expression) && !node.typeArguments?.length && !node.questionDotToken) {
    if (node.arguments.length > maxCallArguments) failBudget('call exceeds the argument limit');
    const callee = normalizedIdentifier(node.expression, sourceFile, file, collector, 'call callee');
    const args = [];
    let valid = callee !== null;
    for (const argument of node.arguments) {
      if (ts.isSpreadElement(argument)) {
        collector.unsupported(argument, sourceFile, file, 'call argument');
        valid = false;
        continue;
      }
      const normalized = normalizeExpression(argument, context, depth + 1);
      if (normalized === null) valid = false;
      else args.push(normalized);
    }
    const open = requiredToken(node, ts.SyntaxKind.OpenParenToken, sourceFile, 'a call open parenthesis');
    const close = requiredToken(node, ts.SyntaxKind.CloseParenToken, sourceFile, 'a call close parenthesis');
    if (valid) return pushExpression(context, { span: nodeSpan(node, sourceFile, file), kind: { kind: 'call', callee, open_paren_span: nodeSpan(open, sourceFile, file), arguments: args, close_paren_span: nodeSpan(close, sourceFile, file) } });
    return null;
  }
  collector.unsupported(node, sourceFile, file, 'expression');
  return null;
}

function requireSemicolon(node, context, label) {
  const token = findToken(node, ts.SyntaxKind.SemicolonToken, context.sourceFile);
  if (!token) {
    context.collector.unsupported(node, context.sourceFile, context.file, `${label} without a semicolon`);
    return null;
  }
  return nodeSpan(token, context.sourceFile, context.file);
}

function pushStatement(context, statement) {
  if (context.statements.length >= maxStatementsPerFunction) failBudget('function exceeds the statement limit');
  context.budgets.statements += 1;
  if (context.budgets.statements > maxStatementsPerProject) failBudget('project exceeds the statement limit');
  const id = context.statements.length;
  context.statements.push(statement);
  return id;
}

function allocateBlock(block, context, depth) {
  const { sourceFile, file, collector } = context;
  if (depth > maxNesting) {
    collector.unsupported(block, sourceFile, file, 'block nesting');
    return null;
  }
  if (context.blocks.length >= maxBlocksPerFunction) failBudget('function exceeds the lexical-block limit');
  context.budgets.blocks += 1;
  if (context.budgets.blocks > maxBlocksPerProject) failBudget('project exceeds the lexical-block limit');
  const open = requiredToken(block, ts.SyntaxKind.OpenBraceToken, sourceFile, 'a block open brace');
  const close = requiredToken(block, ts.SyntaxKind.CloseBraceToken, sourceFile, 'a block close brace');
  const id = context.blocks.length;
  const output = { span: nodeSpan(block, sourceFile, file), open_brace_span: nodeSpan(open, sourceFile, file), statements: [], close_brace_span: nodeSpan(close, sourceFile, file) };
  context.blocks.push(output);
  for (const statement of block.statements) {
    const normalized = normalizeStatement(statement, context, depth);
    if (normalized === null) context.valid = false;
    else output.statements.push(normalized);
  }
  return id;
}

function normalizeStatement(node, context, depth) {
  const { sourceFile, file, collector } = context;
  const placeholder = pushStatement(context, null);
  let kind = null;
  if (ts.isVariableStatement(node)) {
    const declarationList = node.declarationList;
    const mutable = (declarationList.flags & ts.NodeFlags.Let) !== 0;
    const isConst = (declarationList.flags & ts.NodeFlags.Const) !== 0;
    const declaration = declarationList.declarations[0];
    if ((!mutable && !isConst) || declarationList.declarations.length !== 1 || !declaration || !ts.isIdentifier(declaration.name) || !declaration.type || !declaration.initializer) {
      collector.unsupported(node, sourceFile, file, 'local declaration');
    } else if (node.modifiers?.length || declaration.dotDotDotToken || declaration.exclamationToken) {
      collector.unsupported(node, sourceFile, file, 'local declaration');
    } else {
      context.locals += 1;
      context.budgets.locals += 1;
      if (context.locals > maxLocalsPerFunction) failBudget('function exceeds the local limit');
      if (context.budgets.locals > maxLocalsPerProject) failBudget('project exceeds the local limit');
      const keywordKind = mutable ? ts.SyntaxKind.LetKeyword : ts.SyntaxKind.ConstKeyword;
      const keyword = requiredToken(declarationList, keywordKind, sourceFile, 'a local declaration keyword');
      const equals = requiredToken(declaration, ts.SyntaxKind.EqualsToken, sourceFile, 'a local declaration equals token');
      const semicolon = requireSemicolon(node, context, 'local declaration');
      const name = normalizedIdentifier(declaration.name, sourceFile, file, collector, 'local name');
      const typeSyntax = normalizeType(declaration.type, declaration.name.getEnd(), sourceFile, file, collector, 'local annotation');
      const initializer = normalizeExpression(declaration.initializer, context, depth + 1);
      if (semicolon && name && typeSyntax && initializer !== null) kind = { kind: 'local-declaration', keyword_span: nodeSpan(keyword, sourceFile, file), mutable, name, type_syntax: typeSyntax, equals_span: nodeSpan(equals, sourceFile, file), initializer, semicolon_span: semicolon };
    }
  } else if (ts.isExpressionStatement(node) && ts.isBinaryExpression(node.expression) && node.expression.operatorToken.kind === ts.SyntaxKind.EqualsToken && ts.isIdentifier(node.expression.left)) {
    const semicolon = requireSemicolon(node, context, 'assignment');
    const target = normalizedIdentifier(node.expression.left, sourceFile, file, collector, 'assignment target');
    const value = normalizeExpression(node.expression.right, context, depth + 1);
    if (semicolon && target && value !== null) kind = { kind: 'assignment', target, equals_span: nodeSpan(node.expression.operatorToken, sourceFile, file), value, semicolon_span: semicolon };
  } else if (ts.isReturnStatement(node) && node.expression) {
    const semicolon = requireSemicolon(node, context, 'return');
    const value = normalizeExpression(node.expression, context, depth + 1);
    const keyword = requiredToken(node, ts.SyntaxKind.ReturnKeyword, sourceFile, 'a return keyword');
    if (semicolon && value !== null) kind = { kind: 'return', keyword_span: nodeSpan(keyword, sourceFile, file), value, semicolon_span: semicolon };
  } else if (ts.isBlock(node)) {
    const block = allocateBlock(node, context, depth + 1);
    if (block !== null) kind = { kind: 'block', block };
  } else if (ts.isIfStatement(node) && ts.isBlock(node.thenStatement) && (!node.elseStatement || ts.isBlock(node.elseStatement))) {
    const condition = normalizeExpression(node.expression, context, depth + 1);
    const thenBlock = allocateBlock(node.thenStatement, context, depth + 1);
    let elseClause = null;
    if (node.elseStatement) {
      const elseKeyword = node.elseStatement.getFullStart() >= 0 ? findToken(node, ts.SyntaxKind.ElseKeyword, sourceFile) : null;
      if (!elseKeyword) failInvariant('TypeScript omitted an else keyword');
      const block = allocateBlock(node.elseStatement, context, depth + 1);
      if (block !== null) elseClause = { keyword_span: nodeSpan(elseKeyword, sourceFile, file), block };
    }
    const keyword = requiredToken(node, ts.SyntaxKind.IfKeyword, sourceFile, 'an if keyword');
    const open = requiredToken(node, ts.SyntaxKind.OpenParenToken, sourceFile, 'an if open parenthesis');
    const close = requiredToken(node, ts.SyntaxKind.CloseParenToken, sourceFile, 'an if close parenthesis');
    if (condition !== null && thenBlock !== null && (!node.elseStatement || elseClause)) kind = { kind: 'if', keyword_span: nodeSpan(keyword, sourceFile, file), open_paren_span: nodeSpan(open, sourceFile, file), condition, close_paren_span: nodeSpan(close, sourceFile, file), then_block: thenBlock, else_clause: elseClause };
  } else if (ts.isWhileStatement(node) && ts.isBlock(node.statement)) {
    const condition = normalizeExpression(node.expression, context, depth + 1);
    const bodyBlock = allocateBlock(node.statement, context, depth + 1);
    const keyword = requiredToken(node, ts.SyntaxKind.WhileKeyword, sourceFile, 'a while keyword');
    const open = requiredToken(node, ts.SyntaxKind.OpenParenToken, sourceFile, 'a while open parenthesis');
    const close = requiredToken(node, ts.SyntaxKind.CloseParenToken, sourceFile, 'a while close parenthesis');
    if (condition !== null && bodyBlock !== null) kind = { kind: 'while', keyword_span: nodeSpan(keyword, sourceFile, file), open_paren_span: nodeSpan(open, sourceFile, file), condition, close_paren_span: nodeSpan(close, sourceFile, file), body_block: bodyBlock };
  } else {
    collector.unsupported(node, sourceFile, file, 'statement');
  }
  if (kind === null) {
    context.statements[placeholder] = { span: nodeSpan(node, sourceFile, file), kind: { kind: 'invalid' } };
    return null;
  }
  context.statements[placeholder] = { span: nodeSpan(node, sourceFile, file), kind };
  return placeholder;
}

function normalizeImport(node, sourceFile, file, collector, budgets) {
  let valid = true;
  if (!node.importClause || node.importClause.isTypeOnly || node.importClause.name || !node.importClause.namedBindings || !ts.isNamedImports(node.importClause.namedBindings) || node.attributes) {
    collector.unsupported(node, sourceFile, file, 'import declaration');
    return null;
  }
  if (!ts.isStringLiteral(node.moduleSpecifier)) {
    collector.unsupported(node.moduleSpecifier, sourceFile, file, 'module specifier');
    return null;
  }
  const elements = node.importClause.namedBindings.elements;
  if (elements.length === 0) {
    collector.unsupported(node.importClause.namedBindings, sourceFile, file, 'empty named import');
    valid = false;
  }
  if (elements.length > maxBindingsPerImport) failBudget('import exceeds the imported-name limit');
  budgets.bindings += elements.length;
  if (budgets.bindings > maxBindingsPerProject) failBudget('project exceeds the imported-name limit');
  const raw = node.moduleSpecifier.getText(sourceFile);
  const quote = raw[0];
  const value = node.moduleSpecifier.text;
  const components = value.split('/');
  const specifierBody = value.startsWith('./')
    ? value.slice(2)
    : value.replace(/^(?:\.\.\/)+/, '');
  const validSpecifier =
    (value.startsWith('./') || value.startsWith('../')) &&
    value.endsWith('.zry') &&
    /^[\x00-\x7f]*$/.test(value) &&
    !value.includes('\\') &&
    !value.includes('\0') &&
    !value.includes('?') &&
    !value.includes('#') &&
    !value.includes('://') &&
    specifierBody.length > 0 &&
    components.every((component) => component.length > 0);
  if ((quote !== '"' && quote !== "'") || raw.at(-1) !== quote || raw.slice(1, -1) !== value || raw.includes('\\') || Buffer.byteLength(value, 'utf8') > 1024 || !validSpecifier) {
    collector.unsupported(node.moduleSpecifier, sourceFile, file, 'module specifier');
    valid = false;
  }
  const bindings = [];
  for (const element of elements) {
    if (element.isTypeOnly) {
      collector.unsupported(element, sourceFile, file, 'type-only import');
      valid = false;
      continue;
    }
    const importedNode = element.propertyName ?? element.name;
    const imported = normalizedIdentifier(importedNode, sourceFile, file, collector, 'imported name');
    const local = normalizedIdentifier(element.name, sourceFile, file, collector, 'import local name');
    const asToken = element.propertyName ? requiredToken(element, ts.SyntaxKind.AsKeyword, sourceFile, 'an import alias keyword') : null;
    if (!imported || !local) valid = false;
    else bindings.push({ span: nodeSpan(element, sourceFile, file), imported, local, as_span: asToken ? nodeSpan(asToken, sourceFile, file) : null });
  }
  const importToken = requiredToken(node, ts.SyntaxKind.ImportKeyword, sourceFile, 'an import keyword');
  const fromToken = requiredToken(node, ts.SyntaxKind.FromKeyword, sourceFile, 'an import from keyword');
  const semicolon = findToken(node, ts.SyntaxKind.SemicolonToken, sourceFile);
  if (!semicolon) {
    collector.unsupported(node, sourceFile, file, 'import without a semicolon');
    valid = false;
  }
  if (!valid || !semicolon) return null;
  const tokenStart = node.moduleSpecifier.getStart(sourceFile);
  return {
    span: nodeSpan(node, sourceFile, file),
    import_span: nodeSpan(importToken, sourceFile, file),
    bindings,
    from_span: nodeSpan(fromToken, sourceFile, file),
    specifier: {
      text: value,
      token_span: nodeSpan(node.moduleSpecifier, sourceFile, file),
      value_span: spanFromOffsets(sourceFile, file, tokenStart + 1, node.moduleSpecifier.getEnd() - 1),
    },
    semicolon_span: nodeSpan(semicolon, sourceFile, file),
  };
}

function normalizeFunction(node, sourceFile, file, collector, budgets, index) {
  let valid = true;
  let exportSpan = null;
  for (const modifier of node.modifiers ?? []) {
    if (modifier.kind === ts.SyntaxKind.ExportKeyword && exportSpan === null) exportSpan = nodeSpan(modifier, sourceFile, file);
    else {
      collector.unsupported(modifier, sourceFile, file, `function ${index} modifier`);
      valid = false;
    }
  }
  if (node.asteriskToken || !node.name || !ts.isIdentifier(node.name) || node.typeParameters?.length || !node.body) {
    collector.unsupported(node, sourceFile, file, `function ${index}`);
    valid = false;
  }
  const name = node.name && ts.isIdentifier(node.name) ? normalizedIdentifier(node.name, sourceFile, file, collector, `function ${index} name`) : null;
  if (!name) valid = false;
  if (node.parameters.length > maxParametersPerFunction) failBudget('function exceeds the parameter limit');
  budgets.parameters += node.parameters.length;
  if (budgets.parameters > maxParametersPerProject) failBudget('project exceeds the parameter limit');
  const parameters = node.parameters.map((parameter, parameterIndex) => normalizeParameter(parameter, sourceFile, file, collector, parameterIndex));
  if (parameters.some((parameter) => parameter === null)) valid = false;
  const resultType = normalizeType(node.type, node.parameters.end, sourceFile, file, collector, `function ${index} result annotation`);
  if (!resultType) valid = false;
  if (!node.body) return null;
  const context = { sourceFile, file, collector, budgets, blocks: [], statements: [], expressions: [], locals: 0, valid: true };
  const rootBlock = allocateBlock(node.body, context, 1);
  valid &&= context.valid && rootBlock === 0;
  const functionToken = requiredToken(node, ts.SyntaxKind.FunctionKeyword, sourceFile, 'a function keyword');
  if (!valid || !name || !resultType || rootBlock === null) return null;
  return {
    span: nodeSpan(node, sourceFile, file), export_span: exportSpan,
    function_span: nodeSpan(functionToken, sourceFile, file), name,
    parameters, result_type: resultType,
    body: { span: nodeSpan(node.body, sourceFile, file), root_block: rootBlock, blocks: context.blocks, statements: context.statements, expressions: context.expressions },
  };
}

function addParseDiagnostics(sourceFile, file, collector) {
  const diagnostics = [...sourceFile.parseDiagnostics].sort((a, b) => (a.start ?? -1) - (b.start ?? -1) || a.code - b.code);
  if (diagnostics.length > 0) {
    const diagnostic = diagnostics[0];
    const start = diagnostic.start ?? 0;
    const end = Math.min(sourceFile.text.length, start + (diagnostic.length ?? 0));
    const span = spanFromOffsets(sourceFile, file, start, end);
    throw new AdapterError(
      'ZRYNA-F2002',
      `TypeScript parse error TS${diagnostic.code} at file ${file} bytes ${span.start}..${span.end}: ${compactText(ts.flattenDiagnosticMessageText(diagnostic.messageText, ' '))}`,
    );
  }
}

function normalizeSource(input, file, collector, budgets) {
  if (!input.text.isWellFormed()) failRequest(`source '${input.path}' contains an unpaired UTF-16 surrogate`);
  if (input.text.startsWith('#!')) {
    throw new AdapterError('ZRYNA-F2002', `source '${input.path}' uses an unsupported hashbang`);
  }
  const bytes = Buffer.byteLength(input.text, 'utf8');
  if (bytes > maxSourceFileBytes) failBudget('source file exceeds the byte limit');
  budgets.sourceBytes += bytes;
  if (budgets.sourceBytes > maxSourceBytes) failBudget('project exceeds the source byte limit');
  enforceParserNesting(input.text);
  let sourceFile;
  try {
    sourceFile = ts.createSourceFile(input.path, input.text, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
  } catch (error) {
    if (error instanceof RangeError) failBudget('source exceeds TypeScript parser capacity');
    throw error;
  }
  buildUtf8OffsetMap(sourceFile);
  addParseDiagnostics(sourceFile, file, collector);
  const imports = [];
  const functions = [];
  let sawFunction = false;
  let importDeclarations = 0;
  let functionDeclarations = 0;
  for (const node of sourceFile.statements) {
    if (ts.isImportDeclaration(node)) {
      if (sawFunction) {
        collector.unsupported(node, sourceFile, file, 'import after function');
        continue;
      }
      importDeclarations += 1;
      if (importDeclarations > maxImportsPerFile) failBudget('module exceeds the import-declaration limit');
      budgets.imports += 1;
      if (budgets.imports > maxImportsPerProject) failBudget('project exceeds the import-declaration limit');
      const normalized = normalizeImport(node, sourceFile, file, collector, budgets);
      if (normalized) imports.push(normalized);
      continue;
    }
    sawFunction = true;
    if (!ts.isFunctionDeclaration(node)) {
      collector.unsupported(node, sourceFile, file, 'top-level declaration');
      continue;
    }
    functionDeclarations += 1;
    if (functionDeclarations > maxFunctionsPerFile) failBudget('module exceeds the function limit');
    budgets.functions += 1;
    if (budgets.functions > maxFunctionsPerProject) failBudget('project exceeds the function limit');
    const normalized = normalizeFunction(node, sourceFile, file, collector, budgets, functionDeclarations - 1);
    if (normalized) functions.push(normalized);
  }
  return { id: file, path: input.path, imports, functions };
}

function validateAnalyzeParams(params) {
  requireExactKeys(params, ['schema_version', 'files'], 'analyze params');
  if (params.schema_version !== protocolVersion || !Array.isArray(params.files)) failRequest('analyze requires a protocol-v3 file list');
  if (params.files.length > maxFiles) failBudget('request exceeds the source-file limit');
  const identities = new Set();
  const files = params.files.map((input, index) => {
    requireExactKeys(input, ['path', 'text'], `source file ${index}`);
    const path = validatePortablePath(input.path);
    if (typeof input.text !== 'string') failRequest(`source file ${index} text must be a string`);
    const identity = path.toLowerCase();
    if (identities.has(identity)) failRequest('source paths collide under portable identity');
    identities.add(identity);
    return { path, text: input.text };
  });
  return files.sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
}

function handle(request) {
  if (!isRecord(request)) failRequest('request must be an object');
  requireRequestId(request.id);
  if (request.method === 'handshake') {
    requireExactKeys(request, ['id', 'method'], 'handshake request');
    return { id: request.id, result: { provider: 'typescript-6', provider_version: providerVersion, protocol_version: protocolVersion, capabilities: { module_resolution: false, semantic_diagnostics: false, control_flow_v1: true } } };
  }
  if (request.method === 'analyze') {
    requireExactKeys(request, ['id', 'method', 'params'], 'analyze request');
    const files = validateAnalyzeParams(request.params);
    const collector = new DiagnosticCollector();
    const budgets = { sourceBytes: 0, imports: 0, bindings: 0, functions: 0, parameters: 0, blocks: 0, statements: 0, expressions: 0, locals: 0 };
    const normalized = files.map((file, index) => normalizeSource(file, index, collector, budgets));
    return { id: request.id, result: { schema_version: protocolVersion, files: normalized, diagnostics: collector.finish() } };
  }
  failRequest('request method is unsupported');
}

function errorResponse(id, error) {
  return { id: isRequestId(id) ? id : null, error: { code: error instanceof AdapterError ? error.code : 'ZRYNA-F1003', message: compactText(error instanceof Error ? error.message : String(error)) } };
}

async function writeResponse(response) {
  let serialized = JSON.stringify(response);
  if (Buffer.byteLength(serialized, 'utf8') > maxResponseBytes) serialized = JSON.stringify(errorResponse(response?.id, new AdapterError('ZRYNA-F1002', 'response exceeds the byte limit')));
  if (!process.stdout.write(`${serialized}\n`)) await once(process.stdout, 'drain');
}

async function processLine(bytes) {
  let request;
  try {
    let text;
    try { text = new TextDecoder('utf-8', { fatal: true }).decode(bytes); }
    catch { failRequest('request is not valid UTF-8'); }
    if (!text.trim()) return;
    rejectDuplicateObjectKeys(text);
    try { request = JSON.parse(text); }
    catch { failRequest('request is not valid JSON'); }
    await writeResponse(handle(request));
  } catch (error) {
    await writeResponse(errorResponse(request?.id, error));
  }
}

let lineParts = [];
let lineBytes = 0;
let discardingOversizedLine = false;
for await (const chunk of process.stdin) {
  let cursor = 0;
  while (cursor < chunk.length) {
    const newline = chunk.indexOf(0x0a, cursor);
    const end = newline === -1 ? chunk.length : newline;
    const part = chunk.subarray(cursor, end);
    if (!discardingOversizedLine) {
      if (lineBytes + part.length > maxRequestBytes) {
        lineParts = [];
        lineBytes = 0;
        discardingOversizedLine = true;
      } else if (part.length > 0) {
        lineParts.push(part);
        lineBytes += part.length;
      }
    }
    if (newline === -1) break;
    if (discardingOversizedLine) await writeResponse(errorResponse(null, new AdapterError('ZRYNA-F1002', 'request exceeds the byte limit')));
    else await processLine(Buffer.concat(lineParts, lineBytes));
    lineParts = [];
    lineBytes = 0;
    discardingOversizedLine = false;
    cursor = newline + 1;
  }
}
if (discardingOversizedLine) await writeResponse(errorResponse(null, new AdapterError('ZRYNA-F1002', 'request exceeds the byte limit')));
else if (lineBytes > 0) await processLine(Buffer.concat(lineParts, lineBytes));
