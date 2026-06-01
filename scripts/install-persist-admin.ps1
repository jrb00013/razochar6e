# Run razochar6e install-persist elevated (called from setup.sh / WSL).
param(
    [int]$Start = 20,
    [int]$End = 80
)

$ErrorActionPreference = "Stop"
$exe = Join-Path $env:LOCALAPPDATA "Programs\razochar6e\razochar6e.exe"
if (-not (Test-Path $exe)) {
    throw "Not found: $exe — run setup.ps1, setup.sh, or install-windows.ps1 first"
}
& $exe install-persist --start $Start --end $End
Write-Host "install-persist OK (start=$Start end=$End)"
