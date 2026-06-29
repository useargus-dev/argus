# Download official WinDivert x64 runtime (WinDivert.dll, WinDivert64.sys).
# Used by CI and local release builds; binaries are gitignored under third_party/windivert/.
param(
    [string]$DestDir = (Join-Path $PSScriptRoot "..\third_party\windivert"),
    [string]$Version = "2.2.2-A",
    [string]$Url = "https://www.reqrypt.org/download/WinDivert-2.2.2-A.zip"
)

$ErrorActionPreference = "Stop"
New-Item -ItemType Directory -Force -Path $DestDir | Out-Null

$dll = Join-Path $DestDir "WinDivert.dll"
$sys = Join-Path $DestDir "WinDivert64.sys"
if ((Test-Path $dll) -and (Test-Path $sys)) {
    Write-Host "WinDivert already present in $DestDir"
    exit 0
}

$zip = Join-Path $env:TEMP "WinDivert-$Version.zip"
$extractRoot = Join-Path $env:TEMP "WinDivert-$Version-extract"

Write-Host "Downloading WinDivert $Version from $Url ..."
Invoke-WebRequest -Uri $Url -OutFile $zip -UseBasicParsing

if (Test-Path $extractRoot) {
    Remove-Item -Recurse -Force $extractRoot
}
Expand-Archive -Path $zip -DestinationPath $extractRoot -Force

$archDir = Join-Path $extractRoot "WinDivert-$Version\x64"
if (-not (Test-Path $archDir)) {
    $found = Get-ChildItem -Path $extractRoot -Recurse -Directory -Filter "x64" |
        Select-Object -First 1
    if ($found) {
        $archDir = $found.FullName
    } else {
        throw "WinDivert x64 directory not found in downloaded package"
    }
}

foreach ($name in @("WinDivert.dll", "WinDivert64.sys")) {
    $src = Join-Path $archDir $name
    if (-not (Test-Path $src)) {
        throw "Missing $name in WinDivert package (looked in $archDir)"
    }
    Copy-Item -Force $src (Join-Path $DestDir $name)
    Write-Host "Installed $name -> $DestDir"
}

Remove-Item -Force $zip -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force $extractRoot -ErrorAction SilentlyContinue
Write-Host "WinDivert runtime ready: $DestDir"
