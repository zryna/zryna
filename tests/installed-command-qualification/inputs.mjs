import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';

export function writePrivateFile(output, name, document) {
  const path = join(output, `${name}.json`);
  const text = typeof document === 'string' ? document : JSON.stringify(document);
  if (process.platform !== 'win32') writeFileSync(path, text, { flag: 'wx', mode: 0o600 });
  else {
    const script = `$ErrorActionPreference = 'Stop'
$path = $env:ZRYNA_PRIVATE_PATH
$bytes = [Text.Encoding]::UTF8.GetBytes($env:ZRYNA_PRIVATE_TEXT)
$user = [System.Security.Principal.WindowsIdentity]::GetCurrent().User
$acl = [System.Security.AccessControl.FileSecurity]::new()
$acl.SetOwner($user)
$acl.SetAccessRuleProtection($true, $false)
foreach ($sid in @($user, [System.Security.Principal.SecurityIdentifier]::new('S-1-5-18'))) {
 $acl.AddAccessRule([System.Security.AccessControl.FileSystemAccessRule]::new($sid,'FullControl','Allow'))
}
$file = [IO.FileStream]::new($path, [IO.FileMode]::CreateNew, [Security.AccessControl.FileSystemRights]::Write, [IO.FileShare]::None, 4096, [IO.FileOptions]::None, $acl)
try { $file.Write($bytes, 0, $bytes.Length) } finally { $file.Dispose() }`;
    const result = spawnSync('powershell.exe', ['-NoLogo', '-NoProfile', '-NonInteractive', '-Command', script],
      { env: { ...process.env, ZRYNA_PRIVATE_PATH: path, ZRYNA_PRIVATE_TEXT: text }, encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr);
  }
  return ['--grant-file', path];
}
export function request(value, key = 'MODE', capability = 'environment') {
  return { schema: 'zryna.wasi-command-request.v1', world: 'zryna:capability-profiles/command@0.1.0',
    grant: { capability, key }, input: value === undefined ? { present: false } : { present: true, value } };
}
