import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import Ajv2020 from "ajv/dist/2020.js";

const fixtureUrl = (name) => new URL(`./fixtures/${name}`, import.meta.url);
const parse = async (url) => JSON.parse(await readFile(url, "utf8"));

test("protocol-v2 schema accepts the shared golden fixture", async () => {
  const schema = await parse(new URL("../schemas/zryna-syntax-v2.schema.json", import.meta.url));
  const fixture = await parse(fixtureUrl("syntax-v2-valid.json"));
  const validate = new Ajv2020({ allErrors: true, strict: true }).compile(schema);

  assert.equal(validate(fixture), true, JSON.stringify(validate.errors));
  assert.equal(schema.properties.schema_version.const, 2);
  assert.equal(schema.properties.files.maxItems, 4096);
  assert.equal(schema.$defs.body.properties.expressions.maxItems, 16384);
});

test("protocol-v2 schema rejects shared unknown and missing field fixtures", async () => {
  const schema = await parse(new URL("../schemas/zryna-syntax-v2.schema.json", import.meta.url));
  const validate = new Ajv2020({ allErrors: true, strict: true }).compile(schema);
  for (const name of ["syntax-v2-unknown-field.json", "syntax-v2-missing-field.json"]) {
    const fixture = await parse(fixtureUrl(name));
    assert.equal(validate(fixture), false, `${name} unexpectedly passed`);
  }
});

test("protocol-v2 schema mirrors canonical literal and portable path syntax", async () => {
  const schema = await parse(new URL("../schemas/zryna-syntax-v2.schema.json", import.meta.url));
  const fixture = await parse(fixtureUrl("syntax-v2-valid.json"));
  const validate = new Ajv2020({ allErrors: true, strict: true }).compile(schema);

  const negativeZero = structuredClone(fixture);
  negativeZero.files[0].functions[0].body.expressions[0].kind = {
    kind: "i32-literal",
    spelling: "-0",
  };
  assert.equal(validate(negativeZero), false, "negative zero unexpectedly passed");

  for (const path of [
    "../main.zry",
    "/main.zry",
    "src\\main.zry",
    "src//main.zry",
    "con.zry",
    "src/NUL",
    "src/CoM1.any",
    `src/${"a".repeat(256)}`,
    Array.from({ length: 33 }, () => "a").join("/"),
  ]) {
    const unsafePath = structuredClone(fixture);
    unsafePath.files[0].path = path;
    assert.equal(validate(unsafePath), false, `${path} unexpectedly passed`);
  }

  for (const path of [
    `src/${"a".repeat(255)}`,
    Array.from({ length: 32 }, () => "a").join("/"),
  ]) {
    const portablePath = structuredClone(fixture);
    portablePath.files[0].path = path;
    assert.equal(validate(portablePath), true, JSON.stringify(validate.errors));
  }
});

test("protocol-v2 schema covers every initial tagged variant and nested strictness", async () => {
  const schema = await parse(new URL("../schemas/zryna-syntax-v2.schema.json", import.meta.url));
  const fixture = await parse(fixtureUrl("syntax-v2-valid.json"));
  const validate = new Ajv2020({ allErrors: true, strict: true }).compile(schema);
  const expression = fixture.files[0].functions[0].body.expressions[0];

  const variants = structuredClone(fixture);
  variants.files[0].functions[0].result_type = {
    span: { file: 0, start: 23, end: 23 },
    kind: { kind: "missing" },
  };
  variants.files[0].functions[0].body.expressions = [
    {
      span: expression.span,
      kind: { kind: "reference", name: { text: "value", span: expression.span } },
    },
    { span: expression.span, kind: { kind: "i32-literal", spelling: "1" } },
    {
      span: expression.span,
      kind: {
        kind: "addition",
        operator_span: expression.span,
        lhs: 0,
        rhs: 1,
      },
    },
  ];
  variants.files[0].functions[0].body.statements[0].kind.value = 2;
  variants.diagnostics = [
    {
      code: "P1000",
      severity: "warning",
      location: { kind: "global" },
      message: "global note",
      guidance: "review it",
    },
    {
      code: "P1001",
      severity: "error",
      location: { kind: "source", span: expression.span },
      message: "source error",
      guidance: "fix it",
    },
  ];
  assert.equal(validate(variants), true, JSON.stringify(validate.errors));

  const nestedUnknown = structuredClone(fixture);
  nestedUnknown.files[0].functions[0].name.unknown = true;
  assert.equal(validate(nestedUnknown), false, "nested unknown field unexpectedly passed");

  const unknownVariant = structuredClone(fixture);
  unknownVariant.files[0].functions[0].body.expressions[0].kind.kind = "dynamic";
  assert.equal(validate(unknownVariant), false, "unknown expression kind unexpectedly passed");

  const oversizedName = structuredClone(fixture);
  oversizedName.files[0].functions[0].name.text = "x".repeat(1025);
  assert.equal(validate(oversizedName), false, "oversized identifier unexpectedly passed");
});
