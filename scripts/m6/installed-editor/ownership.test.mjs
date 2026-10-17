import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, realpath, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { performance } from 'node:perf_hooks';
import test from 'node:test';
import { launchOwned } from './owner-client.mjs';

async function fixture(t) {
  const stage = await mkdtemp(join(await realpath(tmpdir()), 'm6-process-owner-unit-'));
  const output = join(stage, 'records');
  await mkdir(output);
  t.after(() => rm(stage, { recursive: true, force: true }));
  return { stage, output, cwd: stage, env: { ...process.env } };
}

test('natural root exit still requires retained-handle cleanup of its detached child', async t => {
  const options = await fixture(t);
  const source = `const {spawn}=require('node:child_process');
    const child=spawn(process.execPath,['-e','setInterval(()=>{},1000)'],
      {detached:true,stdio:'ignore'});
    console.log(child.pid);child.unref();setTimeout(()=>process.exit(0),1200);`;
  const owner = await launchOwned(process.execPath, ['-e', source],
    // Windows cold CLR compilation and full dependency pinning precede target release.
    // The helper's original15s cleanup and the separate3s timeout negative remain.
    { ...options, name: 'detached-child',
      deadline: performance.now() + (process.platform === 'win32' ? 30000 : 15000) });
  const result = await owner.finish();
  const childPid = Number(result.stdout.trim());
  assert(Number.isSafeInteger(childPid) && childPid > 0);
  const record = JSON.parse(await readFile(join(options.output, 'detached-child.json'), 'utf8'));
  assert.equal(record.cleanupConfirmed, true);
  assert.equal(record.complete.cleanupConfirmed, true);
  assert(record.events.some(event => event.kind === 'identity' && event.pid === childPid));
  assert(record.events.some(event => event.kind === 'gone' && event.pid === childPid));
  assert.equal(record.ownerExit.status, 0);
  assert(record.pins.dependencies.length > 0);
  assert((await readFile(join(options.output, 'detached-child.before-start.json'))).length > 0);
});

test('absolute timeout fails the command while retaining actual process cleanup facts', async t => {
  const options = await fixture(t);
  const owner = await launchOwned(process.execPath,
    ['-e', "process.on('SIGTERM',()=>{});setInterval(()=>{},1000)"],
    { ...options, name: 'timeout', deadline: performance.now() + 3000 });
  await assert.rejects(owner.finish());
  const record = JSON.parse(await readFile(join(options.output, 'timeout.json'), 'utf8'));
  assert.equal(record.cleanupConfirmed, false);
  assert(record.failure);
  assert(record.events.some(event => event.kind === 'identity'));
  // A deadline failure is never converted into a passing receipt, even if the actual
  // retained-handle owner can positively confirm subsequent cleanup.
  if (record.complete?.cleanupConfirmed === true) {
    assert(record.events.some(event => event.kind === 'gone' && event.pid === record.rootPid));
  }
});


test('creation-owned immediate root exit retains natural status0 and confirmed cleanup', async t => {
  const options = await fixture(t);
  const owner = await launchOwned(process.execPath,
    ['-e', "require('node:fs').writeSync(1,'immediate-root\\n')"],
    { ...options, name: 'fast-root', deadline: performance.now() + 15000 });
  const result = await owner.finish();
  assert.equal(result.stdout, 'immediate-root\n');
  const record = JSON.parse(await readFile(join(options.output, 'fast-root.json'), 'utf8'));
  assert.equal(record.complete.status, 0);
  assert.equal(record.ownerExit.status, 0);
  assert.equal(record.cleanupConfirmed, true);
  assert(record.events.some(event => event.kind === 'gone' && event.pid === record.rootPid));
  if (process.platform === 'win32') {
    const roots = record.events.filter(event => event.kind === 'identity' && event.pid === record.rootPid);
    assert(roots.some(event => event.reason === 'created-root' && event.requestedFileName === record.config.program));
    for (const row of roots.filter(event => event.imageUnavailable === 'exited-before-image-query')) {
      assert.equal(row.image, null);
      assert(row.start && row.exitTicks);
    }
  }
});

// Linux-only subreaper regression is not registered or credited as a Windows case/skip.
if (process.platform === 'linux') {
  test('Linux immediate parent exit still cleans its detached adopted child', async t => {
    const options = await fixture(t);
    const source = `const {spawn}=require('node:child_process');
      const child=spawn(process.execPath,['-e','setInterval(()=>{},1000)'],
        {detached:true,stdio:'ignore'});
      require('node:fs').writeSync(1,String(child.pid)+'\\n');child.unref();process.exit(0);`;
    const owner = await launchOwned(process.execPath, ['-e', source],
      { ...options, name: 'fast-adoption', deadline: performance.now() + 15000 });
    const result = await owner.finish();
    const childPid = Number(result.stdout.trim());
    assert(Number.isSafeInteger(childPid) && childPid > 0);
    const record = JSON.parse(await readFile(join(options.output, 'fast-adoption.json'), 'utf8'));
    assert.equal(record.complete.status, 0);
    assert.equal(record.cleanupConfirmed, true);
    assert(record.events.some(event => event.pid === childPid &&
      ((event.kind === 'identity' && event.reason === 'subreaper-child') ||
       (event.kind === 'parent' && event.parent === record.supervisorPid))));
    assert(record.events.some(event => event.kind === 'gone' && event.pid === childPid));
  });
}

// The private job, rather than a sampled parent PID, grants signal authority.
if (process.platform === 'win32') {
  test('Windows private job rejects a live foreign peer and confirms its own exited member', async t => {
    const options = await fixture(t);
    const source = await readFile(new URL('./owner-windows.ps1', import.meta.url), 'utf8');
    const boundary = source.indexOf('\ntry {\n  # One reader owns all control bytes');
    assert(boundary > 0);
    const helper = join(options.stage, 'native-job-control.ps1');
    await writeFile(join(options.stage, 'owner-windows-job.ps1'),
      await readFile(new URL('./owner-windows-job.ps1', import.meta.url)), { flag: 'wx' });
    await writeFile(helper, source.slice(0, boundary) + `
$probeJob = [M6OwnerJob]::Create()
$child = $null; $owned = $null
try {
  $foreignRefused = $false
  try { $unexpected = [M6OwnerJob]::OpenMember($probeJob, $PID); $unexpected.Dispose() } catch {
    if ($_.Exception.Message -notmatch 'outside the private job') { throw }; $foreignRefused = $true
  }
  if (-not $foreignRefused) { throw 'Live foreign process admitted.' }
  $info = [Diagnostics.ProcessStartInfo]::new(); $info.FileName = $args[0]; $info.UseShellExecute = $false
  $info.ArgumentList.Add('-e'); $info.ArgumentList.Add('setTimeout(()=>{},2000)')
  $child = [Diagnostics.Process]::new(); $child.StartInfo = $info
  if (-not $child.Start()) { throw 'Native control child unavailable.' }
  $held = $child.SafeHandle
  [M6OwnerJob]::Assign($probeJob, $held.DangerousGetHandle())
  $owned = [M6OwnerJob]::OpenMember($probeJob, $child.Id)
  if (-not [M6OwnerJob]::Contains($probeJob, $owned.SafeHandle.DangerousGetHandle())) { throw 'Actual private member missing.' }
  $members = @([M6OwnerJob]::Members($probeJob))
  if ($members.Count -ne 1 -or $members[0] -ne $child.Id) { throw 'Exact native job membership differs.' }
  $liveMemberRefused = $false
  try { $null = ConfirmJobTermination $probeJob $child.Id } catch {
    if ($_.Exception.Message -notmatch 'not positively terminated') { throw }; $liveMemberRefused = $true
  }
  if (-not $liveMemberRefused) { throw 'Live owned child admitted as terminated.' }
  if (-not $child.WaitForExit(5000) -or -not $owned.HasExited) { throw 'Native control child did not exit.' }
  $proof = ConfirmJobTermination $probeJob $child.Id
  if ($proof.state -ne 'terminated' -or $proof.method -ne 'WaitForSingleObject') { throw 'Exited held identity not confirmed.' }
  [Console]::Out.WriteLine((ConvertTo-Json -Compress @{foreignRefused=$foreignRefused;liveMemberRefused=$liveMemberRefused;exitedConfirmed=$true;exactMembership=$true}))
} finally {
  if ($owned) { $owned.Dispose() }
  if ($child) { $child.Dispose() }
  $null = [M6OwnerJob]::CloseHandle($probeJob)
}
`, { flag: 'wx' });
    const runtime = join(process.env.ProgramFiles, 'PowerShell', '7', 'pwsh.exe');
    const owner = await launchOwned(runtime, ['-NoLogo', '-NoProfile', '-NonInteractive', '-File', helper, process.execPath],
      { ...options, name: 'native-job', deadline: performance.now() + 30000 });
    const result = await owner.finish();
    assert.deepEqual(JSON.parse(result.stdout.trim()), { foreignRefused: true, liveMemberRefused: true,
      exitedConfirmed: true, exactMembership: true });
  });
}
