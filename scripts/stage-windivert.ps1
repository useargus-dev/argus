# Stage WinDivert runtime files next to the redirector binary.
# Run after: cargo build --release -p argus-redirector-windows
param(
    [string]$TargetDir = (Join-Path $PSScriptRoot "..\target\release"),
    [string]$StageDir = (Join-Path $PSScriptRoot "..\third_party\windivert")
)

$ErrorActionPreference = "Stop"
New-Item -ItemType Directory -Force -Path $StageDir | Out-Null

function Find-WindivertFile {
    param([string]$Name)

    $roots = @()
    if ($env:WINDIVERT_PATH) { $roots += $env:WINDIVERT_PATH }
    $roots += $StageDir
    $roots += $TargetDir

    foreach ($root in ($roots | Select-Object -Unique)) {
        if (-not $root) { continue }

        $direct = Join-Path $root $Name
        if (Test-Path $direct) { return $direct }

        $buildDir = Join-Path $root "build"
        if (Test-Path $buildDir) {
            $fromBuild = Get-ChildItem -Path $buildDir -Recurse -Filter $Name -ErrorAction SilentlyContinue |
                Select-Object -First 1
            if ($fromBuild) { return $fromBuild.FullName }
        }
    }

    return $null
}

$missing = @()
foreach ($name in @("WinDivert.dll", "WinDivert64.sys")) {
    $src = Find-WindivertFile -Name $name
    if (-not $src) {
        $missing += $name
        continue
    }
    $destStage = Join-Path $StageDir $name
    $destTarget = Join-Path $TargetDir $name
    $srcPath = (Resolve-Path -LiteralPath $src).Path
    if (-not (Test-Path $destStage) -or $srcPath -ne (Resolve-Path -LiteralPath $destStage).Path) {
        Copy-Item -Force $src $destStage
    }
    if (-not (Test-Path $destTarget) -or $srcPath -ne (Resolve-Path -LiteralPath $destTarget).Path) {
        Copy-Item -Force $src $destTarget
    }
    Write-Host "Staged $name from $src"
}

if ($missing.Count -gt 0) {
    throw "WinDivert files not found: $($missing -join ', '). Run scripts/fetch-windivert.ps1 first."
}

Write-Host "WinDivert files in: $StageDir"
