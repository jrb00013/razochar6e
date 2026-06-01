# Install razochar6e Windows binary and logon scheduled task (run as Administrator).
param(
    [int]$Start = 20,
    [int]$End = 80,
    [switch]$NoPersist,
    [switch]$NoBuild,
    [switch]$Prebuilt
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path $PSScriptRoot -Parent
$destDir = "$env:LOCALAPPDATA\Programs\razochar6e"
$exe = Join-Path $destDir "razochar6e.exe"

function Install-FromPrebuilt {
    $ver = (Select-String -Path (Join-Path $repoRoot "Cargo.toml") -Pattern '^version = "(.*)"' | ForEach-Object { $_.Matches.Groups[1].Value })
    $tag = "v$ver"
    $url = "https://github.com/jrb00013/razochar6e/releases/download/$tag/razochar6e-$ver-windows-x86_64.tar.gz"
    try {
        $null = Invoke-WebRequest -Uri "https://api.github.com/repos/jrb00013/razochar6e/releases/tags/$tag" -Method Head
    } catch {
        $tag = (Invoke-RestMethod "https://api.github.com/repos/jrb00013/razochar6e/releases/latest").tag_name
        $ver = $tag.TrimStart('v')
        $url = "https://github.com/jrb00013/razochar6e/releases/download/$tag/razochar6e-$ver-windows-x86_64.tar.gz"
    }
    $tmp = Join-Path $env:TEMP "razochar6e-dl"
    New-Item -ItemType Directory -Force -Path $tmp | Out-Null
    $archive = Join-Path $tmp "win.tar.gz"
    Write-Host "Downloading $url"
    Invoke-WebRequest -Uri $url -OutFile $archive
    tar -xzf $archive -C $tmp
    New-Item -ItemType Directory -Force -Path $destDir | Out-Null
    Copy-Item (Join-Path $tmp "razochar6e.exe") $exe -Force
    $scriptsDest = Join-Path $destDir "scripts"
    New-Item -ItemType Directory -Force -Path $scriptsDest | Out-Null
    Copy-Item (Join-Path $repoRoot "scripts\*.ps1") $scriptsDest -Force
}

if ($Prebuilt) {
    Install-FromPrebuilt
} elseif (-not $NoBuild) {
    Push-Location $repoRoot
    Write-Host "Compiling (first run: 5-20 min on Windows — normal to pause at serde/clap step 49/61)..."
    $env:CARGO_TERM_PROGRESS = "always"
    cargo build --release
    Pop-Location
    $exe = Join-Path $repoRoot "target\release\razochar6e.exe"
    New-Item -ItemType Directory -Force -Path $destDir | Out-Null
    Copy-Item $exe (Join-Path $destDir "razochar6e.exe") -Force
    $scriptsDest = Join-Path $destDir "scripts"
    New-Item -ItemType Directory -Force -Path $scriptsDest | Out-Null
    Copy-Item (Join-Path $repoRoot "scripts\*.ps1") $scriptsDest -Force
} else {
    $exe = Join-Path $repoRoot "target\release\razochar6e.exe"
    if (-not (Test-Path $exe)) { throw "Missing $exe — build first or use -Prebuilt" }
    New-Item -ItemType Directory -Force -Path $destDir | Out-Null
    Copy-Item $exe (Join-Path $destDir "razochar6e.exe") -Force
}
if (-not $NoPersist) {
    & (Join-Path $destDir "razochar6e.exe") install-persist --start $Start --end $End
}
Write-Host "Installed to $destDir"
Write-Host "From WSL: razochar6e wsl set --start $Start --end $End"
