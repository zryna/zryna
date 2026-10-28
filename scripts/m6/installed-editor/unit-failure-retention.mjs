// Private unit diagnostics only: copied bytes never prove process cleanup or host qualification.
import assert from 'node:assert/strict';
import { constants } from 'node:fs';
import { lstat, mkdtemp, open } from 'node:fs/promises';
import { dirname, isAbsolute, join, resolve } from 'node:path';
import { capture, descriptor, ordinaryRoot } from './files.mjs';
const MIB = 1048576;
const MANIFEST_BYTES = 262144;
function records(name) {
  return [
    [`${name}.stdout`, `${name}.stdout`, MIB],
    [`${name}.stderr`, `${name}.stderr`, MIB],
    [`${name}.owner.jsonl`, `${name}.owner.jsonl`, 4 * MIB],
    [`${name}.owner.stderr`, `${name}.owner.stderr`, MIB],
    [`${name}.json`, `${name}.json`, 16 * MIB],
    [`${name}.before-start.json`, `${name}.before-start.json`, 16 * MIB],
    [`${name}.owner-material/owner.ps1`, 'owner.ps1', 65536],
    [`${name}.owner-material/owner-windows-job.ps1`, 'owner-windows-job.ps1', 65536],
  ];
}
export async function retainUnitFailure({ output, name, parent, deadline, error }) {
  assert(typeof name === 'string' && /^[a-z][a-z0-9-]{0,63}$/u.test(name));
  for (const path of [output, parent]) assert(isAbsolute(path) && path === resolve(path));
  assert(Number.isFinite(deadline), 'existing absolute retention/cleanup deadline required');
  await ordinaryRoot(output); await ordinaryRoot(parent);
  const directory = await mkdtemp(join(parent, 'm6-installed-editor-unit-failure-'));
  await ordinaryRoot(directory);
  const entries = [];
  for (const [source, destination, maximumBytes] of records(name)) {
    const path = join(output, source);
    try {
      // Distinguish actual absence from identity, size, expiry and capture failures.
      try { await ordinaryRoot(dirname(path)); await lstat(path); } catch (failure) {
        if (failure.code === 'ENOENT') {
          entries.push({ source, destination, maximumBytes, state: 'absent' }); continue;
        }
        throw failure;
      }
      const expected = await descriptor(path, deadline, maximumBytes);
      const copied = await capture(path, join(directory, destination), expected, deadline);
      entries.push({ source, destination, maximumBytes, state: 'captured',
        bytes: copied.source.bytes, sha256: copied.source.sha256, originalIdentity: copied.source.identity });
    } catch (failure) {
      entries.push({ source, destination, maximumBytes, state: 'refused',
        error: String(failure.message ?? failure).slice(0, 2048) });
    }
  }
  const complete = entries.every(row => row.state !== 'refused');
  const record = { format: 'zryna.installed-editor-unit-failure.v1', ownerName: name,
    originalOutput: output, primaryError: String(error?.stack ?? error).slice(0, 4096),
    retentionComplete: complete, entries, originalStagePreserved: true,
    cleanupConfirmed: false, cleanupClaim: 'No process state inferred by diagnostic copying.',
    sourceAuthenticated: false, proofQualified: false, baselineApproved: false,
    thresholds: null, closureStatus: 'blocked' };
  const bytes = Buffer.from(`${JSON.stringify(record, null, 2)}\n`);
  assert(bytes.length <= MANIFEST_BYTES, 'bounded failure manifest');
  const file = await open(join(directory, 'failure.json'), constants.O_WRONLY | constants.O_CREAT
    | constants.O_EXCL | (constants.O_NOFOLLOW ?? 0), 0o600);
  try { await file.writeFile(bytes); await file.sync(); } finally { await file.close(); }
  return { directory, complete };
}
