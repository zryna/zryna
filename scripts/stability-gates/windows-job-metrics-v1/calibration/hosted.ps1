# Private evidence-only build/control packet. No workload baseline or authentication.
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$evidence = Join-Path $env:RUNNER_TEMP 'windows-job-metrics-calibration'
New-Item -ItemType Directory -Path $evidence -ErrorAction Stop | Out-Null
function Record($name, $value) {
    $path = Join-Path $evidence $name
    $text = ($value | ConvertTo-Json -Depth 12) + "`n"
    $stream = [IO.File]::Open($path, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write)
    try { $bytes = [Text.UTF8Encoding]::new($false).GetBytes($text); $stream.Write($bytes, 0, $bytes.Length) }
    finally { $stream.Dispose() }
}
function Material($path) {
    $file = Get-Item -LiteralPath $path
    if (-not ($file -is [IO.FileInfo]) -or ($file.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "Regular material required: $path" }
    return @{ path = $file.FullName; bytes = $file.Length; sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
}
function Capture($name, $exe, $arguments) {
    Record "$name.command.json" @{ executable = $exe; arguments = @($arguments) }
    & $exe @arguments 1> (Join-Path $evidence "$name.stdout.log") 2> (Join-Path $evidence "$name.stderr.log")
    $code = $LASTEXITCODE
    Record "$name.exit.json" @{ exitCode = $code }
    return $code
}
function Selected($directories, $names) {
    return @($names | ForEach-Object {
        $name = $_; $found = $null
        foreach ($directory in $directories) {
            $candidate = Join-Path $directory $name
            if (Test-Path -LiteralPath $candidate -PathType Leaf) { $found = $candidate; break }
        }
        if (-not $found) { throw "Required selected SDK/CRT material absent: $name" }
        Material $found
    })
}
try {
    $head = git rev-parse HEAD
    if ($LASTEXITCODE -ne 0) { throw 'Cannot resolve checkout HEAD' }
    $tree = git rev-parse 'HEAD^{tree}'
    if ($LASTEXITCODE -ne 0) { throw 'Cannot resolve checkout tree' }
    $dirty = @(git status --porcelain=v1 --untracked-files=all)
    if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect checkout status' }
    Record 'checkout.json' @{ head = $head; tree = $tree; workflowSourceRevision = $env:EXPECTED_SOURCE_REVISION
        declaredPrHead = $env:DECLARED_PR_HEAD; dirty = $dirty; runnerImage = $env:ImageOS; runnerImageVersion = $env:ImageVersion
        sourceAuthenticated = $false; proofQualified = $false; baselineApproved = $false; thresholds = $null; closureStatus = 'blocked' }
    if ($dirty.Count -ne 0 -or $head -ne $env:EXPECTED_SOURCE_REVISION) { throw 'Expected clean actual workflow source required' }
    $vsRoot = 'C:\Program Files\Microsoft Visual Studio\2022\Enterprise'
    $developer = Join-Path $vsRoot 'Common7\Tools\Launch-VsDevShell.ps1'
    Record 'developer-environment.command.json' @{ script = (Material $developer); arguments = @('-SkipAutomaticLocation', '-Arch', 'amd64', '-HostArch', 'amd64') }
    & $developer -SkipAutomaticLocation -Arch amd64 -HostArch amd64 1> (Join-Path $evidence 'developer.stdout.log') 2> (Join-Path $evidence 'developer.stderr.log')
    if (-not $?) { throw 'Visual Studio developer environment failed' }
    $compiler = (Get-Command cl.exe -CommandType Application | Select-Object -First 1).Source
    $linker = Join-Path (Split-Path -Parent $compiler) 'link.exe'
    $node = (Get-Command node.exe -CommandType Application | Select-Object -First 1).Source
    Record 'tool-locations.json' @($compiler, $linker, $node | ForEach-Object {
        $item = Get-Item -LiteralPath $_
        @{ requestedPath = $_; fullName = $item.FullName; itemType = $item.GetType().FullName
            attributes = [string]$item.Attributes; linkType = $item.LinkType; target = @($item.Target) }
    })
    $includes = @($env:INCLUDE.Split(';', [StringSplitOptions]::RemoveEmptyEntries))
    $libraries = @($env:LIB.Split(';', [StringSplitOptions]::RemoveEmptyEntries))
    $headers = @('windows.h', 'psapi.h', 'inttypes.h', 'stdio.h', 'wchar.h', 'stdint.h', 'stdbool.h', 'stddef.h', 'limits.h', 'string.h', 'stdarg.h', 'assert.h')
    $libs = @('kernel32.lib', 'libcmt.lib', 'libvcruntime.lib', 'libucrt.lib', 'oldnames.lib')
    Record 'toolchain-before-build.json' @{ compiler = (Material $compiler); linker = (Material $linker); node = (Material $node)
        compilerFileVersion = (Get-Item $compiler).VersionInfo.FileVersion; linkerFileVersion = (Get-Item $linker).VersionInfo.FileVersion
        sdkDirectory = $env:WindowsSdkDir; sdkVersion = $env:WindowsSDKVersion; vcToolsDirectory = $env:VCToolsInstallDir
        vcToolsVersion = $env:VCToolsVersion; includeSearch = $includes; libSearch = $libraries
        selectedDirectHeaders = (Selected $includes $headers); selectedStaticLibraries = (Selected $libraries $libs)
        selectionScope = 'First matching direct includes and static default libraries; not a complete transitive linker/header closure or compiler pin' }
    # cl /Bv with no translation unit may return nonzero; preserve the actual banner and code.
    $null = Capture 'compiler-version' $compiler @('/Bv')
    $null = Capture 'linker-version' $linker @('/?')
    if ((Capture 'node-version' $node @('--version')) -ne 0) { throw 'Node version observation failed' }
    if ((Get-Content -Raw (Join-Path $evidence 'node-version.stdout.log')).Trim() -ne 'v22.22.1') { throw 'Exact Node22.22.1 required' }
    $base = Split-Path -Parent $PSScriptRoot
    $pins = Get-Content -Raw (Join-Path $PSScriptRoot 'source-pins.json') | ConvertFrom-Json
    if ($pins.format -ne 'zryna.windows-job-calibration-source-pins.v1' -or @($pins.files).Count -ne 29) { throw 'Exact composed source inventory required' }
    $materials = @($pins.files | ForEach-Object {
        if ($_.path -notmatch '^[A-Za-z0-9_.-]+(/[A-Za-z0-9_.-]+)*$' -or $_.path.Split('/') -contains '..') { throw 'Invalid source inventory path' }
        $actual = Material (Join-Path $base $_.path)
        if ($actual.bytes -ne $_.bytes -or $actual.sha256 -cne $_.sha256) { throw "Source pin differs: $($_.path)" }
        $actual
    })
    Record 'materials-before-build.json' @{ inventory = (Material (Join-Path $PSScriptRoot 'source-pins.json')); sourceFiles = $materials
        entry = (Material $PSCommandPath); workflow = (Material (Join-Path $env:GITHUB_WORKSPACE '.github/workflows/windows-job-metrics.yml')) }
    $helperDir = Join-Path $evidence 'helper-build'
    $fixtureDir = Join-Path $evidence 'fixture-build'
    $helperRecipe = Join-Path $base 'build-msvc.cmd'
    $fixtureRecipe = Join-Path $PSScriptRoot 'build-msvc.cmd'
    if ((Capture 'build-helper' $helperRecipe @($helperDir)) -ne 0) { throw 'Helper build failed' }
    if ((Capture 'build-fixture' $fixtureRecipe @($fixtureDir)) -ne 0) { throw 'Fixture build failed' }
    $helper = Join-Path $helperDir 'windows-job-metrics-v1.exe'
    $fixture = Join-Path $fixtureDir 'windows-job-calibration-v1.exe'
    Record 'built-tools.json' @{ helper = (Material $helper); fixture = (Material $fixture) }
    $output = Join-Path $evidence 'controls'
    if ((Capture 'calibrate' $node @((Join-Path $PSScriptRoot 'calibrate.mjs'), '--helper', $helper, '--fixture', $fixture, '--out', $output)) -ne 0) { throw 'Calibration failed; retain raw observations' }
    $result = Get-Content -Raw (Join-Path $output 'calibration.json') | ConvertFrom-Json
    if ($result.status -ne 'controls-passed' -or @($result.results).Count -ne 8 -or @($result.results | Where-Object status -ne 'passed').Count -ne 0) { throw 'Exactly eight controls required' }
    Record 'hosted-result.json' @{ status = 'eight-controls-passed'; controls = 8; sourceAuthenticated = $false
        proofQualified = $false; baselineApproved = $false; thresholds = $null; closureStatus = 'blocked'
        remaining = 'Independent actual artifact review and exact MSVC/SDK pinning before further qualification; no workload baseline' }
} catch {
    Record 'hosted-failure.json' @{ status = 'failed'; error = $_.Exception.ToString(); sourceAuthenticated = $false
        proofQualified = $false; baselineApproved = $false; thresholds = $null; closureStatus = 'blocked' }
    throw
}
