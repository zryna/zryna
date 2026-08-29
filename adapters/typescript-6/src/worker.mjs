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

const protocolVersion = 2;
const expectedProviderVersion = '6.0.3';
const providerVersion = ts.version;
if (providerVersion !== expectedProviderVersion) {
  throw new Error(`the TypeScript provider must be exactly ${expectedProviderVersion}`);
}

const maxRequestBytes = boundedTestLimit('REQUEST_BYTES', 72 * 1024 * 1024);
const maxResponseBytes = boundedTestLimit('RESPONSE_BYTES', 16 * 1024 * 1024);
const maxFiles = boundedTestLimit('FILES', 4096);
const maxSourceFileBytes = boundedTestLimit('SOURCE_FILE_BYTES', 2 * 1024 * 1024);
const maxSourceBytes = boundedTestLimit('SOURCE_BYTES', 64 * 1024 * 1024);
const maxLinesPerFile = boundedTestLimit('LINES_PER_FILE', 100_000);
const maxLinesPerProject = boundedTestLimit('LINES_PER_PROJECT', 1_000_000);
const maxSyntaxNodesPerFile = boundedTestLimit('NODES_PER_FILE', 262_144);
const maxSyntaxNodesPerProject = boundedTestLimit('NODES_PER_PROJECT', 1_048_576);
const maxFunctionsPerFile = 4096;
const maxFunctionsPerProject = 16_384;
const maxParametersPerFunction = 256;
const maxParametersPerProject = 262_144;
const maxStatementsPerFunction = 4096;
const maxStatementsPerProject = 65_536;
const maxExpressionsPerFunction = 16_384;
const maxExpressionsPerProject = 262_144;
const maxExpressionDepth = 128;
const maxParserNesting = 512;
const maxDiagnostics = 256;
const maxDiagnosticCharacters = 4096;
const maxNameCharacters = 1024;
const maxIntegerSpellingBytes = 64;
const maxJsonDepth = 8;
const maxJsonContainers = maxFiles + 4;
const maxJsonFields = maxFiles * 2 + 8;
const maxJsonTokens = 50_000;

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
  let tokens = 0;
  for (let index = 0; index < text.length; index += 1) {
    const character = text[index];
    if (!/\s/.test(character)) {
      tokens += 1;
      if (tokens > maxJsonTokens) failBudget('request exceeds the JSON token limit');
    }
    if (character === '{') {
      containers += 1;
      if (stack.length >= maxJsonDepth) failBudget('request exceeds the JSON depth limit');
      if (containers > maxJsonContainers) failBudget('request exceeds the JSON container limit');
      stack.push({ kind: 'object', keys: new Set() });
      continue;
    }
    if (character === '[') {
      containers += 1;
      if (stack.length >= maxJsonDepth) failBudget('request exceeds the JSON depth limit');
      if (containers > maxJsonContainers) failBudget('request exceeds the JSON container limit');
      stack.push({ kind: 'array' });
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
      if (escaped) {
        escaped = false;
      } else if (current === '\\') {
        escaped = true;
      } else if (current === '"') {
        break;
      }
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
      component.length === 0 ||
      component === '.' ||
      component === '..' ||
      component.length > 255 ||
      component.endsWith('.') ||
      component.endsWith(' ') ||
      /[<>:"|?*]/.test(component)
    ) {
      failRequest('source path contains a non-portable component');
    }
    const stem = component.split('.')[0].toLowerCase();
    if (/^(con|prn|aux|nul|com[1-9]|lpt[1-9])$/.test(stem)) {
      failRequest('source path contains a reserved device name');
    }
  }
  return path;
}

const utf8OffsetMaps = new WeakMap();

function buildUtf8OffsetMap(sourceFile) {
  const text = sourceFile.text;
  const offsets = new Uint32Array(text.length + 1);
  let bytes = 0;
  for (let index = 0; index < text.length; index += 1) {
    offsets[index] = bytes;
    const codeUnit = text.charCodeAt(index);
    if (codeUnit <= 0x7f) {
      bytes += 1;
    } else if (codeUnit <= 0x7ff) {
      bytes += 2;
    } else if (codeUnit >= 0xd800 && codeUnit <= 0xdbff) {
      offsets[index + 1] = 0xffff_ffff;
      index += 1;
      bytes += 4;
    } else {
      bytes += 3;
    }
  }
  offsets[text.length] = bytes;
  utf8OffsetMaps.set(sourceFile, offsets);
}

function utf8ByteOffset(sourceFile, utf16Offset) {
  const text = sourceFile.text;
  if (!Number.isInteger(utf16Offset) || utf16Offset < 0 || utf16Offset > text.length) {
    throw new AdapterError(
      'ZRYNA-F1003',
      'TypeScript returned an out-of-range UTF-16 source offset',
    );
  }
  const offsets = utf8OffsetMaps.get(sourceFile);
  if (!offsets) {
    throw new AdapterError('ZRYNA-F1003', 'UTF-8 source offsets were not initialized');
  }
  if (offsets[utf16Offset] === 0xffff_ffff) {
    throw new AdapterError(
      'ZRYNA-F1003',
      'TypeScript returned a UTF-16 offset inside a surrogate pair',
    );
  }
  return offsets[utf16Offset];
}

function spanFromOffsets(sourceFile, fileId, start, end) {
  return {
    file: fileId,
    start: utf8ByteOffset(sourceFile, start),
    end: utf8ByteOffset(sourceFile, end),
  };
}

function nodeSpan(node, sourceFile, fileId) {
  return spanFromOffsets(sourceFile, fileId, node.getStart(sourceFile), node.getEnd());
}

function normalizedIdentifier(node, sourceFile, fileId, collector, context) {
  const spelling = node.getText(sourceFile);
  if (spelling !== node.text || [...spelling].length > maxNameCharacters) {
    collector.unsupported(node, sourceFile, fileId, context);
    return null;
  }
  return { text: spelling, span: nodeSpan(node, sourceFile, fileId) };
}

function compactText(value) {
  const text = String(value).replaceAll(/\s+/g, ' ').trim();
  return [...text].slice(0, maxDiagnosticCharacters).join('') || 'provider diagnostic';
}

function compareDiagnostics(left, right) {
  const leftSpan = left.location.kind === 'source' ? left.location.span : null;
  const rightSpan = right.location.kind === 'source' ? right.location.span : null;
  const leftKey = [
    leftSpan?.file ?? -1,
    leftSpan?.start ?? -1,
    leftSpan?.end ?? -1,
    left.code,
    left.message,
    left.guidance,
  ];
  const rightKey = [
    rightSpan?.file ?? -1,
    rightSpan?.start ?? -1,
    rightSpan?.end ?? -1,
    right.code,
    right.message,
    right.guidance,
  ];
  for (let index = 0; index < leftKey.length; index += 1) {
    if (leftKey[index] < rightKey[index]) return -1;
    if (leftKey[index] > rightKey[index]) return 1;
  }
  return 0;
}

class DiagnosticCollector {
  #diagnostics = [];
  #truncated = false;

  add(diagnostic) {
    const retainedLimit = maxDiagnostics - 1;
    if (this.#diagnostics.length < retainedLimit) {
      this.#diagnostics.push(diagnostic);
      return;
    }
    this.#truncated = true;
    let worstIndex = 0;
    for (let index = 1; index < this.#diagnostics.length; index += 1) {
      if (compareDiagnostics(this.#diagnostics[worstIndex], this.#diagnostics[index]) < 0) {
        worstIndex = index;
      }
    }
    if (compareDiagnostics(diagnostic, this.#diagnostics[worstIndex]) < 0) {
      this.#diagnostics[worstIndex] = diagnostic;
    }
  }

  located(code, sourceFile, fileId, start, end, message, guidance) {
    this.add({
      code,
      severity: 'error',
      location: { kind: 'source', span: spanFromOffsets(sourceFile, fileId, start, end) },
      message: compactText(message),
      guidance: compactText(guidance),
    });
  }

  unsupported(node, sourceFile, fileId, context) {
    const kind = ts.SyntaxKind[node.kind] ?? `kind ${node.kind}`;
    this.located(
      'ZRYNA-F2002',
      sourceFile,
      fileId,
      node.getStart(sourceFile),
      node.getEnd(),
      `${context} uses unsupported syntax '${kind}'`,
      'use only the documented protocol-v2 bootstrap syntax',
    );
  }

  finish() {
    this.#diagnostics.sort(compareDiagnostics);
    if (this.#truncated) {
      this.#diagnostics.push({
        code: 'ZRYNA-F2003',
        severity: 'error',
        location: { kind: 'global' },
        message: 'frontend diagnostics exceeded the deterministic limit',
        guidance: 'reduce unsupported or malformed source before analysis',
      });
    }
    return this.#diagnostics;
  }
}

function syntacticDiagnostics(sourceFile) {
  const options = {
    target: ts.ScriptTarget.Latest,
    noLib: true,
    noResolve: true,
    noEmit: true,
  };
  const host = {
    fileExists: (fileName) => fileName === sourceFile.fileName,
    readFile: (fileName) => (fileName === sourceFile.fileName ? sourceFile.text : undefined),
    getSourceFile: (fileName) => (fileName === sourceFile.fileName ? sourceFile : undefined),
    getDefaultLibFileName: () => 'lib.d.ts',
    writeFile: () => {},
    getCurrentDirectory: () => '',
    getDirectories: () => [],
    getCanonicalFileName: (fileName) => fileName,
    useCaseSensitiveFileNames: () => true,
    getNewLine: () => '\n',
  };
  return withTypeScriptCapacityGuard(() => {
    const program = ts.createProgram({ rootNames: [sourceFile.fileName], options, host });
    return program.getSyntacticDiagnostics(sourceFile);
  });
}

function addParseDiagnostics(sourceFile, fileId, collector) {
  const diagnostics = [...syntacticDiagnostics(sourceFile)].sort((left, right) => {
    const leftStart = left.start ?? -1;
    const rightStart = right.start ?? -1;
    return leftStart - rightStart || left.code - right.code;
  });
  for (const diagnostic of diagnostics) {
    const message = ts.flattenDiagnosticMessageText(diagnostic.messageText, ' ');
    if (Number.isInteger(diagnostic.start)) {
      const start = diagnostic.start;
      const end = Math.min(sourceFile.text.length, start + (diagnostic.length ?? 0));
      collector.located(
        `TS${diagnostic.code}`,
        sourceFile,
        fileId,
        start,
        end,
        message,
        'fix the TypeScript parse error before Zryna analysis',
      );
    } else {
      collector.add({
        code: `TS${diagnostic.code}`,
        severity: 'error',
        location: { kind: 'global' },
        message: compactText(message),
        guidance: 'fix the TypeScript parse error before Zryna analysis',
      });
    }
  }
  return diagnostics.length > 0;
}

function countSyntaxNodes(sourceFile, budgets) {
  const stack = [sourceFile];
  let count = 0;
  while (stack.length > 0) {
    const node = stack.pop();
    count += 1;
    if (count > maxSyntaxNodesPerFile) failBudget('source file exceeds the syntax-node limit');
    ts.forEachChild(node, (child) => {
      stack.push(child);
    });
  }
  budgets.syntaxNodes += count;
  if (budgets.syntaxNodes > maxSyntaxNodesPerProject) {
    failBudget('project exceeds the syntax-node limit');
  }
}

function enforceParserNesting(text) {
  const scanner = ts.createScanner(
    ts.ScriptTarget.Latest,
    true,
    ts.LanguageVariant.Standard,
    text,
  );
  let depth = 0;
  for (let token = scanner.scan(); token !== ts.SyntaxKind.EndOfFileToken; token = scanner.scan()) {
    if (
      token === ts.SyntaxKind.OpenBraceToken ||
      token === ts.SyntaxKind.OpenBracketToken ||
      token === ts.SyntaxKind.OpenParenToken
    ) {
      depth += 1;
      if (depth > maxParserNesting) failBudget('source exceeds the parser nesting limit');
    } else if (
      token === ts.SyntaxKind.CloseBraceToken ||
      token === ts.SyntaxKind.CloseBracketToken ||
      token === ts.SyntaxKind.CloseParenToken
    ) {
      depth = Math.max(0, depth - 1);
    }
  }
}

function withTypeScriptCapacityGuard(operation) {
  try {
    return operation();
  } catch (error) {
    if (error instanceof RangeError) failBudget('source exceeds TypeScript parser capacity');
    throw error;
  }
}

function findToken(node, kind, sourceFile) {
  return node.getChildren(sourceFile).find((child) => child.kind === kind) ?? null;
}

function normalizeType(node, insertionOffset, sourceFile, fileId, collector, context) {
  if (!node) {
    return {
      span: spanFromOffsets(sourceFile, fileId, insertionOffset, insertionOffset),
      kind: { kind: 'missing' },
    };
  }
  let name = null;
  if (node.kind === ts.SyntaxKind.AnyKeyword) {
    name = 'any';
  } else if (
    ts.isTypeReferenceNode(node) &&
    ts.isIdentifier(node.typeName) &&
    (!node.typeArguments || node.typeArguments.length === 0)
  ) {
    const identifier = normalizedIdentifier(
      node.typeName,
      sourceFile,
      fileId,
      collector,
      context,
    );
    name = identifier?.text ?? null;
  }
  if (name === null || [...name].length > maxNameCharacters) {
    collector.unsupported(node, sourceFile, fileId, context);
    return null;
  }
  return {
    span: nodeSpan(node, sourceFile, fileId),
    kind: { kind: 'named', name },
  };
}

function normalizeParameter(parameter, sourceFile, fileId, collector, index) {
  let valid = true;
  let name = null;
  if (!ts.isIdentifier(parameter.name)) {
    collector.unsupported(parameter.name, sourceFile, fileId, `parameter ${index} name`);
    valid = false;
  } else {
    name = normalizedIdentifier(
      parameter.name,
      sourceFile,
      fileId,
      collector,
      `parameter ${index} name`,
    );
    if (parameter.name.text === 'this') {
      collector.unsupported(parameter.name, sourceFile, fileId, `parameter ${index} name`);
      name = null;
    }
    if (!name) valid = false;
  }
  for (const token of [
    parameter.dotDotDotToken,
    parameter.questionToken,
    parameter.initializer,
    ...(parameter.modifiers ?? []),
  ]) {
    if (token) {
      collector.unsupported(token, sourceFile, fileId, `parameter ${index}`);
      valid = false;
    }
  }
  const typeSyntax = normalizeType(
    parameter.type,
    parameter.name.getEnd(),
    sourceFile,
    fileId,
    collector,
    `parameter ${index} annotation`,
  );
  if (!typeSyntax) valid = false;
  if (!valid) return null;
  return {
    span: nodeSpan(parameter, sourceFile, fileId),
    name,
    type_syntax: typeSyntax,
  };
}

function pushExpression(expressions, expression, budgets) {
  if (expressions.length >= maxExpressionsPerFunction) {
    failBudget('function exceeds the expression limit');
  }
  budgets.expressions += 1;
  if (budgets.expressions > maxExpressionsPerProject) {
    failBudget('project exceeds the expression limit');
  }
  const id = expressions.length;
  expressions.push(expression);
  return id;
}

function normalizeExpression(root, sourceFile, fileId, collector, expressions, budgets) {
  const frames = [{ node: root, exiting: false, depth: 1 }];
  const values = [];
  while (frames.length > 0) {
    const frame = frames.pop();
    const { node, depth } = frame;
    if (depth > maxExpressionDepth) {
      collector.unsupported(node, sourceFile, fileId, 'expression depth');
      values.push(null);
      continue;
    }
    if (frame.exiting) {
      const rhs = values.pop();
      const lhs = values.pop();
      if (lhs === null || rhs === null || lhs === undefined || rhs === undefined) {
        values.push(null);
        continue;
      }
      values.push(
        pushExpression(
          expressions,
          {
            span: nodeSpan(node, sourceFile, fileId),
            kind: {
              kind: 'addition',
              operator_span: nodeSpan(node.operatorToken, sourceFile, fileId),
              lhs,
              rhs,
            },
          },
          budgets,
        ),
      );
      continue;
    }
    if (ts.isIdentifier(node)) {
      const name = normalizedIdentifier(node, sourceFile, fileId, collector, 'reference');
      if (!name) {
        values.push(null);
      } else {
        values.push(
          pushExpression(
            expressions,
            {
              span: nodeSpan(node, sourceFile, fileId),
              kind: { kind: 'reference', name },
            },
            budgets,
          ),
        );
      }
    } else if (node.kind === ts.SyntaxKind.TrueKeyword || node.kind === ts.SyntaxKind.FalseKeyword) {
      values.push(
        pushExpression(
          expressions,
          {
            span: nodeSpan(node, sourceFile, fileId),
            kind: { kind: 'bool-literal', value: node.kind === ts.SyntaxKind.TrueKeyword },
          },
          budgets,
        ),
      );
    } else if (ts.isNumericLiteral(node)) {
      const spelling = node.getText(sourceFile);
      if (
        Buffer.byteLength(spelling, 'utf8') > maxIntegerSpellingBytes ||
        !/^(0|[1-9][0-9]*)$/.test(spelling)
      ) {
        collector.unsupported(node, sourceFile, fileId, 'integer literal');
        values.push(null);
      } else {
        values.push(
          pushExpression(
            expressions,
            {
              span: nodeSpan(node, sourceFile, fileId),
              kind: { kind: 'i32-literal', spelling },
            },
            budgets,
          ),
        );
      }
    } else if (
      ts.isPrefixUnaryExpression(node) &&
      node.operator === ts.SyntaxKind.MinusToken &&
      ts.isNumericLiteral(node.operand) &&
      Buffer.byteLength(node.getText(sourceFile), 'utf8') <= maxIntegerSpellingBytes &&
      /^-[1-9][0-9]*$/.test(node.getText(sourceFile))
    ) {
      values.push(
        pushExpression(
          expressions,
          {
            span: nodeSpan(node, sourceFile, fileId),
            kind: { kind: 'i32-literal', spelling: node.getText(sourceFile) },
          },
          budgets,
        ),
      );
    } else if (
      ts.isBinaryExpression(node) &&
      node.operatorToken.kind === ts.SyntaxKind.PlusToken
    ) {
      frames.push({ node, exiting: true, depth });
      frames.push({ node: node.right, exiting: false, depth: depth + 1 });
      frames.push({ node: node.left, exiting: false, depth: depth + 1 });
    } else {
      collector.unsupported(node, sourceFile, fileId, 'expression');
      values.push(null);
    }
  }
  return values.length === 1 ? values[0] : null;
}

function normalizeStatement(statement, sourceFile, fileId, collector, expressions, budgets) {
  if (!ts.isReturnStatement(statement) || !statement.expression) {
    collector.unsupported(statement, sourceFile, fileId, 'statement');
    return null;
  }
  const keyword = findToken(statement, ts.SyntaxKind.ReturnKeyword, sourceFile);
  if (!keyword) {
    throw new AdapterError('ZRYNA-F1003', 'TypeScript omitted a return keyword token');
  }
  const expressionCheckpoint = expressions.length;
  const projectExpressionCheckpoint = budgets.expressions;
  const value = normalizeExpression(
    statement.expression,
    sourceFile,
    fileId,
    collector,
    expressions,
    budgets,
  );
  if (value === null) {
    expressions.length = expressionCheckpoint;
    budgets.expressions = projectExpressionCheckpoint;
    return null;
  }
  return {
    span: nodeSpan(statement, sourceFile, fileId),
    kind: {
      kind: 'return',
      keyword_span: nodeSpan(keyword, sourceFile, fileId),
      value,
    },
  };
}

function normalizeFunction(node, sourceFile, fileId, collector, budgets, index) {
  let valid = true;
  let name = null;
  const modifiers = [...(node.modifiers ?? [])];
  const exportModifier = modifiers.find((modifier) => modifier.kind === ts.SyntaxKind.ExportKeyword);
  if (!exportModifier) {
    collector.unsupported(node, sourceFile, fileId, `top-level function ${index}`);
    valid = false;
  }
  for (const modifier of modifiers) {
    if (modifier.kind !== ts.SyntaxKind.ExportKeyword) {
      collector.unsupported(modifier, sourceFile, fileId, `function ${index} modifier`);
      valid = false;
    }
  }
  if (node.asteriskToken) {
    collector.unsupported(node.asteriskToken, sourceFile, fileId, `function ${index} generator`);
    valid = false;
  }
  if (!node.name || !ts.isIdentifier(node.name)) {
    collector.unsupported(node, sourceFile, fileId, `function ${index} name`);
    valid = false;
  } else {
    name = normalizedIdentifier(
      node.name,
      sourceFile,
      fileId,
      collector,
      `function ${index} name`,
    );
    if (!name) valid = false;
  }
  if (node.typeParameters && node.typeParameters.length > 0) {
    for (const typeParameter of node.typeParameters) {
      collector.unsupported(typeParameter, sourceFile, fileId, `function ${index} type parameter`);
    }
    valid = false;
  }
  if (!node.body) {
    collector.unsupported(node, sourceFile, fileId, `function ${index} body`);
    valid = false;
  }
  if (node.parameters.length > maxParametersPerFunction) {
    failBudget('function exceeds the parameter limit');
  }
  budgets.parameters += node.parameters.length;
  if (budgets.parameters > maxParametersPerProject) {
    failBudget('project exceeds the parameter limit');
  }
  const parameters = node.parameters.map((parameter, parameterIndex) => {
    const normalized = normalizeParameter(
      parameter,
      sourceFile,
      fileId,
      collector,
      parameterIndex,
    );
    if (!normalized) valid = false;
    return normalized;
  });
  const resultType = normalizeType(
    node.type,
    node.parameters.end,
    sourceFile,
    fileId,
    collector,
    `function ${index} result annotation`,
  );
  if (!resultType) valid = false;
  const expressions = [];
  const statements = [];
  if (node.body) {
    if (node.body.statements.length > maxStatementsPerFunction) {
      failBudget('function exceeds the statement limit');
    }
    budgets.statements += node.body.statements.length;
    if (budgets.statements > maxStatementsPerProject) {
      failBudget('project exceeds the statement limit');
    }
    for (const statement of node.body.statements) {
      const normalized = normalizeStatement(
        statement,
        sourceFile,
        fileId,
        collector,
        expressions,
        budgets,
      );
      if (normalized) statements.push(normalized);
      else valid = false;
    }
  }
  const functionKeyword = findToken(node, ts.SyntaxKind.FunctionKeyword, sourceFile);
  if (!functionKeyword) {
    throw new AdapterError('ZRYNA-F1003', 'TypeScript omitted a function keyword token');
  }
  if (!valid || !exportModifier || !name || !resultType || !node.body) return null;
  return {
    span: nodeSpan(node, sourceFile, fileId),
    export_span: nodeSpan(exportModifier, sourceFile, fileId),
    function_span: nodeSpan(functionKeyword, sourceFile, fileId),
    name,
    parameters,
    result_type: resultType,
    body: {
      span: nodeSpan(node.body, sourceFile, fileId),
      statements,
      expressions,
    },
  };
}

function normalizeSource(input, fileId, collector, budgets) {
  if (!input.text.isWellFormed()) {
    failRequest(`source '${input.path}' contains an unpaired UTF-16 surrogate`);
  }
  const sourceBytes = Buffer.byteLength(input.text, 'utf8');
  if (sourceBytes > maxSourceFileBytes) failBudget('source file exceeds the byte limit');
  budgets.sourceBytes += sourceBytes;
  if (budgets.sourceBytes > maxSourceBytes) failBudget('project exceeds the source byte limit');
  let lines = 1;
  for (let index = 0; index < input.text.length; index += 1) {
    const character = input.text[index];
    if (character === '\r') {
      lines += 1;
      if (input.text[index + 1] === '\n') index += 1;
    } else if (character === '\n' || character === '\u2028' || character === '\u2029') {
      lines += 1;
    }
    if (lines > maxLinesPerFile) failBudget('source file exceeds the line limit');
  }
  budgets.lines += lines;
  if (budgets.lines > maxLinesPerProject) failBudget('project exceeds the line limit');
  enforceParserNesting(input.text);
  const sourceFile = withTypeScriptCapacityGuard(() =>
    ts.createSourceFile(
      input.path,
      input.text,
      ts.ScriptTarget.Latest,
      true,
      ts.ScriptKind.TS,
    ),
  );
  buildUtf8OffsetMap(sourceFile);
  countSyntaxNodes(sourceFile, budgets);
  if (addParseDiagnostics(sourceFile, fileId, collector)) {
    return { id: fileId, path: input.path, functions: [] };
  }
  const functions = [];
  let functionIndex = 0;
  for (const node of sourceFile.statements) {
    if (!ts.isFunctionDeclaration(node)) {
      collector.unsupported(node, sourceFile, fileId, 'top-level declaration');
      continue;
    }
    functionIndex += 1;
    if (functionIndex > maxFunctionsPerFile) failBudget('source file exceeds the function limit');
    budgets.functions += 1;
    if (budgets.functions > maxFunctionsPerProject) {
      failBudget('project exceeds the function limit');
    }
    const normalized = normalizeFunction(
      node,
      sourceFile,
      fileId,
      collector,
      budgets,
      functionIndex - 1,
    );
    if (normalized) functions.push(normalized);
  }
  return { id: fileId, path: input.path, functions };
}

function validateAnalyzeParams(params) {
  requireExactKeys(params, ['schema_version', 'files'], 'analyze params');
  if (params.schema_version !== protocolVersion || !Array.isArray(params.files)) {
    failRequest('analyze requires a protocol-v2 file list');
  }
  if (params.files.length > maxFiles) failBudget('request exceeds the source-file limit');
  const identities = new Set();
  const files = params.files.map((file, index) => {
    requireExactKeys(file, ['path', 'text'], `source file ${index}`);
    const path = validatePortablePath(file.path);
    if (typeof file.text !== 'string') failRequest(`source file ${index} text must be a string`);
    const identity = path.toLowerCase();
    if (identities.has(identity)) failRequest('source paths collide under portable identity');
    identities.add(identity);
    return { path, text: file.text };
  });
  files.sort((left, right) => {
    if (left.path < right.path) return -1;
    if (left.path > right.path) return 1;
    return 0;
  });
  return files;
}

function handle(request) {
  if (!isRecord(request)) failRequest('request must be an object');
  requireRequestId(request.id);
  if (request.method === 'handshake') {
    requireExactKeys(request, ['id', 'method'], 'handshake request');
    return {
      id: request.id,
      result: {
        provider: 'typescript-6',
        provider_version: providerVersion,
        protocol_version: protocolVersion,
        capabilities: {
          module_resolution: false,
          semantic_diagnostics: false,
        },
      },
    };
  }
  if (request.method === 'analyze') {
    requireExactKeys(request, ['id', 'method', 'params'], 'analyze request');
    const files = validateAnalyzeParams(request.params);
    const collector = new DiagnosticCollector();
    const budgets = {
      sourceBytes: 0,
      lines: 0,
      syntaxNodes: 0,
      functions: 0,
      parameters: 0,
      statements: 0,
      expressions: 0,
    };
    const normalized = files.map((file, index) =>
      normalizeSource(file, index, collector, budgets),
    );
    return {
      id: request.id,
      result: {
        schema_version: protocolVersion,
        files: normalized,
        diagnostics: collector.finish(),
      },
    };
  }
  failRequest('request method is unsupported');
}

function errorResponse(id, error) {
  return {
    id: isRequestId(id) ? id : null,
    error: {
      code: error instanceof AdapterError ? error.code : 'ZRYNA-F1003',
      message: compactText(error instanceof Error ? error.message : String(error)),
    },
  };
}

async function writeResponse(response) {
  let serialized = JSON.stringify(response);
  if (Buffer.byteLength(serialized, 'utf8') > maxResponseBytes) {
    serialized = JSON.stringify(
      errorResponse(response?.id, new AdapterError('ZRYNA-F1002', 'response exceeds the byte limit')),
    );
  }
  if (!process.stdout.write(`${serialized}\n`)) await once(process.stdout, 'drain');
}

async function processLine(bytes) {
  let request;
  try {
    let text;
    try {
      text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    } catch {
      failRequest('request is not valid UTF-8');
    }
    if (!text.trim()) return;
    rejectDuplicateObjectKeys(text);
    try {
      request = JSON.parse(text);
    } catch {
      failRequest('request is not valid JSON');
    }
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
    if (discardingOversizedLine) {
      await writeResponse(
        errorResponse(null, new AdapterError('ZRYNA-F1002', 'request exceeds byte limit')),
      );
    } else {
      await processLine(Buffer.concat(lineParts, lineBytes));
    }
    lineParts = [];
    lineBytes = 0;
    discardingOversizedLine = false;
    cursor = newline + 1;
  }
}
if (discardingOversizedLine) {
  await writeResponse(
    errorResponse(null, new AdapterError('ZRYNA-F1002', 'request exceeds byte limit')),
  );
} else if (lineBytes > 0) {
  await processLine(Buffer.concat(lineParts, lineBytes));
}
