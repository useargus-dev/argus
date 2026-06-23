# Stage WinDivert runtime files next to the redirector binary.
# Run after: cargo build --release -p argus-redirector-windows
param(
    [string]$TargetDir = (Join-Path $PSScriptRoot "..\target\release"),
    [string]$StageDir = (Join-Path $PSScriptRoot "..\third_party\windivert")
)

$ErrorActionPreference = "Stop"
New-Item -ItemType Directory -Force -Path $StageDir | Out-Null

$candidates = @(
    (Join-Path $TargetDir "WinDivert.dll"),
    (Join-Path $TargetDir "WinDivert64.sys")
)

# windivert-sys may place files under target/release/build/windivert-sys-*/out/
Get-ChildItem -Path (Join-Path $TargetDir "build") -Recurse -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -in @("WinDivert.dll", "WinDivert64.sys") } |
    ForEach-Object { $candidates += $_.FullName }

foreach ($name in @("WinDivert.dll", "WinDivert64.sys")) {
    $src = $candidates | Where-Object { $_ -and (Split-Path $_ -Leaf) -eq $name } | Select-Object -First 1
    if (-not $src) {
        Write-Warning "WinDivert: $name not found under $TargetDir (build redirector first)"
        continue
    }
    Copy-Item -Force $src (Join-Path $StageDir $name)
    Copy-Item -Force $src (Join-Path $TargetDir $name)
    Write-Host "Staged $name"
}

Write-Host "WinDivert files in: $StageDir"
