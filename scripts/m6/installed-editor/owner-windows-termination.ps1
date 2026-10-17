# A CIM snapshot can outlive a short-lived child. Establish termination with a
# fresh OS handle or ERROR_INVALID_PARAMETER, never by treating a query failure
# as disappearance. These calls cannot signal any process.
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class M6OwnerTermination {
  [DllImport("kernel32.dll", SetLastError=true)]
  public static extern IntPtr OpenProcess(uint access, bool inherit, uint pid);
  [DllImport("kernel32.dll", SetLastError=true)]
  public static extern bool CloseHandle(IntPtr handle);
  [DllImport("kernel32.dll", SetLastError=true)]
  public static extern uint WaitForSingleObject(IntPtr handle, uint milliseconds);
  [DllImport("kernel32.dll", SetLastError=true)]
  public static extern bool GetProcessTimes(IntPtr handle, out long creation,
    out long exit, out long kernel, out long user);
}
'@
function ConfirmCimTermination($row) {
  if ([int]$row.ProcessId -le 0) {
    throw 'Complete attributed CIM identity required for termination proof.'
  }
  $probe = [M6OwnerTermination]::OpenProcess(0x00101000, $false, [uint32]$row.ProcessId)
  if ($probe -eq [IntPtr]::Zero) {
    $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
    if ($code -ne 87) { throw "Attributed process termination unavailable: OpenProcess=$code." }
    return @{ method = 'OpenProcess'; win32Error = 87; state = 'absent' }
  }
  try {
    if (-not $row.CreationDate) { throw 'Complete attributed CIM identity required for termination proof.' }
    [long]$creation = 0; [long]$exitTime = 0; [long]$kernel = 0; [long]$user = 0
    if (-not [M6OwnerTermination]::GetProcessTimes($probe, [ref]$creation,
        [ref]$exitTime, [ref]$kernel, [ref]$user)) { throw 'Attributed process creation time unavailable.' }
    $expected = $row.CreationDate.ToUniversalTime().ToFileTimeUtc()
    if (($creation - ($creation % 10)) -ne ($expected - ($expected % 10))) {
      throw 'Attributed process epoch changed before termination proof.'
    }
    if ([M6OwnerTermination]::WaitForSingleObject($probe, 0) -ne 0) {
      throw 'Attributed process is not positively terminated.'
    }
    return @{ method = 'WaitForSingleObject'; state = 'terminated'; creationFileTime = $creation.ToString() }
  } finally { $null = [M6OwnerTermination]::CloseHandle($probe) }
}
function RetainCimTermination($row, [int]$parent, $proof) {
  $pidValue = [int]$row.ProcessId
  if ($known.ContainsKey($pidValue)) {
    if (-not $known[$pidValue].gone) { throw 'Owned PID reused before disappearance proof.' }
    if ($known[$pidValue].process) { $known[$pidValue].process.Dispose() }
  }
  if ($known.Count -ge 256) { throw 'Owned identity bound exceeded.' }
  $item = @{ pid = $pidValue; start = $(if ($row.CreationDate) { $row.CreationDate.ToUniversalTime().Ticks.ToString() } else { $null });
    image = $row.ExecutablePath; parent = $parent; reason = 'cim-owned-terminated-before-retention';
    process = $null; handle = $null; gone = $true; terminationProof = $proof;
    imageUnavailable = $(if (-not $row.ExecutablePath) { 'cim-image-unavailable-positive-termination' } else { $null }) }
  $known[$pidValue] = $item
  Emit 'identity' (Public $item); Emit 'gone' (Public $item)
}
