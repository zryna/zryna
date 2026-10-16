# Trusted test-only owner. This process creates the root and retains all signal handles.
$ErrorActionPreference = 'Stop'
$known = @{}
$rootProcess = $null
$completed = $false
$failures = [System.Collections.Generic.List[string]]::new()
$clock = [System.Diagnostics.Stopwatch]::StartNew()
function Emit([string]$kind, [hashtable]$data) {
  $data.kind = $kind
  [Console]::Out.WriteLine((ConvertTo-Json -InputObject $data -Compress -Depth 12))
  [Console]::Out.Flush()
}
function Stamp($process) {
  # Acquire and hold the same cached Process SafeHandle BEFORE identity checks.
  $handle = $process.SafeHandle
  if ($handle.IsInvalid -or $handle.IsClosed) { throw 'Invalid retained process handle.' }
  return @{ pid = $process.Id; start = $process.StartTime.ToUniversalTime().Ticks.ToString();
    image = $process.MainModule.FileName; handle = $handle; process = $process; gone = $false }
}
function Public($item) {
  return @{ pid = $item.pid; start = $item.start; image = $item.image;
    parent = $item.parent; reason = $item.reason; gone = $item.gone;
    requestedFileName = $item.requestedFileName; imageUnavailable = $item.imageUnavailable;
    exitTicks = $item.exitTicks }
}
function Retain($process, [string]$reason, [int]$parent) {
  $item = Stamp $process
  if ($known.ContainsKey($item.pid)) {
    $old = $known[$item.pid]
    if ($old.start -eq $item.start) { $process.Dispose(); return }
    if (-not $old.gone) { throw 'Owned PID reused before disappearance proof.' }
    $old.process.Dispose()
  }
  if ($known.Count -ge 256) { throw 'Owned identity bound exceeded.' }
  $item.reason = $reason; $item.parent = $parent
  $known[$item.pid] = $item
  Emit 'identity' (Public $item)
}
function SameCimTime($date, [string]$start) {
  if (-not $date) { return $false }
  $ticks = $date.ToUniversalTime().Ticks
  $held = [long]::Parse($start)
  return ($ticks - ($ticks % 10)) -eq ($held - ($held % 10))
}
function Observe([string]$stage, [long]$deadline) {
  if ($clock.ElapsedMilliseconds -ge $deadline) { throw 'Process observation deadline.' }
  # Fixed local provider; no credentials, remote machine or arbitrary query.
  $rows = @(Get-CimInstance -ClassName Win32_Process -OperationTimeoutSec 1 |
    Select-Object -First 4097 ProcessId, ParentProcessId, CreationDate, ExecutablePath)
  if ($rows.Count -gt 4096) { throw 'Process inventory bound exceeded.' }
  $byId = @{}
  foreach ($row in $rows) { $byId[[int]$row.ProcessId] = $row }
  $changed = $true
  while ($changed) {
    $changed = $false
    foreach ($row in $rows) {
      if ($clock.ElapsedMilliseconds -ge $deadline) { throw 'Process observation deadline.' }
      $pidValue = [int]$row.ProcessId; $parent = [int]$row.ParentProcessId
      if ($pidValue -eq $PID) { continue }
      if ($known.ContainsKey($pidValue) -and
          (SameCimTime $row.CreationDate $known[$pidValue].start)) { continue }
      $imageOwned = $row.ExecutablePath -and $row.ExecutablePath.StartsWith(
        $stage + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)
      $parentOwned = $known.ContainsKey($parent) -and -not $known[$parent].gone -and
        $byId.ContainsKey($parent) -and $byId[$parent].CreationDate -and
        (SameCimTime $byId[$parent].CreationDate $known[$parent].start)
      if (-not ($imageOwned -or $parentOwned)) { continue }
      try {
        $process = [Diagnostics.Process]::GetProcessById($pidValue)
        $probe = Stamp $process
        if (-not $row.CreationDate -or
            -not (SameCimTime $row.CreationDate $probe.start) -or
            -not $row.ExecutablePath -or
            -not [string]::Equals($probe.image, $row.ExecutablePath, [StringComparison]::OrdinalIgnoreCase)) {
          $process.Dispose(); throw 'Owned observation identity changed.'
        }
        Retain $process $(if ($parentOwned) { 'owned-parent' } else { 'private-stage-image' }) $parent
        $changed = $true
      } catch [ArgumentException] { throw 'Possibly owned process vanished before handle capture.' }
    }
  }
  foreach ($item in $known.Values) {
    if ($item.gone) { continue }
    if ($item.process.HasExited) {
      # HasExited polls the retained handle; no PID reopen is used for this decision.
      $item.gone = $true; Emit 'gone' (Public $item); continue
    }
    try { $current = Stamp $item.process } catch {
      if ($item.reason -ne 'created-root' -or -not $item.process.HasExited -or
          -not [object]::ReferenceEquals($item.handle, $item.process.SafeHandle)) { throw }
      $item.exitTicks = $item.process.ExitTime.ToUniversalTime().Ticks.ToString()
      $item.imageUnavailable = 'creation-root-exited-during-refresh'
      $item.gone = $true; Emit 'gone' (Public $item); continue
    }
    if ($current.start -ne $item.start -or
        -not [string]::Equals($current.image, $item.image, [StringComparison]::OrdinalIgnoreCase)) {
      throw 'Owned executing identity changed.'
    }
  }
}
function KillHeld {
  foreach ($item in $known.Values) {
    if (-not $item.gone -and -not $item.process.HasExited) {
      # Kill(false) is required to use this Process object's cached retained SafeHandle.
      # No entireProcessTree overload, numeric-PID kill, taskkill, or Stop-Process fallback.
      if ($null -eq $item.handle -and $item.reason -eq 'created-root') {
        $item.handle = $item.process.SafeHandle
      }
      if (-not [object]::ReferenceEquals($item.handle, $item.process.SafeHandle)) {
        throw 'Retained signal handle changed.'
      }
      $item.process.Kill($false)
    }
  }
}
# Diagnostic-only stderr; no stdout protocol or execution-policy change.
function TraceStopRead([string]$phase, $task) {
  try {
    $elapsed = $clock.ElapsedMilliseconds
    $readerType = [Console]::In.GetType().FullName
    $dotnet = [Environment]::Version.ToString()
    $taskCompleted = if ($null -eq $task) { $null } else { [bool]$task.IsCompleted }
    if ($readerType.Length -gt 256) { $readerType = $readerType.Substring(0, 256) }
    if ($dotnet.Length -gt 128) { $dotnet = $dotnet.Substring(0, 128) }
    $line = ConvertTo-Json -Compress -Depth 3 -InputObject @{
      diagnosticOnly = $true; qualification = $false; phase = $phase
      elapsedMs = $elapsed; readerType = $readerType; dotnet = $dotnet
      taskIsCompleted = $taskCompleted
    }
    if ($line.Length -le 2048) {
      [Console]::Error.WriteLine('M6_STOP_READ_PHASE ' + $line)
      [Console]::Error.Flush()
    }
  } catch { }
}
function NewRead($stream) {
  return @{ stream = $stream; buffer = [byte[]]::new(65536); task = $null; done = $false; size = 0 }
}
try {
  $config = ConvertFrom-Json -AsHashtable -InputObject ([Console]::In.ReadLine())
  $expected = @('args','cleanupMs','cwd','program','stage','timeoutMs')
  if ((($config.Keys | Sort-Object) -join ',') -ne ($expected -join ',')) { throw 'Closed owner config required.' }
  foreach ($key in @('program','cwd','stage')) {
    if ($config[$key] -isnot [string] -or $config[$key].Length -gt 32768 -or
        -not [IO.Path]::IsPathFullyQualified($config[$key])) { throw 'Absolute owner paths required.' }
  }
  if ($config.args.Count -gt 256 -or $config.timeoutMs -lt 1 -or $config.timeoutMs -gt 600000 -or
      $config.cleanupMs -lt 1 -or $config.cleanupMs -gt 15000) { throw 'Bounded owner config required.' }
  $stage = [IO.Path]::GetFullPath($config.stage).TrimEnd('\')
  $null = Get-CimInstance -ClassName Win32_Process -Filter "ProcessId=$PID" -OperationTimeoutSec 1
  $self = [Diagnostics.Process]::GetCurrentProcess()
  $dependencies = @($self.Modules | ForEach-Object { $_.FileName }) +
    @([AppDomain]::CurrentDomain.GetAssemblies() | Where-Object { -not $_.IsDynamic } |
      ForEach-Object { $_.Location }) + @((Get-Module).Path)
  $dependencies = @($dependencies | Where-Object { $_ } | Sort-Object -Unique)
  Emit 'ready' @{ dependencies = $dependencies; runtime = (Get-Process -Id $PID).Path; pid = $PID;
    powershell = $PSVersionTable.PSVersion.ToString(); dotnet = [Environment]::Version.ToString() }
  if ([Console]::In.ReadLine() -ne '{"op":"start"}') { throw 'Explicit owner start required.' }
  $start = [Diagnostics.ProcessStartInfo]::new()
  $start.FileName = $config.program; $start.WorkingDirectory = $config.cwd
  $start.UseShellExecute = $false; $start.CreateNoWindow = $false
  $start.RedirectStandardOutput = $true; $start.RedirectStandardError = $true
  $start.RedirectStandardInput = $true
  foreach ($arg in $config.args) {
    if ($arg -isnot [string] -or $arg.Length -gt 32768 -or $arg.Contains([char]0)) {
      throw 'Bounded owner arguments required.'
    }
    $start.ArgumentList.Add($arg)
  }
  $rootProcess = [Diagnostics.Process]::new(); $rootProcess.StartInfo = $start
  if (-not $rootProcess.Start()) { throw 'Owned root start failed.' }
  # The creation-owned object is registered before ANY fallible image/time query.
  $created = @{ process = $rootProcess; pid = $rootProcess.Id; handle = $null;
    start = $null; image = $null; gone = $false; reason = 'created-root'; parent = $PID;
    requestedFileName = $config.program; imageUnavailable = 'metadata-not-yet-observed'; exitTicks = $null }
  $known[$created.pid] = $created
  Emit 'identity' (Public $created)
  $created.handle = $rootProcess.SafeHandle
  $created.start = $rootProcess.StartTime.ToUniversalTime().Ticks.ToString()
  try {
    $created.image = $rootProcess.MainModule.FileName
    if ([string]::IsNullOrEmpty($created.image)) { throw 'Root image unavailable.' }
    $created.imageUnavailable = $null
  } catch {
    # Only the original creation-owned object may use this positively-exited fallback.
    if (-not $rootProcess.HasExited -or
        -not [object]::ReferenceEquals($created.handle, $rootProcess.SafeHandle)) { throw }
    $created.image = $null; $created.imageUnavailable = 'exited-before-image-query'
    $created.exitTicks = $rootProcess.ExitTime.ToUniversalTime().Ticks.ToString()
  }
  if ($created.image -and
      -not [string]::Equals($created.image, $config.program, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Created root image differs from selected executable.'
  }
  Emit 'identity' (Public $created)
  $rootProcess.StandardInput.Close()
  Emit 'started' @{ pid = $rootProcess.Id }
  $reads = @((NewRead $rootProcess.StandardOutput.BaseStream), (NewRead $rootProcess.StandardError.BaseStream))
  TraceStopRead 'BEFORE_READ_LINE_ASYNC' $null
  $stopRead = [Console]::In.ReadLineAsync()
  TraceStopRead 'AFTER_READ_LINE_ASYNC' $stopRead
  $deadline = $clock.ElapsedMilliseconds + [long]$config.timeoutMs
  $cleanup = $null; $quiet = $null; $rootStatus = $null
  while ($true) {
    $now = $clock.ElapsedMilliseconds
    if ($now -ge $deadline -and $null -eq $cleanup) {
      $failures.Add('Absolute execution deadline.'); $cleanup = $now + $config.cleanupMs
    }
    Observe $stage $(if ($null -eq $cleanup) { $deadline } else { $cleanup })
    if ($null -eq $rootStatus -and $rootProcess.HasExited) {
      $rootStatus = $rootProcess.ExitCode
      Emit 'root-exit' @{ pid = $rootProcess.Id; status = $rootStatus }
      if ($null -eq $cleanup) { $cleanup = $now + $config.cleanupMs }
    }
    if ($stopRead.IsCompleted -and $null -eq $cleanup) {
      $line = $stopRead.GetAwaiter().GetResult()
      if ($null -ne $line -and $line -ne '{"op":"stop"}') { throw 'Closed owner operation required.' }
      $cleanup = $now + $config.cleanupMs
    }
    for ($index = 0; $index -lt 2; $index++) {
      $read = $reads[$index]
      if ($read.done) { continue }
      if ($null -eq $read.task) { $read.task = $read.stream.ReadAsync($read.buffer, 0, $read.buffer.Length) }
      if (-not $read.task.IsCompleted) { continue }
      $count = $read.task.GetAwaiter().GetResult(); $read.task = $null
      if ($count -eq 0) { $read.done = $true; continue }
      $read.size += $count
      if ($read.size -gt 1048576) { throw 'Owned stream cap exceeded.' }
      Emit 'data' @{ stream = $index; data = [Convert]::ToBase64String($read.buffer, 0, $count) }
    }
    if ($null -ne $cleanup) {
      KillHeld
      $live = @($known.Values | Where-Object { -not $_.gone })
      if ($null -ne $rootStatus -and $live.Count -eq 0 -and $reads[0].done -and $reads[1].done) {
        if ($null -eq $quiet) { $quiet = $now }
        if ($now - $quiet -ge 250) {
          Emit 'complete' @{ status = $rootStatus; cleanupConfirmed = $true; failures = @($failures) }
          $completed = $true; break
        }
      } else { $quiet = $null }
      if ($now -ge $cleanup) { throw 'Owned process disappearance unconfirmed.' }
    }
    [Threading.Thread]::Sleep(25)
  }
  if ($rootStatus -eq 0 -and $failures.Count -eq 0) { exit 0 } else { exit 1 }
} catch {
  $failures.Add('Owner failed; detailed native failure is unavailable or retained in stderr.')
  try { KillHeld } catch { $failures.Add('Retained-handle kill failed.') }
  Emit 'unknown' @{ failures = @($failures); identities = @($known.Values | ForEach-Object { Public $_ }) }
  exit 70
} finally {
  if (-not $completed -and $rootProcess) {
    try { $rootProcess.StandardOutput.Dispose(); $rootProcess.StandardError.Dispose() } catch {}
  }
  foreach ($item in $known.Values) { $item.process.Dispose() }
}
