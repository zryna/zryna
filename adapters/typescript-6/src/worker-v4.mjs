import process from 'node:process';
import { once } from 'node:events';
import { TextDecoder } from 'node:util';

import ts from '@typescript/typescript6';

import { PROTOCOL_V4_LIMITS } from './limits-v4.mjs';

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

const protocolVersion = 4;
const expectedProviderVersion = '6.0.3';
const providerVersion = ts.version;
if (providerVersion !== expectedProviderVersion) {
  throw new Error(`the TypeScript provider must be exactly ${expectedProviderVersion}`);
}

const maxRequestBytes = boundedTestLimit('REQUEST_BYTES', PROTOCOL_V4_LIMITS.requestBytes);
const maxResponseBytes = boundedTestLimit('RESPONSE_BYTES', PROTOCOL_V4_LIMITS.responseBytes);
const maxFiles = boundedTestLimit('FILES', PROTOCOL_V4_LIMITS.files);
const maxSourceFileBytes = boundedTestLimit('SOURCE_FILE_BYTES', PROTOCOL_V4_LIMITS.sourceFileBytes);
const maxSourceBytes = boundedTestLimit('SOURCE_BYTES', PROTOCOL_V4_LIMITS.sourceBytes);
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
const maxNominalDeclarationsPerModule = boundedTestLimit(
  'NOMINAL_DECLARATIONS_PER_MODULE',
  boundedTestLimit('NOMINAL_DECLARATIONS', PROTOCOL_V4_LIMITS.nominalDeclarationsPerModule),
);
const maxNominalDeclarationsPerProject = boundedTestLimit('NOMINAL_DECLARATIONS_PER_PROJECT', PROTOCOL_V4_LIMITS.nominalDeclarationsPerProject);
const maxMembersPerDeclaration = boundedTestLimit('MEMBERS_PER_DECLARATION', PROTOCOL_V4_LIMITS.membersPerDeclaration);
const maxMembersPerProject = boundedTestLimit('MEMBERS_PER_PROJECT', PROTOCOL_V4_LIMITS.membersPerProject);
const maxTypeSyntaxNodesPerModule = boundedTestLimit(
  'TYPE_SYNTAX_NODES_PER_MODULE',
  boundedTestLimit('TYPE_SYNTAX_NODES', PROTOCOL_V4_LIMITS.typeSyntaxNodesPerModule),
);
const maxTypeSyntaxNodesPerProject = boundedTestLimit('TYPE_SYNTAX_NODES_PER_PROJECT', PROTOCOL_V4_LIMITS.typeSyntaxNodesPerProject);
const maxTypeSyntaxNesting = boundedTestLimit('TYPE_SYNTAX_NESTING', PROTOCOL_V4_LIMITS.typeSyntaxNesting);
const maxObjectInitializersPerConstruction = boundedTestLimit('OBJECT_INITIALIZERS_PER_CONSTRUCTION', PROTOCOL_V4_LIMITS.objectInitializersPerConstruction);
const maxArrayElementsPerConstruction = boundedTestLimit('ARRAY_ELEMENTS_PER_CONSTRUCTION', PROTOCOL_V4_LIMITS.arrayElementsPerConstruction);
const maxConstructionOperandsPerProject = boundedTestLimit(
  'CONSTRUCTION_OPERANDS_PER_PROJECT',
  boundedTestLimit('AGGREGATE_OPERANDS', PROTOCOL_V4_LIMITS.constructionOperandsPerProject),
);
const maxMatchArmsPerExpression = boundedTestLimit('MATCH_ARMS_PER_EXPRESSION', PROTOCOL_V4_LIMITS.matchArmsPerExpression);
const maxMatchArmsPerProject = boundedTestLimit('MATCH_ARMS_PER_PROJECT', PROTOCOL_V4_LIMITS.matchArmsPerProject);
const maxFixedArrayLength = PROTOCOL_V4_LIMITS.fixedArrayLength;
const maxFixedArrayLengthSpellingBytes = PROTOCOL_V4_LIMITS.fixedArrayLengthSpellingBytes;
const maxDiagnostics = PROTOCOL_V4_LIMITS.diagnostics;
const maxDiagnosticCharacters = 4096;
const maxIdentifierBytes = 128;
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
  if (
    spelling !== node.text || !/^[A-Za-z_][A-Za-z0-9_]*$/.test(spelling) ||
    Buffer.byteLength(spelling, 'utf8') > maxIdentifierBytes || defensiveNames.has(spelling)
  ) {
    collector.unsupported(node, sourceFile, file, context);
    return null;
  }
  return { text: spelling, span: nodeSpan(node, sourceFile, file) };
}

function findToken(node, kind, sourceFile) {
  const children = node.getChildren(sourceFile);
  const direct = children.find((child) => child.kind === kind);
  if (direct) return direct;
  for (const child of children) {
    const nested = findToken(child, kind, sourceFile);
    if (nested) return nested;
  }
  return null;
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

function pushType(typeSyntax, budgets, value) {
  if (typeSyntax.length >= maxTypeSyntaxNodesPerModule) failBudget('module exceeds the type-syntax limit');
  budgets.types += 1;
  if (budgets.types > maxTypeSyntaxNodesPerProject) failBudget('project exceeds the type-syntax limit');
  const id = typeSyntax.length;
  typeSyntax.push(value);
  return id;
}

function normalizeType(node, insertionOffset, sourceFile, file, collector, context, typeSyntax, budgets, depth = 1) {
  if (depth > maxTypeSyntaxNesting) failBudget('type syntax exceeds the nesting limit');
  if (!node) {
    return pushType(typeSyntax, budgets, {
      span: spanFromOffsets(sourceFile, file, insertionOffset, insertionOffset),
      kind: { kind: 'missing' },
    });
  }
  if (!ts.isTypeReferenceNode(node) || !ts.isIdentifier(node.typeName)) {
    collector.unsupported(node, sourceFile, file, context);
    return null;
  }
  const name = dataName(node.typeName, sourceFile, file, collector, context);
  if (!name) return null;
  const args = [...(node.typeArguments ?? [])];
  if (args.length === 0) {
    const kind = name.text === 'String'
      ? { kind: 'string', keyword_span: name.span }
      : { kind: 'named', name };
    return pushType(typeSyntax, budgets, { span: nodeSpan(node, sourceFile, file), kind });
  }
  const unary = new Map([
    ['Vec', 'vec'], ['Shared', 'shared'], ['Weak', 'weak'],
    ['Borrow', 'borrow'], ['BorrowMut', 'borrow-mut'],
  ]);
  const container = unary.get(name.text);
  if (container && args.length === 1) {
    const argument = normalizeType(args[0], args[0].getStart(sourceFile), sourceFile, file, collector, context, typeSyntax, budgets, depth + 1);
    const less = requiredToken(node, ts.SyntaxKind.LessThanToken, sourceFile, 'a type-argument open token');
    const greater = requiredToken(node, ts.SyntaxKind.GreaterThanToken, sourceFile, 'a type-argument close token');
    if (argument === null) return null;
    return pushType(typeSyntax, budgets, {
      span: nodeSpan(node, sourceFile, file),
      kind: {
        kind: container, keyword_span: name.span, less_than_span: nodeSpan(less, sourceFile, file),
        argument, greater_than_span: nodeSpan(greater, sourceFile, file),
      },
    });
  }
  if (name.text === 'FixedArray' && args.length === 2) {
    const element = normalizeType(args[0], args[0].getStart(sourceFile), sourceFile, file, collector, context, typeSyntax, budgets, depth + 1);
    const lengthNode = args[1];
    if (!ts.isLiteralTypeNode(lengthNode) || !ts.isNumericLiteral(lengthNode.literal)) {
      collector.unsupported(lengthNode, sourceFile, file, 'fixed-array length');
      return null;
    }
    const spelling = lengthNode.literal.getText(sourceFile);
    if (
      !/^(0|[1-9][0-9]*)$/.test(spelling) ||
      Buffer.byteLength(spelling, 'utf8') > maxFixedArrayLengthSpellingBytes ||
      BigInt(spelling) > BigInt(maxFixedArrayLength)
    ) {
      failBudget(`fixed-array length must be canonical and at most ${maxFixedArrayLength}`);
    }
    const less = requiredToken(node, ts.SyntaxKind.LessThanToken, sourceFile, 'a fixed-array type open token');
    const greater = requiredToken(node, ts.SyntaxKind.GreaterThanToken, sourceFile, 'a fixed-array type close token');
    const comma = requiredToken(node, ts.SyntaxKind.CommaToken, sourceFile, 'a fixed-array type comma');
    if (element === null) return null;
    return pushType(typeSyntax, budgets, {
      span: nodeSpan(node, sourceFile, file),
      kind: {
        kind: 'fixed-array', keyword_span: name.span, less_than_span: nodeSpan(less, sourceFile, file),
        element, comma_span: nodeSpan(comma, sourceFile, file),
        length_span: nodeSpan(lengthNode.literal, sourceFile, file),
        length: Number(spelling), length_spelling: spelling,
        greater_than_span: nodeSpan(greater, sourceFile, file),
      },
    });
  }
  collector.unsupported(node, sourceFile, file, context);
  return null;
}

function normalizeParameter(parameter, sourceFile, file, collector, index, typeSyntax, budgets) {
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
  const typeId = normalizeType(parameter.type, parameter.name.getEnd(), sourceFile, file, collector, `parameter ${index} annotation`, typeSyntax, budgets);
  valid &&= typeId !== null;
  return valid ? { span: nodeSpan(parameter, sourceFile, file), name, type_syntax: typeId } : null;
}

function pushExpression(context, expression) {
  if (context.expressions.length >= maxExpressionsPerFunction) failBudget('function exceeds the expression limit');
  context.budgets.expressions += 1;
  if (context.budgets.expressions > maxExpressionsPerProject) failBudget('project exceeds the expression limit');
  const id = context.expressions.length;
  context.expressions.push(expression);
  return id;
}

function countAggregateOperands(context, count) {
  context.budgets.aggregateOperands += count;
  if (context.budgets.aggregateOperands > maxConstructionOperandsPerProject) {
    failBudget('project exceeds the aggregate-construction operand limit');
  }
}

function normalizeConstructionFields(object, context, depth) {
  const { sourceFile, file, collector } = context;
  const fields = [];
  const seen = new Set();
  if (object.properties.length > maxObjectInitializersPerConstruction) {
    failBudget('struct construction exceeds the initializer limit');
  }
  countAggregateOperands(context, object.properties.length);
  for (const property of object.properties) {
    if (!ts.isPropertyAssignment(property) && !ts.isShorthandPropertyAssignment(property)) {
      collector.unsupported(property, sourceFile, file, 'aggregate construction field');
      return null;
    }
    if (!ts.isIdentifier(property.name) || property.modifiers?.length) {
      collector.unsupported(property, sourceFile, file, 'aggregate construction field name');
      return null;
    }
    const name = dataName(property.name, sourceFile, file, collector, 'aggregate construction field name');
    if (!name || seen.has(name.text)) {
      collector.unsupported(property.name, sourceFile, file, 'duplicate aggregate construction field');
      return null;
    }
    seen.add(name.text);
    if (ts.isShorthandPropertyAssignment(property)) {
      if (property.objectAssignmentInitializer) {
        collector.unsupported(property, sourceFile, file, 'aggregate shorthand initializer');
        return null;
      }
      const value = pushExpression(context, {
        span: nodeSpan(property.name, sourceFile, file), kind: { kind: 'reference', name },
      });
      fields.push({
        span: nodeSpan(property, sourceFile, file),
        kind: { kind: 'shorthand', name, value },
      });
      continue;
    }
    const colon = requiredToken(property, ts.SyntaxKind.ColonToken, sourceFile, 'an aggregate field colon');
    const value = normalizeExpression(property.initializer, context, depth + 1);
    if (value === null) return null;
    fields.push({
      span: nodeSpan(property, sourceFile, file),
      kind: { kind: 'explicit', name, colon_span: nodeSpan(colon, sourceFile, file), value },
    });
  }
  return fields;
}

function normalizeArrayConstruction(node, context, depth) {
  const { sourceFile, file, collector, typeSyntax, budgets } = context;
  if (
    !ts.isIdentifier(node.expression) || node.questionDotToken ||
    !node.typeArguments || node.arguments.length !== 1 ||
    !ts.isArrayLiteralExpression(node.arguments[0])
  ) return undefined;
  const spelling = node.expression.text;
  if (spelling !== 'Vec' && spelling !== 'FixedArray') return undefined;
  const requiredArguments = spelling === 'Vec' ? 1 : 2;
  if (node.typeArguments.length !== requiredArguments) {
    collector.unsupported(node, sourceFile, file, 'typed array construction');
    return null;
  }
  const array = node.arguments[0];
  if (array.elements.length > maxArrayElementsPerConstruction) {
    failBudget('array construction exceeds the element limit');
  }
  const elements = [];
  for (const element of array.elements) {
    if (ts.isOmittedExpression(element) || ts.isSpreadElement(element)) {
      collector.unsupported(element, sourceFile, file, 'array construction element');
      return null;
    }
    const value = normalizeExpression(element, context, depth + 1);
    if (value === null) return null;
    elements.push(value);
  }
  countAggregateOperands(context, elements.length);
  const name = dataName(node.expression, sourceFile, file, collector, 'array construction type');
  if (!name) return null;
  const less = requiredToken(node, ts.SyntaxKind.LessThanToken, sourceFile, 'a type-argument open token');
  const greater = requiredToken(node, ts.SyntaxKind.GreaterThanToken, sourceFile, 'a type-argument close token');
  const element = normalizeType(
    node.typeArguments[0], node.typeArguments[0].getStart(sourceFile), sourceFile, file, collector,
    'array element type', typeSyntax, budgets,
  );
  if (element === null) return null;
  let typeKind;
  if (spelling === 'Vec') {
    typeKind = {
      kind: 'vec', keyword_span: name.span, less_than_span: nodeSpan(less, sourceFile, file),
      argument: element, greater_than_span: nodeSpan(greater, sourceFile, file),
    };
  } else {
    const lengthNode = node.typeArguments[1];
    if (!ts.isLiteralTypeNode(lengthNode) || !ts.isNumericLiteral(lengthNode.literal)) {
      collector.unsupported(lengthNode, sourceFile, file, 'fixed-array length');
      return null;
    }
    const lengthSpelling = lengthNode.literal.getText(sourceFile);
    if (
      !/^(0|[1-9][0-9]*)$/.test(lengthSpelling) ||
      Buffer.byteLength(lengthSpelling, 'utf8') > maxFixedArrayLengthSpellingBytes ||
      BigInt(lengthSpelling) > BigInt(maxFixedArrayLength)
    ) {
      failBudget(`fixed-array length must be canonical and at most ${maxFixedArrayLength}`);
    }
    const comma = requiredToken(node, ts.SyntaxKind.CommaToken, sourceFile, 'a fixed-array type comma');
    typeKind = {
      kind: 'fixed-array', keyword_span: name.span, less_than_span: nodeSpan(less, sourceFile, file),
      element, comma_span: nodeSpan(comma, sourceFile, file),
      length_span: nodeSpan(lengthNode.literal, sourceFile, file),
      length: Number(lengthSpelling), length_spelling: lengthSpelling,
      greater_than_span: nodeSpan(greater, sourceFile, file),
    };
  }
  const typeId = pushType(typeSyntax, budgets, {
    span: spanFromOffsets(sourceFile, file, node.expression.getStart(sourceFile), greater.getEnd()),
    kind: typeKind,
  });
  const openParen = requiredToken(node, ts.SyntaxKind.OpenParenToken, sourceFile, 'an array construction open parenthesis');
  const closeParen = requiredToken(node, ts.SyntaxKind.CloseParenToken, sourceFile, 'an array construction close parenthesis');
  const openBracket = requiredToken(array, ts.SyntaxKind.OpenBracketToken, sourceFile, 'an array construction open bracket');
  const closeBracket = requiredToken(array, ts.SyntaxKind.CloseBracketToken, sourceFile, 'an array construction close bracket');
  return pushExpression(context, {
    span: nodeSpan(node, sourceFile, file), kind: {
      kind: spelling === 'Vec' ? 'vec-construction' : 'fixed-array-construction',
      type_syntax: typeId, open_paren_span: nodeSpan(openParen, sourceFile, file),
      open_bracket_span: nodeSpan(openBracket, sourceFile, file), elements,
      close_bracket_span: nodeSpan(closeBracket, sourceFile, file),
      close_paren_span: nodeSpan(closeParen, sourceFile, file),
    },
  });
}

function normalizeMatchExpression(node, context, depth) {
  const { sourceFile, file, collector } = context;
  if (
    !ts.isIdentifier(node.expression) || node.expression.text !== 'match' || node.questionDotToken ||
    node.typeArguments?.length || node.arguments.length !== 2 ||
    !ts.isObjectLiteralExpression(node.arguments[1])
  ) return undefined;
  const scrutinee = normalizeExpression(node.arguments[0], context, depth + 1);
  const object = node.arguments[1];
  if (object.properties.length > maxMatchArmsPerExpression) failBudget('match exceeds the arm limit');
  context.budgets.matchArms += object.properties.length;
  if (context.budgets.matchArms > maxMatchArmsPerProject) failBudget('project exceeds the match-arm limit');
  const arms = [];
  const seen = new Set();
  for (const property of object.properties) {
    if (!ts.isPropertyAssignment(property) || !ts.isStringLiteral(property.name) || !ts.isArrowFunction(property.initializer)) {
      collector.unsupported(property, sourceFile, file, 'match arm');
      return null;
    }
    const raw = property.name.getText(sourceFile);
    const qualified = property.name.text;
    const match = /^([A-Za-z_][A-Za-z0-9_]*)\.([A-Za-z_][A-Za-z0-9_]*)$/.exec(qualified);
    if (
      raw[0] !== '"' || raw.at(-1) !== '"' || raw.slice(1, -1) !== qualified || raw.includes('\\') ||
      !match || defensiveNames.has(match?.[1]) || defensiveNames.has(match?.[2]) || seen.has(qualified)
    ) {
      collector.unsupported(property.name, sourceFile, file, 'match arm key');
      return null;
    }
    seen.add(qualified);
    const arrow = property.initializer;
    const arrowToken = requiredToken(arrow, ts.SyntaxKind.EqualsGreaterThanToken, sourceFile, 'a match arm arrow');
    const parameterSpelling = sourceFile.text.slice(arrow.getStart(sourceFile), arrowToken.getStart(sourceFile)).trim();
    if (
      arrow.modifiers?.length || arrow.typeParameters?.length || arrow.type ||
      arrow.parameters.length > 1 || ts.isBlock(arrow.body) ||
      !parameterSpelling.startsWith('(') || !parameterSpelling.endsWith(')')
    ) {
      collector.unsupported(arrow, sourceFile, file, 'match arm function');
      return null;
    }
    let binding = null;
    if (arrow.parameters.length === 1) {
      const parameter = arrow.parameters[0];
      if (
        !ts.isIdentifier(parameter.name) || parameter.type || parameter.initializer ||
        parameter.questionToken || parameter.dotDotDotToken || parameter.modifiers?.length
      ) {
        collector.unsupported(parameter, sourceFile, file, 'match arm binding');
        return null;
      }
      binding = dataName(parameter.name, sourceFile, file, collector, 'match arm binding');
      if (!binding) return null;
    }
    const value = normalizeExpression(arrow.body, context, depth + 1);
    if (value === null) return null;
    const tokenStart = property.name.getStart(sourceFile) + 1;
    arms.push({
      span: nodeSpan(property, sourceFile, file),
      type_name: {
        text: match[1],
        span: spanFromOffsets(sourceFile, file, tokenStart, tokenStart + match[1].length),
      },
      dot_span: spanFromOffsets(sourceFile, file, tokenStart + match[1].length, tokenStart + match[1].length + 1),
      variant: {
        text: match[2],
        span: spanFromOffsets(sourceFile, file, tokenStart + match[1].length + 1, tokenStart + qualified.length),
      },
      binding, arrow_span: nodeSpan(arrowToken, sourceFile, file), value,
    });
  }
  const openParen = requiredToken(node, ts.SyntaxKind.OpenParenToken, sourceFile, 'a match open parenthesis');
  const closeParen = requiredToken(node, ts.SyntaxKind.CloseParenToken, sourceFile, 'a match close parenthesis');
  const openBrace = requiredToken(object, ts.SyntaxKind.OpenBraceToken, sourceFile, 'a match open brace');
  const closeBrace = requiredToken(object, ts.SyntaxKind.CloseBraceToken, sourceFile, 'a match close brace');
  if (scrutinee === null) return null;
  return pushExpression(context, {
    span: nodeSpan(node, sourceFile, file), kind: {
      kind: 'match', keyword_span: nodeSpan(node.expression, sourceFile, file),
      open_paren_span: nodeSpan(openParen, sourceFile, file), scrutinee,
      close_paren_span: nodeSpan(closeParen, sourceFile, file),
      open_brace_span: nodeSpan(openBrace, sourceFile, file), arms,
      close_brace_span: nodeSpan(closeBrace, sourceFile, file),
    },
  });
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
  if (ts.isStringLiteral(node)) {
    const spelling = node.getText(sourceFile);
    if ((spelling[0] !== '"' && spelling[0] !== "'") || spelling.at(-1) !== spelling[0] || spelling.includes('\\')) {
      collector.unsupported(node, sourceFile, file, 'string literal');
      return null;
    }
    return pushExpression(context, {
      span: nodeSpan(node, sourceFile, file), kind: { kind: 'string-literal', spelling },
    });
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
  } else if (ts.isPropertyAccessExpression(node) && !node.questionDotToken && ts.isIdentifier(node.name)) {
    const base = normalizeExpression(node.expression, context, depth + 1);
    const field = dataName(node.name, sourceFile, file, collector, 'field access name');
    const dot = requiredToken(node, ts.SyntaxKind.DotToken, sourceFile, 'a field-access dot');
    if (base !== null && field) {
      return pushExpression(context, {
        span: nodeSpan(node, sourceFile, file),
        kind: { kind: 'field-access', base, dot_span: nodeSpan(dot, sourceFile, file), field },
      });
    }
    return null;
  } else if (ts.isElementAccessExpression(node) && node.argumentExpression && !node.questionDotToken) {
    const base = normalizeExpression(node.expression, context, depth + 1);
    const index = normalizeExpression(node.argumentExpression, context, depth + 1);
    const open = requiredToken(node, ts.SyntaxKind.OpenBracketToken, sourceFile, 'an index open bracket');
    const close = requiredToken(node, ts.SyntaxKind.CloseBracketToken, sourceFile, 'an index close bracket');
    if (base !== null && index !== null) {
      return pushExpression(context, {
        span: nodeSpan(node, sourceFile, file),
        kind: {
          kind: 'index', base, open_bracket_span: nodeSpan(open, sourceFile, file), index,
          close_bracket_span: nodeSpan(close, sourceFile, file),
        },
      });
    }
    return null;
  } else if (ts.isCallExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === 'match') {
    const normalized = normalizeMatchExpression(node, context, depth);
    if (normalized !== undefined) return normalized;
    collector.unsupported(node, sourceFile, file, 'match expression');
    return null;
  } else if (ts.isCallExpression(node) && node.typeArguments?.length) {
    const normalized = normalizeArrayConstruction(node, context, depth);
    if (normalized !== undefined) return normalized;
    collector.unsupported(node, sourceFile, file, 'typed construction');
    return null;
  } else if (
    ts.isCallExpression(node) && ts.isIdentifier(node.expression) && !node.questionDotToken &&
    !node.typeArguments?.length && node.arguments.length === 1 && ts.isObjectLiteralExpression(node.arguments[0]) &&
    !new Set(['clone', 'borrow', 'borrowMut', 'shared', 'downgrade', 'push', 'match', 'upgradeWeak']).has(node.expression.text)
  ) {
    const typeName = dataName(node.expression, sourceFile, file, collector, 'struct construction type');
    const fields = normalizeConstructionFields(node.arguments[0], context, depth);
    const openParen = requiredToken(node, ts.SyntaxKind.OpenParenToken, sourceFile, 'a construction open parenthesis');
    const closeParen = requiredToken(node, ts.SyntaxKind.CloseParenToken, sourceFile, 'a construction close parenthesis');
    const openBrace = requiredToken(node.arguments[0], ts.SyntaxKind.OpenBraceToken, sourceFile, 'a construction open brace');
    const closeBrace = requiredToken(node.arguments[0], ts.SyntaxKind.CloseBraceToken, sourceFile, 'a construction close brace');
    if (typeName && fields) {
      return pushExpression(context, {
        span: nodeSpan(node, sourceFile, file), kind: {
          kind: 'struct-construction', type_name: typeName,
          open_paren_span: nodeSpan(openParen, sourceFile, file),
          open_brace_span: nodeSpan(openBrace, sourceFile, file), fields,
          close_brace_span: nodeSpan(closeBrace, sourceFile, file),
          close_paren_span: nodeSpan(closeParen, sourceFile, file),
        },
      });
    }
    return null;
  } else if (
    ts.isCallExpression(node) && ts.isPropertyAccessExpression(node.expression) &&
    ts.isIdentifier(node.expression.expression) && ts.isIdentifier(node.expression.name) &&
    !node.questionDotToken && !node.typeArguments?.length && node.arguments.length <= 1
  ) {
    const typeName = dataName(node.expression.expression, sourceFile, file, collector, 'enum construction type');
    const variant = dataName(node.expression.name, sourceFile, file, collector, 'enum construction variant');
    const dot = requiredToken(node.expression, ts.SyntaxKind.DotToken, sourceFile, 'an enum construction dot');
    const open = requiredToken(node, ts.SyntaxKind.OpenParenToken, sourceFile, 'an enum construction open parenthesis');
    const close = requiredToken(node, ts.SyntaxKind.CloseParenToken, sourceFile, 'an enum construction close parenthesis');
    const payload = node.arguments.length === 1 ? normalizeExpression(node.arguments[0], context, depth + 1) : null;
    countAggregateOperands(context, node.arguments.length);
    if (typeName && variant && (node.arguments.length === 0 || payload !== null)) {
      return pushExpression(context, {
        span: nodeSpan(node, sourceFile, file), kind: {
          kind: 'enum-construction', type_name: typeName, dot_span: nodeSpan(dot, sourceFile, file),
          variant, open_paren_span: nodeSpan(open, sourceFile, file), payload,
          close_paren_span: nodeSpan(close, sourceFile, file),
        },
      });
    }
    return null;
  } else if (
    ts.isCallExpression(node) && ts.isIdentifier(node.expression) && !node.typeArguments?.length &&
    !node.questionDotToken && ['clone', 'borrow', 'borrowMut', 'shared', 'downgrade'].includes(node.expression.text)
  ) {
    if (node.arguments.length !== 1 || ts.isSpreadElement(node.arguments[0])) {
      collector.unsupported(node, sourceFile, file, 'ownership intrinsic');
      return null;
    }
    const value = normalizeExpression(node.arguments[0], context, depth + 1);
    const open = requiredToken(node, ts.SyntaxKind.OpenParenToken, sourceFile, 'an intrinsic open parenthesis');
    const close = requiredToken(node, ts.SyntaxKind.CloseParenToken, sourceFile, 'an intrinsic close parenthesis');
    const kind = new Map([
      ['clone', 'clone'], ['borrow', 'borrow'], ['borrowMut', 'borrow-mut'],
      ['shared', 'shared'], ['downgrade', 'downgrade'],
    ]).get(node.expression.text);
    if (value !== null) return pushExpression(context, {
      span: nodeSpan(node, sourceFile, file), kind: {
        kind, keyword_span: nodeSpan(node.expression, sourceFile, file),
        open_paren_span: nodeSpan(open, sourceFile, file), value,
        close_paren_span: nodeSpan(close, sourceFile, file),
      },
    });
    return null;
  } else if (
    ts.isCallExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === 'push' &&
    !node.typeArguments?.length && !node.questionDotToken
  ) {
    if (node.arguments.length !== 2 || node.arguments.some(ts.isSpreadElement)) {
      collector.unsupported(node, sourceFile, file, 'vector push');
      return null;
    }
    const vector = normalizeExpression(node.arguments[0], context, depth + 1);
    const value = normalizeExpression(node.arguments[1], context, depth + 1);
    const open = requiredToken(node, ts.SyntaxKind.OpenParenToken, sourceFile, 'a vector-push open parenthesis');
    const comma = requiredToken(node, ts.SyntaxKind.CommaToken, sourceFile, 'a vector-push comma');
    const close = requiredToken(node, ts.SyntaxKind.CloseParenToken, sourceFile, 'a vector-push close parenthesis');
    if (vector !== null && value !== null) return pushExpression(context, {
      span: nodeSpan(node, sourceFile, file), kind: {
        kind: 'vec-push', keyword_span: nodeSpan(node.expression, sourceFile, file),
        open_paren_span: nodeSpan(open, sourceFile, file), vector,
        comma_span: nodeSpan(comma, sourceFile, file), value,
        close_paren_span: nodeSpan(close, sourceFile, file),
      },
    });
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
      const typeSyntax = normalizeType(
        declaration.type, declaration.name.getEnd(), sourceFile, file, collector,
        'local annotation', context.typeSyntax, context.budgets,
      );
      const initializer = normalizeExpression(declaration.initializer, context, depth + 1);
      if (semicolon && name && typeSyntax !== null && initializer !== null) kind = { kind: 'local-declaration', keyword_span: nodeSpan(keyword, sourceFile, file), mutable, name, type_syntax: typeSyntax, equals_span: nodeSpan(equals, sourceFile, file), initializer, semicolon_span: semicolon };
    }
  } else if (
    ts.isExpressionStatement(node) && ts.isCallExpression(node.expression) &&
    ts.isIdentifier(node.expression.expression) && node.expression.expression.text === 'upgradeWeak'
  ) {
    const call = node.expression;
    const semicolon = requireSemicolon(node, context, 'weak upgrade');
    if (
      call.questionDotToken || call.typeArguments?.length || call.arguments.length !== 3 ||
      !ts.isArrowFunction(call.arguments[1]) || !ts.isArrowFunction(call.arguments[2])
    ) {
      collector.unsupported(call, sourceFile, file, 'weak upgrade');
    } else {
      const success = call.arguments[1];
      const failure = call.arguments[2];
      const successParameter = success.parameters[0];
      const validSuccess =
        success.parameters.length === 1 && successParameter && ts.isIdentifier(successParameter.name) &&
        !successParameter.type && !successParameter.initializer && !successParameter.questionToken &&
        !successParameter.dotDotDotToken && !successParameter.modifiers?.length && ts.isBlock(success.body) &&
        !success.modifiers?.length && !success.typeParameters?.length && !success.type;
      const validFailure =
        failure.parameters.length === 0 && ts.isBlock(failure.body) &&
        !failure.modifiers?.length && !failure.typeParameters?.length && !failure.type;
      const asToken = requiredToken(success, ts.SyntaxKind.EqualsGreaterThanToken, sourceFile, 'a weak-upgrade success arrow');
      const elseToken = requiredToken(failure, ts.SyntaxKind.EqualsGreaterThanToken, sourceFile, 'a weak-upgrade failure arrow');
      const successParameters = sourceFile.text.slice(success.getStart(sourceFile), asToken.getStart(sourceFile)).trim();
      const failureParameters = sourceFile.text.slice(failure.getStart(sourceFile), elseToken.getStart(sourceFile)).trim();
      if (
        !validSuccess || !validFailure ||
        !successParameters.startsWith('(') || !successParameters.endsWith(')') ||
        !failureParameters.startsWith('(') || !failureParameters.endsWith(')')
      ) {
        collector.unsupported(call, sourceFile, file, 'weak upgrade callbacks');
      } else {
        const weak = normalizeExpression(call.arguments[0], context, depth + 1);
        const binding = dataName(successParameter.name, sourceFile, file, collector, 'weak upgrade binding');
        const successBlock = allocateBlock(success.body, context, depth + 1);
        const failureBlock = allocateBlock(failure.body, context, depth + 1);
        if (semicolon && weak !== null && binding && successBlock !== null && failureBlock !== null) {
          kind = {
            kind: 'weak-upgrade', keyword_span: nodeSpan(call.expression, sourceFile, file), weak,
            as_span: nodeSpan(asToken, sourceFile, file), binding, success_block: successBlock,
            else_span: nodeSpan(elseToken, sourceFile, file), failure_block: failureBlock,
          };
        }
      }
    }
  } else if (ts.isExpressionStatement(node) && ts.isBinaryExpression(node.expression) && node.expression.operatorToken.kind === ts.SyntaxKind.EqualsToken) {
    const semicolon = requireSemicolon(node, context, 'assignment');
    const target = normalizeExpression(node.expression.left, context, depth + 1);
    const value = normalizeExpression(node.expression.right, context, depth + 1);
    if (semicolon && target !== null && value !== null) kind = { kind: 'assignment', target, equals_span: nodeSpan(node.expression.operatorToken, sourceFile, file), value, semicolon_span: semicolon };
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
  } else if (ts.isExpressionStatement(node)) {
    const semicolon = requireSemicolon(node, context, 'expression statement');
    const expression = normalizeExpression(node.expression, context, depth + 1);
    if (semicolon && expression !== null) kind = { kind: 'expression-statement', expression, semicolon_span: semicolon };
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

const defensiveNames = new Set(['constructor', 'prototype', '__proto__']);

function dataName(node, sourceFile, file, collector, context) {
  const name = normalizedIdentifier(node, sourceFile, file, collector, context);
  if (!name) return null;
  if (Buffer.byteLength(name.text, 'utf8') > maxIdentifierBytes || defensiveNames.has(name.text)) {
    collector.unsupported(node, sourceFile, file, context);
    return null;
  }
  return name;
}

function normalizeDataDeclaration(node, sourceFile, file, collector, budgets, typeSyntax) {
  let exportSpan = null;
  for (const modifier of node.modifiers ?? []) {
    if (modifier.kind === ts.SyntaxKind.ExportKeyword && exportSpan === null) {
      exportSpan = nodeSpan(modifier, sourceFile, file);
    } else {
      collector.unsupported(modifier, sourceFile, file, 'data declaration modifier');
      return null;
    }
  }
  if (node.typeParameters?.length || node.heritageClauses?.length !== 1) {
    collector.unsupported(node, sourceFile, file, 'data declaration');
    return null;
  }
  const heritage = node.heritageClauses[0];
  const markerType = heritage.types?.[0];
  if (
    heritage.token !== ts.SyntaxKind.ExtendsKeyword || heritage.types.length !== 1 ||
    !markerType || !ts.isIdentifier(markerType.expression) || markerType.typeArguments?.length
  ) {
    collector.unsupported(heritage, sourceFile, file, 'data declaration marker');
    return null;
  }
  const marker = markerType.expression.text;
  if (marker !== 'ZrynaStruct' && marker !== 'ZrynaEnum') {
    collector.unsupported(markerType, sourceFile, file, 'data declaration marker');
    return null;
  }
  const name = dataName(node.name, sourceFile, file, collector, 'data declaration name');
  if (!name) return null;
  if (node.members.length === 0) {
    collector.unsupported(node, sourceFile, file, 'empty data declaration');
    return null;
  }
  if (node.members.length > maxMembersPerDeclaration) {
    failBudget('data declaration exceeds the member limit');
  }
  budgets.members += node.members.length;
  if (budgets.members > maxMembersPerProject) failBudget('project exceeds the data-member limit');
  const seen = new Set();
  const members = [];
  for (const member of node.members) {
    if (
      !ts.isPropertySignature(member) || !member.type || !member.name ||
      !ts.isIdentifier(member.name) || member.questionToken || member.modifiers?.length
    ) {
      collector.unsupported(member, sourceFile, file, 'data member');
      return null;
    }
    const memberName = dataName(member.name, sourceFile, file, collector, 'data member name');
    if (!memberName || seen.has(memberName.text)) {
      collector.unsupported(member.name, sourceFile, file, 'duplicate or invalid data member name');
      return null;
    }
    seen.add(memberName.text);
    const colon = requiredToken(member, ts.SyntaxKind.ColonToken, sourceFile, 'a data member colon');
    const semicolon = requiredToken(member, ts.SyntaxKind.SemicolonToken, sourceFile, 'a data member semicolon');
    const base = {
      span: nodeSpan(member, sourceFile, file), name: memberName,
      colon_span: nodeSpan(colon, sourceFile, file), semicolon_span: nodeSpan(semicolon, sourceFile, file),
    };
    if (marker === 'ZrynaEnum' && ts.isTypeReferenceNode(member.type) && ts.isIdentifier(member.type.typeName) && member.type.typeName.text === 'ZrynaNone' && !member.type.typeArguments?.length) {
      members.push({ ...base, payload_type: null, none_span: nodeSpan(member.type, sourceFile, file) });
    } else {
      const typeId = normalizeType(member.type, member.name.getEnd(), sourceFile, file, collector, 'data member type', typeSyntax, budgets);
      if (typeId === null) return null;
      if (marker === 'ZrynaEnum') members.push({ ...base, payload_type: typeId, none_span: null });
      else members.push({ ...base, type_syntax: typeId });
    }
  }
  budgets.dataDeclarations += 1;
  if (budgets.dataDeclarations > maxNominalDeclarationsPerProject) failBudget('project exceeds the nominal-declaration limit');
  const interfaceToken = requiredToken(node, ts.SyntaxKind.InterfaceKeyword, sourceFile, 'an interface keyword');
  const extendsToken = requiredToken(heritage, ts.SyntaxKind.ExtendsKeyword, sourceFile, 'an extends keyword');
  const open = requiredToken(node, ts.SyntaxKind.OpenBraceToken, sourceFile, 'a data declaration open brace');
  const close = requiredToken(node, ts.SyntaxKind.CloseBraceToken, sourceFile, 'a data declaration close brace');
  const common = {
    kind: marker === 'ZrynaStruct' ? 'struct' : 'enum',
    interface_span: nodeSpan(interfaceToken, sourceFile, file), name,
    extends_span: nodeSpan(extendsToken, sourceFile, file),
    marker_span: nodeSpan(markerType.expression, sourceFile, file),
    open_brace_span: nodeSpan(open, sourceFile, file), close_brace_span: nodeSpan(close, sourceFile, file),
  };
  if (marker === 'ZrynaStruct') common.fields = members;
  else common.variants = members;
  return { span: nodeSpan(node, sourceFile, file), export_span: exportSpan, kind: common };
}

function normalizeFunction(node, sourceFile, file, collector, budgets, typeSyntax, index) {
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
  const parameters = node.parameters.map((parameter, parameterIndex) => normalizeParameter(parameter, sourceFile, file, collector, parameterIndex, typeSyntax, budgets));
  if (parameters.some((parameter) => parameter === null)) valid = false;
  const resultType = normalizeType(node.type, node.parameters.end, sourceFile, file, collector, `function ${index} result annotation`, typeSyntax, budgets);
  if (resultType === null) valid = false;
  if (!node.body) return null;
  const context = { sourceFile, file, collector, budgets, typeSyntax, blocks: [], statements: [], expressions: [], locals: 0, valid: true };
  const rootBlock = allocateBlock(node.body, context, 1);
  valid &&= context.valid && rootBlock === 0;
  const functionToken = requiredToken(node, ts.SyntaxKind.FunctionKeyword, sourceFile, 'a function keyword');
  if (!valid || !name || resultType === null || rootBlock === null) return null;
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
  const typeSyntax = [];
  const dataDeclarations = [];
  const functions = [];
  let sawNonImport = false;
  let importDeclarations = 0;
  let functionDeclarations = 0;
  for (const node of sourceFile.statements) {
    if (ts.isImportDeclaration(node)) {
      if (sawNonImport) {
        collector.unsupported(node, sourceFile, file, 'import after declaration');
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
    sawNonImport = true;
    if (ts.isInterfaceDeclaration(node)) {
      if (dataDeclarations.length >= maxNominalDeclarationsPerModule) {
        failBudget('module exceeds the nominal-declaration limit');
      }
      const normalized = normalizeDataDeclaration(node, sourceFile, file, collector, budgets, typeSyntax);
      if (normalized) dataDeclarations.push(normalized);
      continue;
    }
    if (!ts.isFunctionDeclaration(node)) {
      collector.unsupported(node, sourceFile, file, 'top-level declaration');
      continue;
    }
    functionDeclarations += 1;
    if (functionDeclarations > maxFunctionsPerFile) failBudget('module exceeds the function limit');
    budgets.functions += 1;
    if (budgets.functions > maxFunctionsPerProject) failBudget('project exceeds the function limit');
    const normalized = normalizeFunction(node, sourceFile, file, collector, budgets, typeSyntax, functionDeclarations - 1);
    if (normalized) functions.push(normalized);
  }
  return { id: file, path: input.path, imports, type_syntax: typeSyntax, data_declarations: dataDeclarations, functions };
}

function validateAnalyzeParams(params) {
  requireExactKeys(params, ['schema_version', 'files'], 'analyze params');
  if (params.schema_version !== protocolVersion || !Array.isArray(params.files)) failRequest('analyze requires a protocol-v4 file list');
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
    return {
      id: request.id,
      result: {
        provider: 'typescript-6', provider_version: providerVersion, protocol_version: protocolVersion,
        capabilities: {
          module_resolution: false, semantic_diagnostics: false,
          control_flow_v1: true, data_ownership_syntax_v1: true,
        },
      },
    };
  }
  if (request.method === 'analyze') {
    requireExactKeys(request, ['id', 'method', 'params'], 'analyze request');
    const files = validateAnalyzeParams(request.params);
    const collector = new DiagnosticCollector();
    const budgets = {
      sourceBytes: 0, imports: 0, bindings: 0, functions: 0, parameters: 0,
      blocks: 0, statements: 0, expressions: 0, locals: 0,
      dataDeclarations: 0, members: 0, types: 0, aggregateOperands: 0, matchArms: 0,
    };
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
