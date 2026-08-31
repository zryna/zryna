import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import Ajv2020 from "ajv/dist/2020.js";

const parse = async (url) => JSON.parse(await readFile(url, "utf8"));
const schemaUrl = new URL("../schemas/zryna-syntax-v4.schema.json", import.meta.url);
const fixtureUrl = new URL("./m3-fixtures/syntax-v4-valid.json", import.meta.url);

const load = async () => {
  const schema = await parse(schemaUrl);
  const fixture = await parse(fixtureUrl);
  const validate = new Ajv2020({ allErrors: true, strict: true }).compile(schema);
  return { schema, fixture, validate };
};

const tags = (schema, name) => schema.$defs[name].oneOf.flatMap((entry) => {
  const resolved = entry.$ref
    ? schema.$defs[entry.$ref.slice("#/$defs/".length)]
    : entry;
  const kind = resolved.properties.kind;
  return kind.enum ?? [kind.const];
});

test("protocol-v4 accepts the M3 golden and freezes its closed tag inventory", async () => {
  const { schema, fixture, validate } = await load();
  assert.equal(validate(fixture), true, JSON.stringify(validate.errors));
  assert.equal(schema.properties.schema_version.const, 4);
  assert.deepEqual(schema.$defs.sourceUnit.required, [
    "id", "path", "imports", "type_syntax", "data_declarations", "functions",
  ]);
  assert.deepEqual(tags(schema, "typeKind"), [
    "missing", "named", "string", "vec", "shared", "weak", "borrow", "borrow-mut",
    "fixed-array",
  ]);
  assert.deepEqual(tags(schema, "dataDeclarationKind"), ["struct", "enum"]);
  assert.deepEqual(tags(schema, "statementKind"), [
    "local-declaration", "assignment", "return", "block", "if", "while",
    "expression-statement", "weak-upgrade",
  ]);
  assert.deepEqual(tags(schema, "expressionKind"), [
    "reference", "bool-literal", "i32-literal", "string-literal", "negation",
    "addition", "subtraction", "multiplication", "equal", "not-equal", "less-than",
    "less-equal", "greater-than", "greater-equal", "call", "struct-construction",
    "enum-construction", "fixed-array-construction", "vec-construction", "field-access",
    "index", "clone", "shared", "downgrade", "borrow", "borrow-mut", "vec-push", "match",
  ]);
});

test("protocol-v4 closes every nested record and rejects prototype-sensitive names", async () => {
  const { fixture, validate } = await load();
  const unknownUnit = structuredClone(fixture);
  unknownUnit.files[0].unknown = true;
  assert.equal(validate(unknownUnit), false);
  const unknownKind = structuredClone(fixture);
  unknownKind.files[0].data_declarations[0].kind.unknown = true;
  assert.equal(validate(unknownKind), false);
  const rejectedName = await parse(
    new URL("./m3-fixtures/syntax-v4-rejected-name.json", import.meta.url),
  );
  assert.equal(validate(rejectedName), false);
  for (const name of ["__proto__", "prototype", "constructor"]) {
    const candidate = structuredClone(fixture);
    candidate.files[0].data_declarations[0].kind.name.text = name;
    assert.equal(validate(candidate), false, `${name} was accepted`);
  }
});

test("protocol-v4 identifier and diagnostic schema ceilings accept exact and reject first extra", async () => {
  const { schema, validate } = await load();
  const validateIdentifier = new Ajv2020({ strict: true }).compile({
    $ref: "#/$defs/identifierText", $defs: schema.$defs,
  });
  assert.equal(validateIdentifier(`a${"b".repeat(127)}`), true);
  for (const spelling of [
    `a${"b".repeat(128)}`, "café", "$value", "__proto__", "prototype", "constructor",
  ]) assert.equal(validateIdentifier(spelling), false, `${spelling} was accepted`);

  const diagnostic = {
    code: "P0001", severity: "warning", location: { kind: "global" },
    message: "message", guidance: "guidance",
  };
  const exact = { schema_version: 4, files: [], diagnostics: Array(256).fill(diagnostic) };
  assert.equal(validate(exact), true, JSON.stringify(validate.errors));
  const firstExtra = { ...exact, diagnostics: Array(257).fill(diagnostic) };
  assert.equal(validate(firstExtra), false, "diagnostic limit + 1 was accepted");
});

test("protocol-v4 exposes declaration, type-arena, initializer, and arm limits", async () => {
  const { fixture, validate } = await load();
  const declarations = structuredClone(fixture);
  declarations.files[0].data_declarations = Array(4097).fill(
    declarations.files[0].data_declarations[0],
  );
  assert.equal(validate(declarations), false);
  const fields = structuredClone(fixture);
  fields.files[0].data_declarations[0].kind.fields = Array(1025).fill(
    fields.files[0].data_declarations[0].kind.fields[0],
  );
  assert.equal(validate(fields), false);
  const types = structuredClone(fixture);
  types.files[0].type_syntax = Array(65537).fill(types.files[0].type_syntax[0]);
  assert.equal(validate(types), false);
  assert.equal(
    new Ajv2020({ strict: true }).compile(
      { $ref: "#/$defs/expressionKind", $defs: (await parse(schemaUrl)).$defs },
    )({
      kind: "vec-construction",
      type_syntax: 2,
      open_paren_span: { file: 0, start: 0, end: 1 },
      open_bracket_span: { file: 0, start: 1, end: 2 },
      elements: Array(4097).fill(0),
      close_bracket_span: { file: 0, start: 2, end: 3 },
      close_paren_span: { file: 0, start: 3, end: 4 },
    }),
    false,
  );
});

test("enum variants encode exactly one of ZrynaNone or a payload type", async () => {
  const { fixture, validate } = await load();
  const noneWithPayload = structuredClone(fixture);
  noneWithPayload.files[0].data_declarations[1].kind.variants[0].payload_type = 0;
  assert.equal(validate(noneWithPayload), false);
  const payloadWithNone = structuredClone(fixture);
  payloadWithNone.files[0].data_declarations[1].kind.variants[1].none_span = {
    file: 0, start: 0, end: 1,
  };
  assert.equal(validate(payloadWithNone), false);
  const neither = structuredClone(fixture);
  neither.files[0].data_declarations[1].kind.variants[0].none_span = null;
  assert.equal(validate(neither), false);
});

test("FixedArray length has canonical spelling and an explicit numeric profile bound", async () => {
  const { fixture, validate } = await load();
  for (const spelling of ["0", "2", "1048576"]) {
    const candidate = structuredClone(fixture);
    candidate.files[0].type_syntax[7].kind.length_spelling = spelling;
    candidate.files[0].type_syntax[7].kind.length = Number(spelling);
    assert.equal(validate(candidate), true, `${spelling}: ${JSON.stringify(validate.errors)}`);
  }
  for (const spelling of ["", "00", "+2", "-1", "42949672960", "1_000"]) {
    const candidate = structuredClone(fixture);
    candidate.files[0].type_syntax[7].kind.length_spelling = spelling;
    if (spelling === "1048577") candidate.files[0].type_syntax[7].kind.length = 1048577;
    assert.equal(validate(candidate), false, `${spelling} was accepted`);
  }
  const overflow = structuredClone(fixture);
  overflow.files[0].type_syntax[7].kind.length_spelling = "1048577";
  overflow.files[0].type_syntax[7].kind.length = 1048577;
  assert.equal(validate(overflow), false, "fixed-array profile limit + 1 was accepted");
  const mismatch = structuredClone(fixture);
  mismatch.files[0].type_syntax[7].kind.length_spelling = "2";
  mismatch.files[0].type_syntax[7].kind.length = 3;
  assert.equal(validate(mismatch), true, "cross-field equality belongs to the Rust verifier");
});

test("string literal spelling is one simple quoted token", async () => {
  const schema = await parse(schemaUrl);
  const validate = new Ajv2020({ strict: true }).compile({
    $ref: "#/$defs/expressionKind", $defs: schema.$defs,
  });
  for (const spelling of ["\"hello\"", "'hello'"]) {
    assert.equal(validate({ kind: "string-literal", spelling }), true);
  }
  for (const spelling of ["hello", "\"line\\nfeed\"", "\"unterminated", "\"a\nb\""]) {
    assert.equal(validate({ kind: "string-literal", spelling }), false, spelling);
  }
});

test("v2 and v3 contracts remain independently valid", async () => {
  for (const version of [2, 3]) {
    const schema = await parse(
      new URL(`../schemas/zryna-syntax-v${version}.schema.json`, import.meta.url),
    );
    const fixture = await parse(
      new URL(`./fixtures/syntax-v${version}-valid.json`, import.meta.url),
    );
    const validate = new Ajv2020({ allErrors: true, strict: true }).compile(schema);
    assert.equal(schema.properties.schema_version.const, version);
    assert.equal(validate(fixture), true, JSON.stringify(validate.errors));
  }
});
