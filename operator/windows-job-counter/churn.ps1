param([Parameter(Mandatory=$true)][string]$Root)
$ErrorActionPreference = 'Stop'
. (Join-Path $Root 'owner-windows-job.instrumented.ps1')
$job = [IntPtr]::Zero; $disarmed = $false; $held = [Collections.Generic.List[object]]::new()
$clock = [Diagnostics.Stopwatch]::StartNew(); $deadline = 15000L
$result = @{ diagnosticOnly=$true; acceptance=$false; requestedTargets=64; actualStartedTargets=0; membershipQueries=0; cleanupConfirmed=$false }
try {
  $job = [M6OwnerJob]::Create()
  $owner = [Diagnostics.Process]::GetCurrentProcess()
  [M6OwnerJob]::Assign($job,$owner.SafeHandle.DangerousGetHandle())
  $command = Join-Path $env:SystemRoot 'System32/cmd.exe'
  for ($i=0; $i -lt 64; $i++) {
    if ($clock.ElapsedMilliseconds -ge $deadline) { throw 'Original finite diagnostic churn deadline.' }
    $start = [Diagnostics.ProcessStartInfo]::new($command)
    $start.UseShellExecute=$false; $start.CreateNoWindow=$true
    $start.ArgumentList.Add('/d'); $start.ArgumentList.Add('/c'); $start.ArgumentList.Add('exit /b 0')
    $child = [Diagnostics.Process]::Start($start)
    $held.Add(@{ process=$child; handle=$child.SafeHandle }); $result.actualStartedTargets++
    $members=@([M6OwnerJob]::Members($job)); $result.membershipQueries++
    if (-not $members.Contains([uint32]$PID)) { throw 'Original owner-membership requirement.' }
  }
  foreach ($item in $held) {
    while (-not $item.process.HasExited) {
      if ($clock.ElapsedMilliseconds -ge $deadline) { throw 'Diagnostic child disappearance unconfirmed.' }
      [Threading.Thread]::Sleep(1)
    }
  }
  $members=@([M6OwnerJob]::Members($job)); $result.membershipQueries++
  if ($members.Count -ne 1 -or $members[0] -ne $PID) { throw 'Original owner-only final inventory requirement.' }
  [M6OwnerJob]::KillOnClose($job,$false); $disarmed=$true
  $result.cleanupConfirmed=$true; $result.status='bounded-native-churn-completed-no-editor-acceptance'
} catch {
  $result.status='bounded-native-churn-failed'; $result.failure=$_.Exception.ToString()
  $result.nativeFailure=@{ message=$_.Exception.Message; scriptLine=$_.InvocationInfo.ScriptLineNumber; stack=$_.ScriptStackTrace }
} finally {
  foreach ($item in $held) {
    try {
      if (-not [object]::ReferenceEquals($item.handle,$item.process.SafeHandle)) { throw 'Retained handle changed.' }
      if (-not $item.process.HasExited) { $item.process.Kill($false) }
    } catch { $result.cleanupFailure=$_.Exception.ToString(); $result.cleanupConfirmed=$false }
  }
  $result.elapsedMs=$clock.ElapsedMilliseconds
  [IO.File]::WriteAllText((Join-Path $Root 'CHURN_RESULT.json'),($result|ConvertTo-Json -Depth 8))
  foreach ($item in $held) { $item.process.Dispose() }
  if ($disarmed) { $null=[M6OwnerJob]::CloseHandle($job) }
  # Unknown keeps original kill-on-close armed through process exit; it is never a cleanup pass.
}
if ($result.cleanupConfirmed -and $result.status -eq 'bounded-native-churn-completed-no-editor-acceptance') { exit 0 }
exit 70
