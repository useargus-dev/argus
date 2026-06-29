# Repair Argus PATH + ARGUS_HOME after a broken or partial NSIS install.
# Run in an elevated PowerShell for machine-wide PATH updates.
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

function Add-PathEntry {
    param(
        [string]$Scope,
        [string]$Entry
    )
    $current = [Environment]::GetEnvironmentVariable("Path", $Scope)
    $parts = @()
    if ($current) { $parts = $current -split ';' | Where-Object { $_ } }
    $exists = $parts | Where-Object { $_.TrimEnd('\').Equals($Entry, [StringComparison]::OrdinalIgnoreCase) }
    if ($exists) {
        Write-Host "[$Scope] PATH already contains $Entry"
        return
    }
    $updated = if ($parts.Count -gt 0) { ($parts + $Entry) -join ';' } else { $Entry }
    [Environment]::SetEnvironmentVariable("Path", $updated, $Scope)
    Write-Host "[$Scope] Added to PATH: $Entry"
}

function Set-ArgusHomeEnv {
    param(
        [string]$Scope,
        [string]$HomeDir
    )
    [Environment]::SetEnvironmentVariable("ARGUS_HOME", $HomeDir, $Scope)
    Write-Host "[$Scope] Set ARGUS_HOME=$HomeDir"
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

Add-PathEntry -Scope User -Entry $bin
Set-ArgusHomeEnv -Scope User -HomeDir $root

if ($isAdmin) {
    Add-PathEntry -Scope Machine -Entry $bin
    Set-ArgusHomeEnv -Scope Machine -HomeDir $root
} else {
    Write-Warning "Not elevated: updated user PATH/ARGUS_HOME only. Re-run as Administrator for system PATH."
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
Write-Host "Repair complete. Open a NEW terminal, then run: argus"
Write-Host "  Install root: $root"
Write-Host "  CLI:          $cli"
