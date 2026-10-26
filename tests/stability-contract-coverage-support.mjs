// Unit-test data helpers. No process, guest, receipt or qualification authority.
import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { MAX_MATERIAL_BYTES, validateDeclaredCoverage, loadDeclaredCoverage } from '../scripts/stability-gates/contract-coverage.mjs';
import { readSafe } from '../scripts/stability-gates/input.mjs';
export const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
export const source = process.env.ZRYNA_COVERAGE_UNIT_SOURCE_ROOT ?? root;
export const pristine = () => loadDeclaredCoverage(root);
export const row = (document, id) => document.rows.find(item => item.id === id);
export const bytes = path => readSafe(source, path, MAX_MATERIAL_BYTES);
export function rejects(mutation) {
  const candidate = pristine();
  mutation(candidate);
  assert.throws(() => validateDeclaredCoverage(candidate));
  assert.equal(validateDeclaredCoverage(pristine()).proofQualified, false, 'pristine declaration retry');
}
