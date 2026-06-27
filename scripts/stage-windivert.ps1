# Stage WinDivert runtime files next to the redirector binary.
# Run after: cargo build --release -p argus-redirector-windows
param(
    [string]$TargetDir = (Join-Path $PSScriptRoot "..\target\release"),
    [string]$StageDir = (Join-Path $PSScriptRoot "..\third_party\windivert")
)

$ErrorActionPreference = "Stop"
New-Item -ItemType Directory -Force -Path $StageDir | Out-Null

function Find-WindivertFile {
    param([string]$Name, [string]$SearchRoot)
    $fromBuild = Get-ChildItem -Path (Join-Path $SearchRoot "build") -Recurse -Filter $Name -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($fromBuild) { return $fromBuild.FullName }
    $direct = Join-Path $SearchRoot $Name
    if (Test-Path $direct) { return $direct }
    return $null
}

foreach ($name in @("WinDivert.dll", "WinDivert64.sys")) {
    $src = Find-WindivertFile -Name $name -SearchRoot $TargetDir
    if (-not $src) {
        Write-Warning "WinDivert: $name not found under $TargetDir (build redirector first)"
        continue
    }
    Copy-Item -Force $src (Join-Path $StageDir $name)
    Copy-Item -Force $src (Join-Path $TargetDir $name)
    Write-Host "Staged $name from $src"
}

Write-Host "WinDivert files in: $StageDir"
