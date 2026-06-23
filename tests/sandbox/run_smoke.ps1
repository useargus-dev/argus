# Smoke tests for argus run on Windows (requires running Argus desktop).
param(
    [string]$ArgusBin = "argus-cli",
    [string]$EnvFile = ".env"
)

$ErrorActionPreference = "Continue"

Write-Host "== argus --version =="
& $ArgusBin --version

Write-Host "== argus status =="
& $ArgusBin status

Write-Host "== argus run --dry-run =="
& $ArgusBin run --env $EnvFile --dry-run -- cmd /c echo hello

Write-Host "Smoke checks completed (full curl tests require live bucket + Administrator)."
