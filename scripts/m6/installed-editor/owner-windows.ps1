# Trusted test-only owner. This process creates the root and retains all signal handles.
$ErrorActionPreference = 'Stop'
$known = @{}
$rootProcess = $null
$job = [IntPtr]::Zero; $jobDisarmed = $false
$completed = $false
$controlInput = $null; $controlReader = $null; $stopRead = $null
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
  if (-not [M6OwnerJob]::Contains($job, $handle.DangerousGetHandle())) { throw 'Retained process is outside the private job.' }
  [long]$creation = 0; [long]$exitTime = 0; [long]$kernel = 0; [long]$user = 0
  if (-not [M6OwnerJob]::GetProcessTimes($handle.DangerousGetHandle(), [ref]$creation,
      [ref]$exitTime, [ref]$kernel, [ref]$user)) { throw 'Retained creation time unavailable.' }
  $image = $null; $queryError = $null
  try { $image = $process.MainModule.FileName } catch {
    # Image-query readiness does not grant ownership. The same retained handle,
    # native creation time and private job membership already establish it.
    $queryError = DiagnosticText $_.Exception.ToString() 1024
  }
  return @{ pid = $process.Id; start = [DateTime]::FromFileTimeUtc($creation).Ticks.ToString();
    image = $image; handle = $handle; process = $process; gone = $false; imageQueryError = $queryError;
    imageUnavailable = $(if (-not $image) { 'private-job-member-image-not-yet-observed' } else { $null }) }
}
function Public($item) {
  return @{ pid = $item.pid; start = $item.start; image = $item.image;
    parent = $item.parent; reason = $item.reason; gone = $item.gone;
    requestedFileName = $item.requestedFileName; imageUnavailable = $item.imageUnavailable;
    exitTicks = $item.exitTicks; terminationProof = $item.terminationProof;
    imageQueryError = $item.imageQueryError }
}
function Retain($process, [string]$reason, [int]$parent, $item) {
  if ($known.ContainsKey($item.pid)) {
    $old = $known[$item.pid]
    if ($old.start -eq $item.start) { $process.Dispose(); return }
    if (-not $old.gone) { throw 'Owned PID reused before disappearance proof.' }
    if ($old.process) { $old.process.Dispose() }
  }
  if ($known.Count -ge 256) { throw 'Owned identity bound exceeded.' }
  $item.reason = $reason; $item.parent = $parent
  $known[$item.pid] = $item
  Emit 'identity' (Public $item)
}
. (Join-Path $PSScriptRoot 'owner-windows-job.ps1')
function Observe([string]$stage, [long]$deadline) {
  if ($clock.ElapsedMilliseconds -ge $deadline) { throw 'Process observation deadline.' }
  $members = @([M6OwnerJob]::Members($job))
  if (-not $members.Contains([uint32]$PID)) { throw 'Owner private job membership lost.' }
  foreach ($pidValue in $members) {
    if ($clock.ElapsedMilliseconds -ge $deadline) { throw 'Process observation deadline.' }
    if ($pidValue -eq $PID) { continue }
    if ($known.ContainsKey([int]$pidValue) -and $known[[int]$pidValue].process) { continue }
    $process = $null
    try {
      $process = [Diagnostics.Process]::GetProcessById([int]$pidValue)
      $probe = Stamp $process
    } catch {
      # The private job selected this PID. Opening a replacement outside that job
      # cannot acquire signal authority; only positive termination is admitted.
      $proof = ConfirmJobTermination $job ([int]$pidValue)
      if ($process) { $process.Dispose() }
      RetainJobTermination ([int]$pidValue) $proof
      continue
    }
    Retain $process 'private-job-member' 0 $probe
  }
  foreach ($item in $known.Values) {
    if ($item.gone) { continue }
    if ($item.process.HasExited) {
      # HasExited polls the retained handle; no PID reopen is used for this decision.
      $item.gone = $true; Emit 'gone' (Public $item); continue
    }
    try { $current = Stamp $item.process } catch {
      if (-not $item.process.HasExited -or
          -not [object]::ReferenceEquals($item.handle, $item.process.SafeHandle)) { throw }
      $item.exitTicks = $item.process.ExitTime.ToUniversalTime().Ticks.ToString()
      $item.imageUnavailable = 'retained-identity-exited-during-refresh'
      $item.gone = $true; Emit 'gone' (Public $item); continue
    }
    if ($current.start -ne $item.start) { throw 'Owned creation identity changed.' }
    if (-not $item.image -and $current.image) {
      $item.image = $current.image; $item.imageUnavailable = $null
      Emit 'image' (Public $item)
    } elseif ($item.image -and $current.image -and -not [string]::Equals(
        $current.image, $item.image, [StringComparison]::OrdinalIgnoreCase)) {
      throw 'Owned executing image changed.'
    }
    if ($current.imageQueryError -and -not $item.imageQueryError) {
      $item.imageQueryError = $current.imageQueryError
      Emit 'image' (Public $item)
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
function NewRead($stream) {
  return @{ stream = $stream; buffer = [byte[]]::new(65536); task = $null; done = $false; size = 0 }
}
function DiagnosticText([string]$value, [int]$maximum) {
  if ($null -eq $value) { return $null }
  return $value.Substring(0, [Math]::Min($maximum, $value.Length))
}
try {
  # One reader owns all control bytes, including any read-ahead from config/start.
  $controlInput = [Console]::OpenStandardInput()
  $controlReader = [IO.StreamReader]::new(
    $controlInput, [Text.UTF8Encoding]::new($false, $true), $false, 4096, $true)
  $config = ConvertFrom-Json -AsHashtable -InputObject ($controlReader.ReadLine())
  $expected = @('args','cleanupMs','cwd','program','stage','timeoutMs')
  if ((($config.Keys | Sort-Object) -join ',') -ne ($expected -join ',')) { throw 'Closed owner config required.' }
  foreach ($key in @('program','cwd','stage')) {
    if ($config[$key] -isnot [string] -or $config[$key].Length -gt 32768 -or
        -not [IO.Path]::IsPathFullyQualified($config[$key])) { throw 'Absolute owner paths required.' }
  }
  if ($config.args.Count -gt 256 -or $config.timeoutMs -lt 1 -or $config.timeoutMs -gt 600000 -or
      $config.cleanupMs -lt 1 -or $config.cleanupMs -gt 15000) { throw 'Bounded owner config required.' }
  $stage = [IO.Path]::GetFullPath($config.stage).TrimEnd('\')
  $self = [Diagnostics.Process]::GetCurrentProcess()
  $job = [M6OwnerJob]::Create()
  [M6OwnerJob]::Assign($job, $self.SafeHandle.DangerousGetHandle())
  $dependencies = @($self.Modules | ForEach-Object { $_.FileName }) +
    @([AppDomain]::CurrentDomain.GetAssemblies() | Where-Object { -not $_.IsDynamic } |
      ForEach-Object { $_.Location }) + @((Get-Module).Path)
  $dependencies = @($dependencies | Where-Object { $_ } | Sort-Object -Unique)
  Emit 'ready' @{ dependencies = $dependencies; runtime = (Get-Process -Id $PID).Path; pid = $PID;
    powershell = $PSVersionTable.PSVersion.ToString(); dotnet = [Environment]::Version.ToString();
    ownership = 'private-inherited-job-before-target-creation'; breakaway = $false; killOnClose = $true }
  if ($controlReader.ReadLine() -ne '{"op":"start"}') { throw 'Explicit owner start required.' }
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
  $stopRead = $controlReader.ReadLineAsync()
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
          $members = @([M6OwnerJob]::Members($job))
          if ($members.Count -ne 1 -or $members[0] -ne $PID) { throw 'Private job still has descendants.' }
          [M6OwnerJob]::KillOnClose($job, $false); $jobDisarmed = $true
          Emit 'complete' @{ status = $rootStatus; cleanupConfirmed = $true; failures = @($failures);
            privateJobDescendants = 0; ownerOnlyConfirmed = $true }
          $completed = $true; break
        }
      } else { $quiet = $null }
      if ($now -ge $cleanup) { throw 'Owned process disappearance unconfirmed.' }
    }
    [Threading.Thread]::Sleep(25)
  }
  if ($rootStatus -eq 0 -and $failures.Count -eq 0) { exit 0 } else { exit 1 }
} catch {
  # Preserve the ORIGINAL ErrorRecord before KillHeld can replace PowerShell's $_.
  $originalFailure = $_
  $nativeFailure = $null
  try {
    $nativeFailure = @{
      exceptionType = (DiagnosticText $originalFailure.Exception.GetType().FullName 256)
      message = (DiagnosticText $originalFailure.Exception.Message 1024)
      fullyQualifiedErrorId = (DiagnosticText $originalFailure.FullyQualifiedErrorId 512)
      scriptLineNumber = $originalFailure.InvocationInfo.ScriptLineNumber
      offsetInLine = $originalFailure.InvocationInfo.OffsetInLine
      scriptStackTrace = (DiagnosticText $originalFailure.ScriptStackTrace 2048)
    }
  } catch { $nativeFailure = @{ diagnosticUnavailable = $true } }
  $failures.Add('Owner failed; detailed native failure is unavailable or retained in stderr.')
  try { KillHeld } catch { $failures.Add('Retained-handle kill failed.') }
  Emit 'unknown' @{ failures = @($failures); identities = @($known.Values | ForEach-Object { Public $_ }); nativeFailure = $nativeFailure }
  exit 70
} finally {
  # Unknown exit retains kill-on-close. Kernel handle rundown then terminates the
  # private job; unknown is never promoted to an observed successful cleanup.
  if ($jobDisarmed) { $null = [M6OwnerJob]::CloseHandle($job) }
  if (-not $completed -and $rootProcess) {
    try { $rootProcess.StandardOutput.Dispose(); $rootProcess.StandardError.Dispose() } catch {}
  }
  foreach ($item in $known.Values) { if ($item.process) { $item.process.Dispose() } }
  # Never dispose an active read or wait for control input during process cleanup.
  # A still-pending .NET worker read and its wrappers end with this owner process.
  if ($null -eq $stopRead -or $stopRead.IsCompleted) {
    try { if ($controlReader) { $controlReader.Dispose() } } catch { }
    try { if ($controlInput) { $controlInput.Dispose() } } catch { }
  }
}
