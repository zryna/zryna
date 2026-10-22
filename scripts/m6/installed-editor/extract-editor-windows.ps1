# Trusted harness-only extraction of the already authenticated official Windows ZIP.
$ErrorActionPreference = 'Stop'
if ($args.Count -ne 2) { throw 'Exactly archive and destination arguments required.' }
foreach ($value in $args) {
  if ($value -isnot [string] -or $value.Length -gt 32768 -or $value.Contains([char]0) -or
      -not [IO.Path]::IsPathFullyQualified($value) -or
      -not [string]::Equals([IO.Path]::GetFullPath($value), $value, [StringComparison]::Ordinal)) {
    throw 'Canonical absolute extraction arguments required.'
  }
}
$archive = $args[0]; $destination = $args[1]
$parent = [IO.Path]::GetDirectoryName($archive)
if (-not [string]::Equals($archive, [IO.Path]::Combine($parent, 'authenticated-editor.archive'),
      [StringComparison]::Ordinal) -or
    -not [string]::Equals($destination, [IO.Path]::Combine($parent, 'editor'),
      [StringComparison]::Ordinal)) { throw 'Fixed same-stage editor extraction paths required.' }
function OrdinaryParents([string]$path) {
  $directory = [IO.DirectoryInfo]::new($path)
  while ($null -ne $directory) {
    if (-not $directory.Exists -or
        ($directory.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
      throw 'Ordinary extraction directory ancestry required.'
    }
    $directory = $directory.Parent
  }
}
# Node ordinaryRoot has already checked physical canonical spelling and created editor exclusively.
OrdinaryParents $destination
$file = [IO.FileInfo]::new($archive)
if (-not $file.Exists -or $file.Length -le 0 -or
    ($file.Attributes -band ([IO.FileAttributes]::ReparsePoint -bor [IO.FileAttributes]::Directory)) -ne 0) {
  throw 'Ordinary authenticated editor archive required.'
}
foreach ($entry in [IO.Directory]::EnumerateFileSystemEntries($destination)) {
  throw 'Fresh empty editor destination required.'
}
[IO.Compression.ZipFile]::ExtractToDirectory($archive, $destination, $false)
