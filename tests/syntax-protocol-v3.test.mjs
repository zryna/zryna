import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import Ajv2020 from "ajv/dist/2020.js";

const parse = async (url) => JSON.parse(await readFile(url, "utf8"));

test("protocol-v3 schema accepts its golden fixture and seals all variants", async () => {
  const schema = await parse(new URL("../schemas/zryna-syntax-v3.schema.json", import.meta.url));
  const fixture = await parse(new URL("./fixtures/syntax-v3-valid.json", import.meta.url));
  const validate = new Ajv2020({ allErrors: true, strict: true }).compile(schema);
  assert.equal(validate(fixture), true, JSON.stringify(validate.errors));
  assert.equal(schema.properties.schema_version.const, 3);
  assert.deepEqual(
    schema.$defs.statementKind.oneOf.map((entry) => entry.properties.kind.const),
    ["local-declaration", "assignment", "return", "block", "if", "while"],
  );
  assert.deepEqual(schema.$defs.binaryKind.properties.kind.enum, [
    "addition", "subtraction", "multiplication", "equal", "not-equal",
    "less-than", "less-equal", "greater-than", "greater-equal",
  ]);
});

test("protocol-v3 nested records are closed and bounds are schema-visible", async () => {
  const schema = await parse(new URL("../schemas/zryna-syntax-v3.schema.json", import.meta.url));
  const fixture = await parse(new URL("./fixtures/syntax-v3-valid.json", import.meta.url));
  const validate = new Ajv2020({ allErrors: true, strict: true }).compile(schema);
  const unknown = structuredClone(fixture);
  unknown.files[0].functions[0].body.blocks[0].unknown = true;
  assert.equal(validate(unknown), false);
  const tooMany = structuredClone(fixture);
  tooMany.files[0].functions[0].body.blocks[0].statements = Array(4097).fill(0);
  assert.equal(validate(tooMany), false);
});

test("module specifier grammar matches the canonical explicit-relative Rust boundary", async () => {
  const schema = await parse(new URL("../schemas/zryna-syntax-v3.schema.json", import.meta.url));
  const fixture = await parse(
    new URL("./fixtures/typescript-adapter-v3-result.json", import.meta.url),
  );
  const validate = new Ajv2020({ allErrors: true, strict: true }).compile(schema);
  const accepts = [
    "./value.zry",
    "../value.zry",
    "../../shared/value.zry",
    "./nested/../value.zry",
    "./.zry",
    "./a:b.zry",
  ];
  const rejects = [
    "value.zry",
    "/value.zry",
    "./value.ts",
    "./value.ZRY",
    "./",
    "../",
    "./a//value.zry",
    "./value.zry?query",
    "./value.zry#fragment",
    "./https://host/value.zry",
    "./bad\\value.zry",
    "./bad\0value.zry",
    "./café.zry",
    `${"../".repeat(341)}x.zry`,
  ];
  for (const text of accepts) {
    const candidate = structuredClone(fixture);
    candidate.files[0].imports[0].specifier.text = text;
    assert.equal(validate(candidate), true, `${text}: ${JSON.stringify(validate.errors)}`);
  }
  for (const text of rejects) {
    const candidate = structuredClone(fixture);
    candidate.files[0].imports[0].specifier.text = text;
    assert.equal(validate(candidate), false, text);
  }
});

test("diagnostic message and guidance are non-empty and bounded", async () => {
  const schema = await parse(new URL("../schemas/zryna-syntax-v3.schema.json", import.meta.url));
  const fixture = await parse(new URL("./fixtures/syntax-v3-valid.json", import.meta.url));
  const validate = new Ajv2020({ allErrors: true, strict: true }).compile(schema);
  const diagnostic = {
    code: "ZRYNA-F2002",
    severity: "error",
    location: { kind: "global" },
    message: "unsupported syntax",
    guidance: "use the protocol-v3 subset",
  };
  const valid = structuredClone(fixture);
  valid.diagnostics = [diagnostic];
  assert.equal(validate(valid), true, JSON.stringify(validate.errors));
  for (const field of ["message", "guidance"]) {
    const empty = structuredClone(valid);
    empty.diagnostics[0][field] = "";
    assert.equal(validate(empty), false, `${field} accepted empty text`);
    const exact = structuredClone(valid);
    exact.diagnostics[0][field] = "x".repeat(4096);
    assert.equal(validate(exact), true, `${field}: ${JSON.stringify(validate.errors)}`);
    const overflow = structuredClone(valid);
    overflow.diagnostics[0][field] = "x".repeat(4097);
    assert.equal(validate(overflow), false, `${field} accepted 4097 scalars`);
  }
});
