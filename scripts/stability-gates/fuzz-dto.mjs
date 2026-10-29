// Detached structural checks only. Rust's source/graph verifier remains an execution observation.
import assert from 'node:assert/strict';
import { canonical, exact } from './input.mjs';
import { walkRanges } from './fuzz-ranges.mjs';


function array(value, maximum) {
  assert(Array.isArray(value) && value.length <= maximum, 'bounded DTO collection');
  return value;
}

function object(value) {
  assert(value !== null && typeof value === 'object' && !Array.isArray(value), 'DTO object');
  return value;
}

function identifier(value) {
  exact(value, ['text', 'span'], 'identifier');
  assert.equal(typeof value.text, 'string');
  assert([...value.text].length <= 1024);
}

function typed(value, version) {
  if (version === 4) { assert(Number.isSafeInteger(value) && value >= 0 && value <= 0xffffffff); return; }
  exact(value, ['span', 'kind'], 'type syntax');
  object(value.kind);
  if (value.kind.kind === 'missing') exact(value.kind, ['kind'], 'missing type');
  else {
    exact(value.kind, ['kind', 'name'], 'named type');
    assert.equal(value.kind.kind, 'named');
    assert.equal(typeof value.kind.name, 'string');
  }
}

function functionShape(value, version) {
  exact(value, ['span', 'export_span', 'function_span', 'name', 'parameters', 'result_type', 'body'], 'function');
  identifier(value.name);
  for (const parameter of array(value.parameters, 256)) {
    exact(parameter, ['span', 'name', 'type_syntax'], 'parameter');
    identifier(parameter.name);
    typed(parameter.type_syntax, version);
  }
  typed(value.result_type, version);
  const body = value.body;
  exact(body, version === 2 ? ['span', 'statements', 'expressions'] :
    ['span', 'root_block', 'blocks', 'statements', 'expressions'], 'function body');
  for (const key of ['statements', 'expressions']) {
    for (const node of array(body[key], key === 'expressions' ? 16384 : 4096)) {
      exact(node, ['span', 'kind'], 'arena node');
      object(node.kind);
      assert.equal(typeof node.kind.kind, 'string');
      assert(node.kind.kind.length > 0 && node.kind.kind.length <= 64);
    }
  }
  if (version !== 2) {
    assert(Number.isSafeInteger(body.root_block) && body.root_block >= 0 && body.root_block <= 0xffffffff);
    for (const block of array(body.blocks, 4096)) {
      exact(block, ['span', 'open_brace_span', 'statements', 'close_brace_span'], 'block');
      for (const id of array(block.statements, 4096)) assert(Number.isSafeInteger(id) && id >= 0 && id <= 0xffffffff);
    }
  }
}

function diagnostics(values) {
  for (const diagnostic of array(values, 256)) {
    exact(diagnostic, ['code', 'severity', 'location', 'message', 'guidance'], 'provider diagnostic');
    assert.match(diagnostic.code, /^ZRYNA-[A-Z][0-9]{4}$/);
    assert(['error', 'warning'].includes(diagnostic.severity));
    for (const key of ['message', 'guidance']) {
      assert.equal(typeof diagnostic[key], 'string');
      assert([...diagnostic[key]].length <= 4096);
    }
    if (diagnostic.location?.kind === 'global') exact(diagnostic.location, ['kind'], 'global location');
    else {
      exact(diagnostic.location, ['kind', 'span'], 'source location');
      assert.equal(diagnostic.location.kind, 'source');
    }
  }
}


export function acceptedDTO(raw, source, version) {
  canonical(raw); // Existing document/node/depth/string budget; not a Rust verifier.
  assert([2, 3, 4].includes(version));
  exact(raw, ['schema_version', 'files', 'diagnostics'], 'accepted DTO');
  assert.equal(raw.schema_version, version);
  assert.equal(array(raw.files, 1).length, 1, 'complete retained one-file source');
  const file = raw.files[0];
  const fields = ['id', 'path', 'functions'];
  if (version > 2) fields.push('imports');
  if (version === 4) fields.push('type_syntax', 'data_declarations');
  exact(file, fields, 'source unit');
  assert.equal(file.id, 0);
  assert.equal(file.path, source.path);
  for (const fn of array(file.functions, 4096)) functionShape(fn, version);
  if (version > 2) for (const entry of array(file.imports, 4096)) {
    object(entry);
    assert(Object.hasOwn(entry, 'span'));
    array(entry.bindings, 256);
  }
  if (version === 4) for (const key of ['type_syntax', 'data_declarations']) {
    for (const entry of array(file[key], 16384)) {
      object(entry);
      assert(Object.hasOwn(entry, 'span'));
    }
  }
  diagnostics(raw.diagnostics);
  walkRanges(raw, source, version);
}
