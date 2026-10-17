import assert from 'node:assert/strict';

// Pure extraction used by the actual recorder preflight. It creates no execution authority.
function select(input, { packageId, target, kind, sourcePath, manifestPath }, testProfile) {
  assert(Buffer.isBuffer(input) && input.length > 0 && input.length <= 4194304);
  const rows = new TextDecoder('utf-8', { fatal: true }).decode(input).split(/\r?\n/)
    .filter(Boolean).map(line => JSON.parse(line));
  const finished = rows.filter(row => row.reason === 'build-finished');
  assert(finished.length === 1 && finished[0].success === true);
  const matches = rows.filter(row => row.reason === 'compiler-artifact'
    && row.package_id === packageId && row.target?.name === target
    && JSON.stringify(row.target.kind) === JSON.stringify([kind])
    && row.target.src_path === sourcePath && row.manifest_path === manifestPath
    && row.profile?.test === testProfile);
  assert.equal(matches.length, 1, 'Unique exact Cargo test artifact required.');
  assert(typeof matches[0].executable === 'string' && matches[0].executable.length > 0);
  return matches[0].executable;
}
export const selectHarness = (input, expected) => select(input, expected, true);
export const selectAuxiliary = (input, expected) => select(input, expected, false);
