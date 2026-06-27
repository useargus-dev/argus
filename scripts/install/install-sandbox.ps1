# Install Argus Windows sandbox redirector + WinDivert bundle.
param(
    [string]$Version = "latest",
    [string]$Prefix = "$env:LOCALAPPDATA\Argus",
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"
$Asset = "argus-sandbox-windows-x86_64.zip"
$Base = "https://github.com/useargus-dev/argus/releases"
if ($Version -eq "latest") {
    $Url = "$Base/latest/download/$Asset"
} else {
    $Url = "$Base/download/$Version/$Asset"
}

$LibDir = Join-Path $Prefix "lib\argus"
Write-Host "Installing sandbox redirector to $LibDir"
Write-Host "  URL: $Url"

if ($DryRun) { exit 0 }

New-Item -ItemType Directory -Force -Path $LibDir | Out-Null
$Tmp = Join-Path $env:TEMP ("argus-sandbox-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force -Path $Tmp | Out-Null
try {
    $Zip = Join-Path $Tmp "sandbox.zip"
    Invoke-WebRequest -Uri $Url -OutFile $Zip
    Expand-Archive -Path $Zip -DestinationPath $LibDir -Force
    Write-Host "Installed redirector under $LibDir"
} finally {
    Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}

Write-Host "Network capture requires Administrator (WinDivert)."
