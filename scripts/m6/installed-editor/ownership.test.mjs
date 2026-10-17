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
    { ...options, name: 'detached-child', deadline: performance.now() + 15000 });
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

// Windows native termination evidence is distinct from managed query failure.
if (process.platform === 'win32') {
  test('Windows CIM termination proof rejects live peers and wrong epochs while confirming an exited handle', async t => {
    const options = await fixture(t);
    const source = await readFile(new URL('./owner-windows.ps1', import.meta.url), 'utf8');
    const boundary = source.indexOf('\ntry {\n  # One reader owns all control bytes');
    assert(boundary > 0);
    const helper = join(options.stage, 'native-termination-control.ps1');
    await writeFile(join(options.stage, 'owner-windows-termination.ps1'),
      await readFile(new URL('./owner-windows-termination.ps1', import.meta.url)), { flag: 'wx' });
    await writeFile(helper, source.slice(0, boundary) + `
$live = Get-CimInstance -ClassName Win32_Process -Filter "ProcessId=$PID" -OperationTimeoutSec 1
$liveRefused = $false
try { $null = ConfirmCimTermination $live } catch {
  if ($_.Exception.Message -notmatch 'not positively terminated') { throw }; $liveRefused = $true
}
if (-not $liveRefused) { throw 'Live peer admitted as terminated.' }
$info = [Diagnostics.ProcessStartInfo]::new(); $info.FileName = $args[0]; $info.UseShellExecute = $false
$info.ArgumentList.Add('-e'); $info.ArgumentList.Add('setTimeout(()=>{},2000)')
$child = [Diagnostics.Process]::new(); $child.StartInfo = $info
try {
  if (-not $child.Start()) { throw 'Native control child unavailable.' }
  $held = $child.SafeHandle
  $row = Get-CimInstance -ClassName Win32_Process -Filter ("ProcessId="+$child.Id) -OperationTimeoutSec 1
  if (-not $row) { throw 'Actual control child identity missing.' }
  if (-not $child.WaitForExit(5000)) { throw 'Native control child did not exit.' }
  $proof = ConfirmCimTermination $row
  if ($proof.state -ne 'terminated' -or $proof.method -ne 'WaitForSingleObject') { throw 'Exited held identity not confirmed.' }
  $wrong = @{ProcessId=$row.ProcessId;CreationDate=$row.CreationDate.AddSeconds(1);ExecutablePath=$row.ExecutablePath}
  $epochRefused = $false
  try { $null = ConfirmCimTermination $wrong } catch {
    if ($_.Exception.Message -notmatch 'epoch changed') { throw }; $epochRefused = $true
  }
  if (-not $epochRefused) { throw 'Wrong creation epoch admitted.' }
  $incompleteRefused = $false
  try { $null = ConfirmCimTermination @{ProcessId=$PID;CreationDate=$null;ExecutablePath=$null} } catch {
    if ($_.Exception.Message -notmatch 'Complete attributed CIM identity') { throw }; $incompleteRefused = $true
  }
  [Console]::Out.WriteLine((ConvertTo-Json -Compress @{liveRefused=$liveRefused;exitedConfirmed=$true;epochRefused=$epochRefused;incompleteRefused=$incompleteRefused}))
} finally { $child.Dispose() }
`, { flag: 'wx' });
    const runtime = join(process.env.ProgramFiles, 'PowerShell', '7', 'pwsh.exe');
    const owner = await launchOwned(runtime, ['-NoLogo', '-NoProfile', '-NonInteractive', '-File', helper, process.execPath],
      { ...options, name: 'native-termination', deadline: performance.now() + 30000 });
    const result = await owner.finish();
    assert.deepEqual(JSON.parse(result.stdout.trim()), { liveRefused: true, exitedConfirmed: true,
      epochRefused: true, incompleteRefused: true });
  });
}
