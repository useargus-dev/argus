# Restore user + machine Path from backup files (run as Administrator for machine Path).
param(
    [string]$UserPathFile = (Join-Path $PSScriptRoot "..\..\path-user-restore.txt"),
    [string]$MachinePathFile = (Join-Path $PSScriptRoot "..\..\path-machine-restore.txt")
)

$ErrorActionPreference = "Stop"

function Broadcast-EnvironmentChange {
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
}

function Read-PathFile {
    param([string]$Path)
    if (-not (Test-Path $Path)) {
        throw "Missing backup file: $Path"
    }
    return (Get-Content -Path $Path -Raw).Trim()
}

$userPath = Read-PathFile -Path $UserPathFile
[Environment]::SetEnvironmentVariable("Path", $userPath, "User")
Write-Host "Restored user Path ($($userPath.Length) chars)"

$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator
)
if ($isAdmin) {
    $machinePath = Read-PathFile -Path $MachinePathFile
    [Environment]::SetEnvironmentVariable("Path", $machinePath, "Machine")
    Write-Host "Restored machine Path ($($machinePath.Length) chars)"
} else {
    Write-Warning "Not elevated: machine Path not restored. Re-run as Administrator."
}

Broadcast-EnvironmentChange
Write-Host "Done. Open a NEW terminal."
