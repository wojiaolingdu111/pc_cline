param(
    [string]$Bundles = "all"
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Resolve-Path (Join-Path $ScriptDir "../..")
Set-Location $RootDir

Write-Host "[1/3] Install frontend dependencies"
pnpm install --frozen-lockfile

Write-Host "[2/3] Build Tauri bundles: $Bundles"
pnpm exec tauri build --bundles $Bundles

Write-Host "Done. Bundles are in src-tauri/target/release/bundle"
