# Trusted private job: the owner joins BEFORE creating the target. Descendants
# inherit membership; no breakaway flags are enabled. No numeric-PID signalling.
Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
public static class M6OwnerJob {
  [StructLayout(LayoutKind.Sequential)] public struct Basic {
    public long ProcessTime, JobTime; public uint Flags;
    public UIntPtr MinWorkingSet, MaxWorkingSet; public uint ActiveLimit;
    public UIntPtr Affinity; public uint Priority, Scheduling;
  }
  [StructLayout(LayoutKind.Sequential)] public struct Counters {
    public ulong ReadOps, WriteOps, OtherOps, ReadBytes, WriteBytes, OtherBytes;
  }
  [StructLayout(LayoutKind.Sequential)] public struct Extended {
    public Basic Limits; public Counters Io;
    public UIntPtr ProcessMemory, JobMemory, PeakProcessMemory, PeakJobMemory;
  }
  [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
  static extern IntPtr CreateJobObject(IntPtr security, string name);
  [DllImport("kernel32.dll", SetLastError=true)]
  static extern bool SetInformationJobObject(IntPtr job, int kind, ref Extended info, uint bytes);
  [DllImport("kernel32.dll", SetLastError=true)]
  static extern bool QueryInformationJobObject(IntPtr job, int kind, IntPtr info, uint bytes, out uint returned);
  [DllImport("kernel32.dll", SetLastError=true)]
  static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);
  [DllImport("kernel32.dll", SetLastError=true)]
  static extern bool IsProcessInJob(IntPtr process, IntPtr job, out bool member);
  [DllImport("kernel32.dll", SetLastError=true)]
  public static extern IntPtr OpenProcess(uint access, bool inherit, uint pid);
  [DllImport("kernel32.dll", SetLastError=true)]
  public static extern bool CloseHandle(IntPtr handle);
  [DllImport("kernel32.dll", SetLastError=true)]
  public static extern uint WaitForSingleObject(IntPtr handle, uint milliseconds);
  [DllImport("kernel32.dll", SetLastError=true)]
  public static extern bool GetProcessTimes(IntPtr handle, out long creation, out long exit, out long kernel, out long user);
  [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
  static extern bool QueryFullProcessImageName(IntPtr process, uint flags, StringBuilder name, ref uint size);
  [DllImport("kernel32.dll", SetLastError=true)]
  public static extern bool TerminateProcess(IntPtr process, uint code);
  public static string Image(IntPtr process) {
    var name = new StringBuilder(32768); uint size = (uint)name.Capacity;
    if (!QueryFullProcessImageName(process, 0, name, ref size)) throw new Win32Exception();
    return name.ToString();
  }
  public static M6OwnedProcess OpenMember(IntPtr job, int pid) {
    // Query-limited + synchronize + terminate; no VM-read/all-access request.
    var raw = OpenProcess(0x00101001, false, (uint)pid);
    if (raw == IntPtr.Zero) throw new Win32Exception();
    var handle = new M6RetainedHandle(raw);
    try {
      if (!Contains(job, raw)) throw new InvalidOperationException("Opened process is outside the private job.");
      return new M6OwnedProcess(pid, handle);
    } catch { handle.Dispose(); throw; }
  }
  public static void KillOnClose(IntPtr job, bool enabled) {
    var info = new Extended(); info.Limits.Flags = enabled ? 0x2000u : 0;
    if (!SetInformationJobObject(job, 9, ref info, (uint)Marshal.SizeOf<Extended>())) throw new Win32Exception();
  }
  public static IntPtr Create() {
    var job = CreateJobObject(IntPtr.Zero, null);
    if (job == IntPtr.Zero) throw new Win32Exception();
    try { KillOnClose(job, true); return job; } catch { CloseHandle(job); throw; }
  }
  public static void Assign(IntPtr job, IntPtr process) {
    if (!AssignProcessToJobObject(job, process)) throw new Win32Exception();
  }
  public static bool Contains(IntPtr job, IntPtr process) {
    bool member; if (!IsProcessInJob(process, job, out member)) throw new Win32Exception(); return member;
  }
  public static uint[] Members(IntPtr job) {
    const int maximum = 257; // Original256 target identities plus this owner.
    int bytes = 8 + maximum * IntPtr.Size; var memory = Marshal.AllocHGlobal(bytes);
    try {
      uint returned;
      if (!QueryInformationJobObject(job, 3, memory, (uint)bytes, out returned)) throw new Win32Exception();
      int assigned = Marshal.ReadInt32(memory), count = Marshal.ReadInt32(memory, 4);
      if (count < 1 || count > maximum || assigned != count) throw new InvalidOperationException("Bounded complete private job inventory required.");
      var result = new uint[count]; var unique = new System.Collections.Generic.HashSet<uint>();
      for (int i=0;i<count;i++) {
        ulong pid = (ulong)Marshal.ReadIntPtr(memory, 8+i*IntPtr.Size).ToInt64();
        if (pid == 0 || pid > Int32.MaxValue || !unique.Add((uint)pid)) throw new InvalidOperationException("Private job PID identity invalid.");
        result[i] = (uint)pid;
      }
      return result;
    } finally { Marshal.FreeHGlobal(memory); }
  }
}
public sealed class M6RetainedHandle : SafeHandle {
  public M6RetainedHandle(IntPtr value) : base(IntPtr.Zero, true) { SetHandle(value); }
  public override bool IsInvalid { get { return handle == IntPtr.Zero || handle == new IntPtr(-1); } }
  protected override bool ReleaseHandle() { return M6OwnerJob.CloseHandle(handle); }
}
public sealed class M6ProcessImage { public string FileName { get; set; } }
public sealed class M6OwnedProcess : IDisposable {
  public int Id { get; private set; }
  public M6RetainedHandle SafeHandle { get; private set; }
  public M6OwnedProcess(int pid, M6RetainedHandle handle) { Id=pid; SafeHandle=handle; }
  public bool HasExited {
    get {
      uint result=M6OwnerJob.WaitForSingleObject(SafeHandle.DangerousGetHandle(),0);
      if (result==0) return true; if (result==258) return false; throw new Win32Exception();
    }
  }
  public DateTime ExitTime {
    get {
      long creation, exit, kernel, user;
      if (!M6OwnerJob.GetProcessTimes(SafeHandle.DangerousGetHandle(),out creation,out exit,out kernel,out user)) throw new Win32Exception();
      return DateTime.FromFileTimeUtc(exit);
    }
  }
  public M6ProcessImage MainModule { get { return new M6ProcessImage { FileName=M6OwnerJob.Image(SafeHandle.DangerousGetHandle()) }; } }
  public void Kill(bool entireTree) {
    if (entireTree) throw new InvalidOperationException("Only retained-handle signalling is permitted.");
    if (!M6OwnerJob.TerminateProcess(SafeHandle.DangerousGetHandle(),0xffffffff)) throw new Win32Exception();
  }
  public void Dispose() { SafeHandle.Dispose(); }
}
'@
function ConfirmJobTermination([IntPtr]$job, [int]$pidValue) {
  if ($pidValue -le 0) { throw 'Positive selected job PID required.' }
  $probe = [M6OwnerJob]::OpenProcess(0x00101000, $false, [uint32]$pidValue)
  if ($probe -eq [IntPtr]::Zero) {
    $code = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
    if ($code -ne 87) { throw "Private job member termination unavailable: OpenProcess=$code." }
    return @{ method = 'OpenProcess'; win32Error = 87; state = 'absent'; start = $null }
  }
  try {
    if (-not [M6OwnerJob]::Contains($job, $probe)) { throw 'Opened process is outside the private job.' }
    if ([M6OwnerJob]::WaitForSingleObject($probe, 0) -ne 0) { throw 'Private job member is not positively terminated.' }
    [long]$creation = 0; [long]$exitTime = 0; [long]$kernel = 0; [long]$user = 0
    if (-not [M6OwnerJob]::GetProcessTimes($probe, [ref]$creation,
        [ref]$exitTime, [ref]$kernel, [ref]$user)) { throw 'Private job member creation time unavailable.' }
    return @{ method = 'WaitForSingleObject'; state = 'terminated';
      start = [DateTime]::FromFileTimeUtc($creation).Ticks.ToString() }
  } finally { $null = [M6OwnerJob]::CloseHandle($probe) }
}
function RetainJobTermination([int]$pidValue, $proof) {
  if ($known.ContainsKey($pidValue)) {
    if (-not $known[$pidValue].gone) { throw 'Owned PID reused before disappearance proof.' }
    if ($known[$pidValue].process) { $known[$pidValue].process.Dispose() }
  }
  if ($known.Count -ge 256) { throw 'Owned identity bound exceeded.' }
  $item = @{ pid = $pidValue; start = $proof.start; image = $null; parent = $null;
    reason = 'private-job-member-terminated-before-metadata'; process = $null; handle = $null;
    gone = $true; terminationProof = $proof; imageUnavailable = 'positive-job-member-termination' }
  $known[$pidValue] = $item
  Emit 'identity' (Public $item); Emit 'gone' (Public $item)
}
