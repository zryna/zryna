// Finite declared-coverage units; synthetic mutations never count as host/source proof.
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { checkDeclaredMaterials, validateDeclaredCoverage, parseDeclaredCoverage } from '../scripts/stability-gates/contract-coverage.mjs';
import { bytes, pristine, rejects, root, row } from './stability-contract-coverage-support.mjs';

test('complete-current-scope-index', () => {
  const document = pristine();
  const result = validateDeclaredCoverage(document);
  assert.equal(result.rows, 25); assert.equal(result.materials, 184);
  assert.equal(result.closureStatus, 'blocked'); assert.equal(result.proofQualified, false);
  assert(document.rows.every(item => item.evidence_state === 'not-collected-by-this-plan'));
  const material = checkDeclaredMaterials(document, bytes);
  assert.equal(material.files, 184); assert(material.bytes > 0);
  assert.equal(material.sourceAuthenticated, false);
  assert.equal(material.proofQualified, false);
  assert.equal(parseDeclaredCoverage(Buffer.from(canonical(document))).rows.length, 25);
  const raw = readFileSync(`${root}/tests/stability-contract-coverage-v2.json`);
  assert.throws(() => parseDeclaredCoverage(Buffer.concat([raw, Buffer.from(' ')])));
  assert.throws(() => parseDeclaredCoverage(Buffer.from('{"format":"first","format":"second"}\n')));
});
test('legacy-v1-is-immutable', () => {
  const expected = 'f339c9edba6773d63a252908dbfa094875a5ecb7c66843dd8017142f547e787b';
  assert.equal(sha256(bytes('tests/stability-gates-v1.json')), expected);
  const legacy = JSON.parse(bytes('tests/stability-gates-v1.json'));
  assert.equal(legacy.scope, 'current-support-preparation');
  assert.equal(legacy.gates.find(gate => gate.lane === 'performance').thresholds, null);
  assert(bytes('scripts/stability-gates/evidence.mjs').includes(Buffer.from("receipt.closureStatus, 'blocked'")));
  rejects(doc => { doc.historyPreserved.sha256 = '0'.repeat(64); });
  rejects(doc => { doc.historyPreserved.closure_status = 'passed'; });
});
test('missing-advertisement-or-version', () => {
  rejects(doc => { doc.rows.splice(0, 1); });
  rejects(doc => { doc.rows[0].versions.pop(); });
  rejects(doc => { doc.rows[0].advertised_surface = ''; });
  rejects(doc => { doc.rows[0].migration_rule_to_freeze = ''; });
});
test('unknown-extra-and-duplicate-row', () => {
  rejects(doc => { doc.rows.push(structuredClone(doc.rows[0])); });
  rejects(doc => { doc.rows[1] = structuredClone(doc.rows[0]); });
  rejects(doc => { [doc.rows[0], doc.rows[1]] = [doc.rows[1], doc.rows[0]]; });
  rejects(doc => { doc.rows[0].unknown = true; });
  rejects(doc => { doc.materialPins[1] = structuredClone(doc.materialPins[0]); });
  rejects(doc => { doc.materialPins[0].path = '../escape'; });
  const cyclic = pristine(); cyclic.unknown = cyclic;
  assert.throws(() => validateDeclaredCoverage(cyclic));
});
test('changed-authority-or-fixture', () => {
  const document = pristine();
  for (const path of ['docs/CLI.md', 'examples/universal/add.zry']) {
    assert.throws(() => checkDeclaredMaterials(document, name => {
      const original = bytes(name);
      if (name !== path) return original;
      const changed = Buffer.from(original); changed[0] ^= 1; return changed;
    }));
  }
  rejects(doc => { doc.materialPins[0].sha256 = '0'.repeat(64); });
  rejects(doc => { doc.materialPins[0].bytes += 1; });
  rejects(doc => { doc.rows[0].fixture_or_proof_inputs.pop(); });
  assert.equal(checkDeclaredMaterials(pristine(), bytes).proofQualified, false);
});
test('provider-migration-is-scope-aware', () => {
  rejects(doc => { row(doc, 'native-provider-and-fallback').versions[0] = 'bootstrap default'; });
  rejects(doc => { row(doc, 'native-provider-and-fallback').advertised_surface = 'bootstrap is default'; });
  rejects(doc => { row(doc, 'distribution-install-release').versions[0] = 'native-distribution.v2'; });
  rejects(doc => { row(doc, 'native-provider-and-fallback').migration_rule_to_freeze = 'v1 install gains v2 without proof'; });
});
test('m7-does-not-widen-legacy', () => {
  rejects(doc => { doc.rows[0].versions = row(doc, 'm7-bounded-generics-candidate').versions; });
  rejects(doc => { row(doc, 'm7-bounded-generics-candidate').explicit_exclusions = ['none']; });
  rejects(doc => { row(doc, 'm7-bounded-generics-candidate').fixture_or_proof_inputs.pop(); });
  rejects(doc => { row(doc, 'm7-bounded-generics-candidate').advertised_surface = 'templates are scalar host exports'; });
});
test('restricted-api-is-not-cli', () => {
  for (const id of ['native-c-restricted-typed-api', 'native-compile-recipe-typed-api', 'package-build-cache-typed-api']) {
    rejects(doc => { row(doc, id).advertised_surface = 'public CLI selector'; });
    rejects(doc => { row(doc, id).platforms = ['Windows native execution']; });
    rejects(doc => { row(doc, id).explicit_exclusions = ['none']; });
  }
});

test('merged-prerequisite-reference-never-authenticates-candidate', () => {
  const document = pristine();
  assert.equal(document.sourceSnapshot.mergedRevision, '4a5b9d423051e78766899d1d5f46ef3f4e0f39d6');
  assert.equal(document.sourceSnapshot.mergedTree, 'f4908f76574c405b68be39a65ccb74ca36c3fefa');
  assert.equal(document.sourceSnapshot.candidateRevision, null);
  assert(row(document, 'm7-bounded-generics-candidate').planning_status.startsWith('#416 implementation merged;'));
  assert.equal(validateDeclaredCoverage(document).sourceAuthenticated, false);
  rejects(doc => { doc.sourceSnapshot.mergedTree = '0'.repeat(40); });
  rejects(doc => { doc.sourceSnapshot.role = 'authenticated-418-candidate'; });
  rejects(doc => { doc.sourceSnapshot.sourceAuthenticated = true; });
  rejects(doc => { doc.sourceSnapshot.refresh = 'merged-once-no-fresh-proof-needed'; });
});
test('extracted-editor-fixtures-retain-byte-bound-coverage', () => {
  const document = pristine();
  const selected = row(document, 'editor-extension').fixture_or_proof_inputs;
  for (const name of ['extension-fixture.cjs', 'extension-responses.test.cjs', 'host-cli.test.cjs']) {
    const path = `editors/vscode-zryna/test/${name}`;
    assert(selected.includes(path));
    assert(document.materialPins.some(pin => pin.path === path));
    assert.throws(() => checkDeclaredMaterials(document, item => {
      const actual = bytes(item);
      if (item !== path) return actual;
      const mutated = Buffer.from(actual); mutated[0] ^= 1; return mutated;
    }));
    rejects(doc => { row(doc, 'editor-extension').fixture_or_proof_inputs = selected.filter(item => item !== path); });
  }
  assert.equal(checkDeclaredMaterials(pristine(), bytes).sourceAuthenticated, false);
});
