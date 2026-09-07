import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import Ajv2020 from "ajv/dist/2020.js";

const parse = async (path) => JSON.parse(await readFile(new URL(path, import.meta.url), "utf8"));
const schema = await parse("../schemas/zryna-diagnostics-v2.schema.json");
const golden = await parse("../crates/zryna-diagnostics/src/protocol_v2/fixtures/golden.json");
const exhausted = await parse("../crates/zryna-diagnostics/src/protocol_v2/fixtures/exhausted.json");
const hostile = await parse("../crates/zryna-diagnostics/src/protocol_v2/fixtures/hostile.json");
const validate = new Ajv2020({ strict: true, allErrors: true }).compile(schema);

test("closed diagnostic v2 schema accepts independent golden and terminal records", () => {
  for (const document of [golden, exhausted, { schema_version: 2, diagnostics: [] }]) {
    assert.equal(validate(document), true, JSON.stringify(validate.errors));
  }
  assert.equal(schema.properties.schema_version.const, 2);
  assert.equal(schema.properties.diagnostics.maxItems, 256);
  assert.equal(schema.$defs.path.maxLength, 1024);
  assert.equal(schema.$defs.text.maxLength, 4096);
});

for (const fixture of hostile) {
  test(`independent hostile diagnostic shape: ${fixture.name}`, () => {
    const document = structuredClone(golden);
    const keys = fixture.pointer.slice(1).split("/");
    const key = keys.pop();
    const object = keys.reduce((current, part) => current[part], document);
    if (fixture.remove) delete object[key];
    else object[key] = fixture.value;
    assert.equal(validate(document), !fixture.schemaRejects, JSON.stringify(validate.errors));
  });
}

test("every required field rejects omission and every closed object rejects extras", () => {
  const objects = [
    [schema, (document) => document],
    [schema.$defs.diagnostic, (document) => document.diagnostics[0]],
    ...schema.$defs.location.oneOf.map((shape, index) => [shape, (document) => document.diagnostics[index].location]),
  ];
  for (const [shape, select] of objects) {
    for (const key of shape.required) {
      const document = structuredClone(golden);
      delete select(document)[key];
      assert.equal(validate(document), false, `missing ${key}`);
    }
    const document = structuredClone(golden);
    select(document).unexpected = false;
    assert.equal(validate(document), false, "extra field");
  }
});

test("schema accepts exact and rejects first-extra collection/string limits", () => {
  const document = { schema_version: 2, diagnostics: Array.from({ length: 256 }, () => structuredClone(golden.diagnostics[0])) };
  assert.equal(validate(document), true);
  document.diagnostics.push(structuredClone(golden.diagnostics[0]));
  assert.equal(validate(document), false);
  for (const field of ["message", "guidance", "path"]) {
    const document = structuredClone(golden);
    const object = field === "path" ? document.diagnostics[1].location : document.diagnostics[0];
    const limit = field === "path" ? 1024 : 4096;
    object[field] = "a".repeat(limit);
    assert.equal(validate(document), true);
    object[field] += "a";
    assert.equal(validate(document), false);
  }
});

test("schema numeric bounds and Unicode lengths do not claim source/byte authority", () => {
  const document = structuredClone(golden);
  document.diagnostics[2].location.byte_end = 4294967295;
  document.diagnostics[0].message = "😀".repeat(4096);
  assert.equal(validate(document), true);
  // The Rust validator rejects these source/UTF-8 byte budgets independently.
  document.diagnostics[2].location.byte_end += 1;
  assert.equal(validate(document), false);
});
