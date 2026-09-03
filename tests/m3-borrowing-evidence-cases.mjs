import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { expectedRegistrySha256 } from "../scripts/check-m3-contract.mjs";

const root = new URL("../", import.meta.url);
const read = file => readFileSync(new URL(file, root), "utf8");
const document = read("docs/M3_BORROWING_SEMANTICS.md");

function checkNamedEvidence(text) {
  const section = text.split("### Named closure evidence\n")[1]?.split("## Issue #113 evidence")[0];
  assert(section, "named closure evidence section is missing");
  let checked = 0;
  function check(file, names) {
    const source = read(file);
    const tests = new Set([...source.matchAll(
      /#\[test\]\s*(?:#\[[^\]]*\]\s*)*fn\s+([a-z][a-z0-9_]+)\s*\(/g,
    )].map(match => match[1]));
    for (const name of names) {
      assert(tests.has(name), `${file}: missing test ${name}`);
      checked++;
    }
  }
  const rows = section.split("\n").filter(line => /^\|.*\(`.*\.rs`\)/.test(line));
  assert.equal(rows.length, 7, "semantic evidence rows changed without review");
  for (const row of rows) {
    const tokens = [...row.matchAll(/`([^`]+)`/g)].map(match => match[1]);
    assert.match(tokens[0], /^[a-z_]+\.rs$/);
    check(`crates/zryna-semantics/src/data_ownership_v1/tests/${tokens[0]}`, tokens.slice(1));
  }
  assert(section.includes("Issue #248 adds"), "authenticated resource evidence is missing");
  const ir = section.split("The independent verifier's tests")[1]?.split("Issue #248 adds")[0];
  assert(ir, "independent verifier evidence is missing");
  check("crates/zryna-ir/src/data_ownership_v1/tests.rs",
    [...ir.matchAll(/`([a-z][a-z_]+)`/g)].map(match => match[1]));
  const limits = section.split("Issue #248 adds")[1]?.split("These tests distinguish")[0];
  assert(limits, "authenticated resource evidence is missing");
  check("crates/zryna-ir/src/data_ownership_v1/tests/borrow_resource_boundaries.rs",
    [...limits.matchAll(/`([a-z][a-z_]+)`/g)].map(match => match[1]));
  assert.equal(checked, 34, "named evidence inventory changed without review");
}

test("borrowing documentation resolves named evidence to actual Rust test functions", () => {
  checkNamedEvidence(document);
});

test("borrowing evidence guard rejects missing tests and removed proof sections", () => {
  assert.throws(() => checkNamedEvidence(document.replace(
    "`shared_root_aliases_read_copy_values_end_in_reverse_and_restore_owner_access`",
    "`missing_borrow_test`",
  )), /missing test missing_borrow_test/);
  assert.throws(() => checkNamedEvidence(document.replace("### Named closure evidence", "### Removed")),
    /section is missing/);
  assert.throws(() => checkNamedEvidence(document.replace("Issue #248 adds", "Removed resource proof")),
    /authenticated resource evidence is missing/);
  checkNamedEvidence(document);
});

test("current M3 registry provenance remains explicit alongside historical evidence", () => {
  for (const file of ["docs/ROADMAP.md", "docs/STATUS.md"])
    assert(read(file).includes(`\`${expectedRegistrySha256}\``), `${file}: current digest missing`);
  assert(document.includes("d61d1ec50005bbed7d86f029fa6ece5efa7517d495b6aed6e9b0f1c15f69e20f"));
  assert(document.includes("ca7ca013771f8ebb0ddc3f7791bc46db6378892e89f3e8e570a44e42e687fc20"));
});
