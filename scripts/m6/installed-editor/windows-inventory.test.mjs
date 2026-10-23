// Windows-native control only; pure decoded rows never grant job membership or signal authority.
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, realpath, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { performance } from 'node:perf_hooks';
import test from 'node:test';
import { launchOwned } from './owner-client.mjs';
if (process.platform === 'win32') {
  test('Windows changing inventory cannot prove cleanup; bounded exact native inventory recovers', async t => {
    const stage = await mkdtemp(join(await realpath(tmpdir()), 'm6-job-inventory-unit-'));
    t.after(() => rm(stage, { recursive: true, force: true }));
    const output = join(stage, 'records');
    await mkdir(output);
    await writeFile(join(stage, 'owner-windows-job.ps1'),
      await readFile(new URL('./owner-windows-job.ps1', import.meta.url)), { flag: 'wx' });
    const helper = join(stage, 'inventory-control.ps1');
    await writeFile(helper, `
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'owner-windows-job.ps1')
function MustReject([scriptblock]$body) {
  $rejected = $false
  try { $null = & $body } catch { $rejected = $true }
  if (-not $rejected) { throw 'Malformed or incomplete inventory was admitted.' }
}
$job = [M6OwnerJob]::Create(); $child = $null
try {
  $info = [Diagnostics.ProcessStartInfo]::new(); $info.FileName = $args[0]; $info.UseShellExecute = $false
  $info.ArgumentList.Add('-e'); $info.ArgumentList.Add('setInterval(()=>{},1000)')
  $child = [Diagnostics.Process]::new(); $child.StartInfo = $info
  if (-not $child.Start()) { throw 'Actual native member unavailable.' }
  $held = $child.SafeHandle
  [M6OwnerJob]::Assign($job, $held.DangerousGetHandle())
  $actual = [M6OwnerJob]::Snapshot($job)
  if (-not $actual.Complete -or $actual.ProcessIds.Count -ne 1 -or $actual.ProcessIds[0] -ne $child.Id) {
    throw 'Exact native job inventory differs.'
  }
  # Simulate count churn with an actual selected member ID; no native ownership is inferred here.
  $changing = [M6OwnerJob]::DecodeMembers(2, [uint32[]]@($child.Id))
  if ($changing.Complete -or $changing.Assigned -ne 2 -or $changing.Listed -ne 1) { throw 'Changing snapshot claimed completeness.' }
  MustReject { $changing.RequireComplete() }
  $maximum = [M6OwnerJob]::DecodeMembers(257, [uint32[]](1..257))
  if ($maximum.RequireComplete().Count -ne 257) { throw 'Original exact owner+256 bound changed.' }
  MustReject { [M6OwnerJob]::DecodeMembers(258, [uint32[]](1..258)) }
  MustReject { [M6OwnerJob]::DecodeMembers(0, [uint32[]]@()) }
  MustReject { [M6OwnerJob]::DecodeMembers(1, [uint32[]]@(1,2)) }
  MustReject { [M6OwnerJob]::DecodeMembers(2, [uint32[]]@(1,1)) }
  MustReject { [M6OwnerJob]::DecodeMembers(1, [uint32[]]@(0)) }
  MustReject { [M6OwnerJob]::DecodeMembers(1, [uint32[]]@(2147483648)) }
  $again = @([M6OwnerJob]::Members($job))
  if ($again.Count -ne 1 -or $again[0] -ne $child.Id -or $child.HasExited) { throw 'Exact native retry lost the live member.' }
  if (-not [M6OwnerJob]::Contains($job, $held.DangerousGetHandle())) { throw 'Retained same-job membership missing.' }
  $child.Kill($false)
  if (-not $child.WaitForExit(5000)) { throw 'Retained native member did not terminate.' }
  [Console]::Out.WriteLine('native-live-inventory;changing-not-complete;malformed-refused;exact-bound;retained-cleanup')
} finally {
  if ($child) { $child.Dispose() }
  $null = [M6OwnerJob]::CloseHandle($job)
}
`, { flag: 'wx' });
    const runtime = join(process.env.ProgramFiles, 'PowerShell', '7', 'pwsh.exe');
    const owner = await launchOwned(runtime,
      ['-NoLogo', '-NoProfile', '-NonInteractive', '-File', helper, process.execPath],
      { stage, output, cwd: stage, env: { ...process.env }, name: 'job-inventory',
        deadline: performance.now() + 30000 });
    const result = await owner.finish();
    assert.equal(result.stdout.trim(),
      'native-live-inventory;changing-not-complete;malformed-refused;exact-bound;retained-cleanup');
    const record = JSON.parse(await readFile(join(output, 'job-inventory.json'), 'utf8'));
    assert.equal(record.cleanupConfirmed, true);
    assert.equal(record.complete.ownerOnlyConfirmed, true);
    assert.equal(record.complete.privateJobDescendants, 0);
  });
}
