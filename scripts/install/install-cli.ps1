# Install Argus CLI sidecar only (requires running Argus desktop for IPC).
param(
    [string]$Version = "latest",
    [string]$Prefix = "$env:LOCALAPPDATA\Argus",
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"
$Asset = "argus-cli-x86_64-pc-windows-msvc.zip"
$Base = "https://github.com/useargus/argus/releases"
if ($Version -eq "latest") {
    $Url = "$Base/latest/download/$Asset"
} else {
    $Url = "$Base/download/$Version/$Asset"
}

$BinDir = Join-Path $Prefix "bin"
Write-Host "Installing Argus CLI to $BinDir"
Write-Host "  URL: $Url"

if ($DryRun) { exit 0 }

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
$Tmp = Join-Path $env:TEMP ("argus-cli-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force -Path $Tmp | Out-Null
try {
    $Zip = Join-Path $Tmp "cli.zip"
    Invoke-WebRequest -Uri $Url -OutFile $Zip
    Expand-Archive -Path $Zip -DestinationPath $Tmp -Force
    Copy-Item (Join-Path $Tmp "argus-cli.exe") (Join-Path $BinDir "argus.exe") -Force
    Write-Host "Installed: $(Join-Path $BinDir 'argus.exe')"
} finally {
    Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}

Write-Host "Ensure Argus desktop is running and signed in before using 'argus run'."
