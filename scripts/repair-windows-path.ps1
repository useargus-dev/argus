# Repair Argus CLI registration after a partial NSIS install.
# Does NOT modify the Path environment variable.
# Run elevated to refresh machine ARGUS_HOME + App Paths.
param(
    [string]$InstallRoot = ""
)

$ErrorActionPreference = "Stop"

function Get-ArgusInstallRoot {
    param([string]$Override)
    if ($Override) { return $Override.TrimEnd('\') }
    foreach ($hive in @(
        @{ Root = "HKLM:\Software\Argus"; Name = "InstallPath" },
        @{ Root = "HKCU:\Software\Argus"; Name = "InstallPath" }
    )) {
        $value = (Get-ItemProperty -Path $hive.Root -Name $hive.Name -ErrorAction SilentlyContinue).InstallPath
        if ($value) { return $value.TrimEnd('\') }
    }
    if ($env:ARGUS_HOME) { return $env:ARGUS_HOME.TrimEnd('\') }
    $default = "${env:ProgramFiles}\argus"
    if (Test-Path $default) { return $default }
    throw "Could not determine Argus install directory. Pass -InstallRoot."
}

function Set-ArgusHomeEnv {
    param([string]$Scope, [string]$HomeDir)
    [Environment]::SetEnvironmentVariable("ARGUS_HOME", $HomeDir, $Scope)
    Write-Host "[$Scope] Set ARGUS_HOME=$HomeDir"
}

function Register-AppPath {
    param([string]$CliExe, [string]$BinDir)
    $key = "HKLM:\Software\Microsoft\Windows\CurrentVersion\App Paths\argus.exe"
    New-Item -Path $key -Force | Out-Null
    Set-ItemProperty -Path $key -Name "(default)" -Value $CliExe
    Set-ItemProperty -Path $key -Name "Path" -Value $BinDir
    Write-Host "[Machine] Registered App Paths: $CliExe"
}

function Install-UserShim {
    param([string]$CliExe)
    $windowsApps = Join-Path $env:LOCALAPPDATA "Microsoft\WindowsApps"
    New-Item -ItemType Directory -Force -Path $windowsApps | Out-Null
    $shim = Join-Path $windowsApps "argus.cmd"
    @(
        "@echo off"
        "`"$CliExe`" %*"
    ) | Set-Content -Path $shim -Encoding ASCII
    Write-Host "[User] Installed CLI shim: $shim"
}

$root = Get-ArgusInstallRoot -Override $InstallRoot
$bin = Join-Path $root "bin"
$cli = Join-Path $bin "argus.exe"

if (-not (Test-Path $cli)) {
    throw "CLI not found at $cli. Reinstall the full Argus setup.exe bundle."
}

$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator
)

Set-ArgusHomeEnv -Scope User -HomeDir $root
Install-UserShim -CliExe $cli

if ($isAdmin) {
    Set-ArgusHomeEnv -Scope Machine -HomeDir $root
    Register-AppPath -CliExe $cli -BinDir $bin
} else {
    Write-Warning "Not elevated: user shim + ARGUS_HOME only. Re-run as Administrator for App Paths."
}

$signature = @"
[DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Auto)]
public static extern IntPtr SendMessageTimeout(
    IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam,
    uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);
"@
Add-Type -MemberDefinition $signature -Name Win32SendMessage -Namespace Win32 -ErrorAction SilentlyContinue | Out-Null
[UIntPtr]$result = [UIntPtr]::Zero
[void][Win32.Win32SendMessage]::SendMessageTimeout(
    [IntPtr]0xffff, 0x001A, [UIntPtr]::Zero, "Environment", 2, 5000, [ref]$result)

Write-Host ""
Write-Host "Repair complete (Path env var untouched). Open a NEW terminal, then run: argus"
Write-Host "  Install root: $root"
Write-Host "  CLI:          $cli"
