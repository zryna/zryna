# Fixed private build and three live controls; no host admission or performance collection.
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$evidence = Join-Path $env:RUNNER_TEMP 'windows-parent-affinity-observation'
New-Item -ItemType Directory -Path $evidence -ErrorAction Stop | Out-Null
function AffinityRecord($name, $value) {
    $stream = [IO.File]::Open((Join-Path $evidence $name), [IO.FileMode]::CreateNew, [IO.FileAccess]::Write)
    try {
        $bytes = [Text.UTF8Encoding]::new($false).GetBytes(($value | ConvertTo-Json -Depth 16) + "`n")
        $stream.Write($bytes, 0, $bytes.Length)
    } finally { $stream.Dispose() }
}
function AffinityMaterial($path) {
    $item = Get-Item -LiteralPath $path
    if (-not ($item -is [IO.FileInfo]) -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "Ordinary file required: $path" }
    return @{ path = $item.FullName; bytes = $item.Length; sha256 = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
}
function AffinitySame($expected) {
    $actual = AffinityMaterial $expected.path
    if ($actual.path -cne $expected.path -or $actual.bytes -ne $expected.bytes -or $actual.sha256 -cne $expected.sha256) { throw "Selected material changed: $($expected.path)" }
    return $actual
}
function AffinityCapture($name, $exe, $arguments) {
    AffinityRecord "$name.command.json" @{ executable = $exe; arguments = @($arguments) }
    & $exe @arguments 1> (Join-Path $evidence "$name.stdout.log") 2> (Join-Path $evidence "$name.stderr.log")
    $code = $LASTEXITCODE
    AffinityRecord "$name.exit.json" @{ exitCode = $code }
    return $code
}
try {
    if ($env:RUNNER_OS -ne 'Windows') { throw 'Windows runner required' }
    $root = (Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
    if ($root -cne (Resolve-Path $env:GITHUB_WORKSPACE).Path) { throw 'Same actual checkout required' }
    Push-Location $root
    try {
        if ((AffinityCapture 'checkout-head' 'git' @('rev-parse', 'HEAD')) -ne 0) { throw 'Head observation failed' }
        if ((AffinityCapture 'checkout-tree' 'git' @('rev-parse', 'HEAD^{tree}')) -ne 0) { throw 'Tree observation failed' }
        if ((AffinityCapture 'checkout-clean' 'git' @('status', '--porcelain=v1', '--untracked-files=all')) -ne 0) { throw 'Status observation failed' }
        $head = (Get-Content -Raw (Join-Path $evidence 'checkout-head.stdout.log')).Trim()
        $tree = (Get-Content -Raw (Join-Path $evidence 'checkout-tree.stdout.log')).Trim()
        $dirty = Get-Content -Raw (Join-Path $evidence 'checkout-clean.stdout.log')
        AffinityRecord 'checkout.json' @{ head = $head; tree = $tree; expectedHead = $env:EXPECTED_SOURCE_REVISION
            declaredPrHead = $env:DECLARED_PR_HEAD; dirty = $dirty; sourceAuthenticated = $false }
        if ($head -notmatch '^[a-f0-9]{40}$' -or $tree -notmatch '^[a-f0-9]{40}$' -or $head -cne $env:EXPECTED_SOURCE_REVISION -or $dirty) { throw 'Expected clean exact checkout required' }
        $calibration = Join-Path $env:RUNNER_TEMP 'windows-job-metrics-calibration'
        $selected = Get-Content -Raw (Join-Path $calibration 'toolchain-before-build.json') | ConvertFrom-Json
        $oldResult = Get-Content -Raw (Join-Path $calibration 'hosted-result.json') | ConvertFrom-Json
        $controls = Get-Content -Raw (Join-Path $calibration 'controls/calibration.json') | ConvertFrom-Json
        AffinityRecord 'calibration-binding.json' @{ toolchain = (AffinityMaterial (Join-Path $calibration 'toolchain-before-build.json'))
            hosted = (AffinityMaterial (Join-Path $calibration 'hosted-result.json')); controls = (AffinityMaterial (Join-Path $calibration 'controls/calibration.json')) }
        if ($oldResult.status -cne 'eight-controls-passed' -or $oldResult.controls -ne 8 -or $controls.status -cne 'controls-passed' -or @($controls.results).Count -ne 8 -or @($controls.results | Where-Object status -ne 'passed').Count -ne 0) { throw 'Existing eight controls must succeed first' }
        $compiler = (Get-Command cl.exe -CommandType Application | Select-Object -First 1).Source
        $linker = (Get-Command link.exe -CommandType Application | Select-Object -First 1).Source
        $node = (Get-Command node.exe -CommandType Application | Select-Object -First 1).Source
        AffinityRecord 'toolchain-observed.json' @{ compiler = (AffinityMaterial $compiler); linker = (AffinityMaterial $linker); node = (AffinityMaterial $node)
            sdkDirectory = $env:WindowsSdkDir; sdkVersion = $env:WindowsSDKVersion; vcToolsDirectory = $env:VCToolsInstallDir
            vcToolsVersion = $env:VCToolsVersion; includeSearch = $env:INCLUDE; libSearch = $env:LIB }
        foreach ($pair in @(@($compiler, $selected.compiler), @($linker, $selected.linker), @($node, $selected.node))) {
            if ((AffinityMaterial $pair[0]).path -cne $pair[1].path) { throw 'Same calibrated tool paths required' }
            $null = AffinitySame $pair[1]
        }
        $includes = @($env:INCLUDE.Split(';', [StringSplitOptions]::RemoveEmptyEntries))
        $libraries = @($env:LIB.Split(';', [StringSplitOptions]::RemoveEmptyEntries))
        if (($includes | ConvertTo-Json -Compress) -cne (@($selected.includeSearch) | ConvertTo-Json -Compress) -or ($libraries | ConvertTo-Json -Compress) -cne (@($selected.libSearch) | ConvertTo-Json -Compress)) { throw 'Same calibrated SDK/CRT search order required' }
        foreach ($pair in @(@($env:WindowsSdkDir, $selected.sdkDirectory), @($env:WindowsSDKVersion, $selected.sdkVersion), @($env:VCToolsInstallDir, $selected.vcToolsDirectory), @($env:VCToolsVersion, $selected.vcToolsVersion))) {
            if ($pair[0] -cne $pair[1]) { throw 'Same calibrated SDK/VC selection required' }
        }
        $header = $null
        foreach ($directory in $includes) {
            $candidate = Join-Path $directory 'tlhelp32.h'
            if (Test-Path -LiteralPath $candidate -PathType Leaf) { $header = AffinityMaterial $candidate; break }
        }
        if (-not $header) { throw 'Selected direct tlhelp32.h absent' }
        $dependencies = @($selected.selectedDirectHeaders) + @($selected.selectedStaticLibraries) + @($header)
        AffinityRecord 'supplemental-sdk.json' @{ tlhelp32 = $header; dependencies = @($dependencies | ForEach-Object { AffinityMaterial $_.path })
            selectionScope = 'Same calibrated direct dependencies plus first matching tlhelp32.h; no complete transitive SDK claim' }
        foreach ($pin in $dependencies) { $null = AffinitySame $pin }
        $source = @('probe.c', 'build-msvc.cmd', 'decode.mjs', 'decode.test.mjs', 'hosted.ps1', 'live-controls.mjs') | ForEach-Object { AffinityMaterial (Join-Path $PSScriptRoot $_) }
        $workflow = AffinityMaterial (Join-Path $root '.github/workflows/windows-job-metrics.yml')
        $toolchain = @{ compiler = $selected.compiler; linker = $selected.linker; node = $selected.node }
        $build = @{ format = 'zryna.windows-parent-affinity-build.v1'; checkout = @{ head = $head; tree = $tree }
            sourceBefore = @($source); workflow = $workflow; toolchain = $toolchain; dependencies = $dependencies
            calibratedToolchain = (AffinityMaterial (Join-Path $calibration 'toolchain-before-build.json'))
            supplementalDirectHeader = $header; sourceAuthenticated = $false; proofQualified = $false
            performanceQualified = $false; hostAdmitted = $false; closureStatus = 'blocked'; thresholds = $null }
        AffinityRecord 'build-inputs.json' $build
        $buildDir = Join-Path $evidence 'build'
        if ((AffinityCapture 'build' (Join-Path $PSScriptRoot 'build-msvc.cmd') @($buildDir)) -ne 0) { throw 'Parent-affinity build failed' }
        $build.helper = AffinityMaterial (Join-Path $buildDir 'windows-parent-affinity-v1.exe')
        AffinityRecord 'affinity-build.json' $build
        AffinityRecord 'materials-after-build.json' @(@($source) + @($workflow) + $dependencies + @($selected.compiler, $selected.linker, $selected.node, $build.helper) | ForEach-Object { AffinityMaterial $_.path })
        foreach ($pin in @($source) + @($workflow) + $dependencies + @($selected.compiler, $selected.linker, $selected.node)) { $null = AffinitySame $pin }
        if ((AffinityCapture 'live-controls' $node @((Join-Path $PSScriptRoot 'live-controls.mjs'), $evidence)) -ne 0) { throw 'Live parent-affinity control failed; original evidence retained' }
        AffinityRecord 'materials-after-live.json' @(@($source) + @($workflow) + $dependencies + @($selected.compiler, $selected.linker, $selected.node, $build.helper) | ForEach-Object { AffinityMaterial $_.path })
        foreach ($pin in @($source) + @($workflow) + $dependencies + @($selected.compiler, $selected.linker, $selected.node, $build.helper)) { $null = AffinitySame $pin }
        AffinityRecord 'source-tools-after.json' @{ sources = @($source | ForEach-Object { AffinitySame $_ }); workflow = (AffinitySame $workflow)
            dependencies = @($dependencies | ForEach-Object { AffinitySame $_ }); tools = $toolchain; helper = (AffinitySame $build.helper) }
        $live = Get-Content -Raw (Join-Path $evidence 'live-controls.json') | ConvertFrom-Json
        if ($live.status -cne 'three-controls-observed-unqualified' -or @($live.results).Count -ne 3) { throw 'Exactly three live controls required' }
        AffinityRecord 'hosted-result.json' @{ status = 'three-controls-observed-unqualified'; controls = 3; sourceAuthenticated = $false
            proofQualified = $false; performanceQualified = $false; hostAdmitted = $false; closureStatus = 'blocked'
            baselineAuthority = $null; thresholdAuthority = $null; thresholds = $null }
    } finally { Pop-Location }
} catch {
    AffinityRecord 'hosted-failure.json' @{ status = 'failed'; error = $_.Exception.ToString(); sourceAuthenticated = $false
        proofQualified = $false; performanceQualified = $false; hostAdmitted = $false; closureStatus = 'blocked'; thresholds = $null }
    throw
}
