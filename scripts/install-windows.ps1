# Install razochar6e Windows binary and logon scheduled task (run as Administrator).
param(
    [int]$Start = 20,
    [int]$End = 80
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path $PSScriptRoot -Parent

Push-Location $repoRoot
cargo build --release
Pop-Location

$exe = Join-Path $repoRoot "target\release\razochar6e.exe"
$destDir = "$env:LOCALAPPDATA\Programs\razochar6e"
New-Item -ItemType Directory -Force -Path $destDir | Out-Null
Copy-Item $exe (Join-Path $destDir "razochar6e.exe") -Force

& (Join-Path $destDir "razochar6e.exe") install-persist --start $Start --end $End
Write-Host "Installed to $destDir"
Write-Host "From WSL: razochar6e wsl set --start $Start --end $End"
