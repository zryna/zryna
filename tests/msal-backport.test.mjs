import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { auditMaterials, root } from './msal-backport/materials.mjs';

test('reviewed MSAL6 authCode backport preserves all eleven network-denied CJS/ESM controls', async t => {
  const before = auditMaterials();
  for (const item of before.variance) t.diagnostic(JSON.stringify(item));
  const home = mkdtempSync(join(tmpdir(), 'zryna-msal-backport-'));
  t.after(() => rmSync(home, { recursive: true, force: false }));
  const child = spawnSync(process.execPath, [
    '--require', fileURLToPath(new URL('./msal-backport/network-deny.cjs', import.meta.url)),
    fileURLToPath(new URL('./msal-backport/probe.mjs', import.meta.url)),
  ], {
    cwd: root, shell: false, windowsHide: true, encoding: 'utf8', timeout: 10000, maxBuffer: 64 * 1024,
    // Deliberately do not read/spread process.env, real credentials, proxies or CLI configuration.
    env: { HOME: home, USERPROFILE: home, APPDATA: home, LOCALAPPDATA: home,
      TMP: home, TEMP: home, TMPDIR: home, CI: '1' },
  });
  if (child.error || child.status !== 0 || child.signal) t.diagnostic(JSON.stringify({
    status: child.status, signal: child.signal, error: child.error?.message, stdout: child.stdout, stderr: child.stderr,
  }));
  assert.ifError(child.error);
  assert.equal(child.signal, null, child.stderr);
  assert.equal(child.stderr, '', 'SDK diagnostics are retained, not silently suppressed');
  const stdout = child.stdout.replaceAll('\r\n', '\n');
  assert(stdout.startsWith('TAP version 13\n'), stdout);
  const rows = [...stdout.matchAll(/^(ok|not ok) (\d+) - (.+)$/gmu)];
  assert.equal(rows.length, 11, stdout);
  assert.deepEqual(rows.map(row => Number(row[2])), Array.from({ length: 11 }, (_, index) => index + 1));
  assert.deepEqual(rows.map(row => row[3]), before.provenance.expectedCaseNames);
  for (const row of rows) await t.test(row[3], () => assert.equal(row[1], 'ok', stdout));
  assert.equal(child.status, 0, stdout);
  assert.match(stdout, /^1\.\.11$/mu); assert.match(stdout, /^# pass 11$/mu); assert.match(stdout, /^# fail 0$/mu);
  const after = auditMaterials();
  assert.equal(after.packageRoot, before.packageRoot, 'same transitive SDK package location after probe');
  assert.deepEqual(after.variance, before.variance, 'same reviewed installed materials after probe');
  t.diagnostic('one fresh-env SDK child executed exactly eleven controls; no credentialed publishing or source authentication');
});
