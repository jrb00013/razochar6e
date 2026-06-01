#Requires -Version 5.1
<#
.SYNOPSIS
  One-shot razochar6e setup for native Windows (ASUS ROG / ATKACPI).

.DESCRIPTION
  Downloads a prebuilt release binary by default (~30s), or compiles with -FromSource.
  Installs to %LOCALAPPDATA%\Programs\razochar6e, writes config, applies charge limits
  (UAC for set / persist), and runs doctor.

.EXAMPLE
  .\setup.ps1
  .\setup.ps1 -Start 25 -End 85
  .\setup.ps1 -FromSource
  irm https://raw.githubusercontent.com/jrb00013/razochar6e/main/setup.ps1 | iex
#>
[CmdletBinding()]
param(
    [int]$Start = 20,
    [int]$End = 80,
    [switch]$FromSource,
    [switch]$NoApply,
    [switch]$NoPersist,
    [switch]$SkipBuild,
    [switch]$Help
)

$ErrorActionPreference = "Stop"
$RepoRoot = $PSScriptRoot
$DestDir = Join-Path $env:LOCALAPPDATA "Programs\razochar6e"
$Exe = Join-Path $DestDir "razochar6e.exe"
$InstallScript = Join-Path $RepoRoot "scripts\install-windows.ps1"
$PersistScript = Join-Path $RepoRoot "scripts\install-persist-admin.ps1"

function Write-Step { param([string]$Message) Write-Host "==> $Message" -ForegroundColor Cyan }
function Write-WarnStep { param([string]$Message) Write-Host "!!> $Message" -ForegroundColor Yellow }
function Write-ErrStep { param([string]$Message) Write-Host "ERR> $Message" -ForegroundColor Red; exit 1 }

function Show-Help {
    @"
Usage: .\setup.ps1 [options]

  -Start N          Lower threshold in config (default: 20; often ignored on ASUS Windows)
  -End N            Upper charge cap (default: 80)
  -FromSource       Compile with cargo instead of downloading prebuilt
  -NoApply          Skip applying limits to hardware
  -NoPersist        Skip logon scheduled task (install-persist)
  -SkipBuild        Skip install (binary must already exist under Programs\razochar6e)
  -Help             Show this help

Examples:
  .\setup.ps1
  .\setup.ps1 -Start 25 -End 85
  powershell -ExecutionPolicy Bypass -File .\setup.ps1

Approve UAC when prompted for set / install-persist.
"@
}

function Test-IsAdmin {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    $p = New-Object Security.Principal.WindowsPrincipal($id)
    $p.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Invoke-Razochar6e {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$CommandArgs,
        [switch]$Elevate
    )

    if (-not (Test-Path $Exe)) {
        throw "razochar6e.exe not found at $Exe — run setup without -SkipBuild first"
    }

    $needsAdmin = $Elevate -or ($CommandArgs[0] -eq "set")
    $isAdmin = Test-IsAdmin

    if ($needsAdmin -and -not $isAdmin) {
        $argLine = $CommandArgs -join " "
        Write-Step "Requesting elevation: $Exe $argLine"
        $proc = Start-Process -FilePath $Exe -ArgumentList $CommandArgs -Verb RunAs -Wait -PassThru
        if ($proc.ExitCode -ne 0) {
            throw "razochar6e exited with code $($proc.ExitCode)"
        }
        return
    }

    & $Exe @CommandArgs
    if ($LASTEXITCODE -and $LASTEXITCODE -ne 0) {
        throw "razochar6e exited with code $LASTEXITCODE"
    }
}

function Install-Razochar6e {
    if (-not (Test-Path $InstallScript)) {
        Write-ErrStep "Missing $InstallScript — run from the repo root"
    }

    $installArgs = @{
        Start     = $Start
        End       = $End
        NoPersist = $true
    }

    if ($FromSource) {
        Write-Step "Mode: compile from source (first build: 5–20 min; pause at step 49/61 is normal)"
    } else {
        Write-Step "Mode: prebuilt download (use -FromSource to compile)"
        $installArgs.Prebuilt = $true
    }

    if ($SkipBuild) {
        $installArgs.NoBuild = $true
    }

    & $InstallScript @installArgs
}

function Set-Config {
    try {
        Invoke-Razochar6e -CommandArgs @("config", "set", "--start", "$Start", "--end", "$End")
        return
    } catch {
        Write-Verbose "config set failed, initializing: $_"
    }
    try {
        Invoke-Razochar6e -CommandArgs @("config", "init")
    } catch {
        Write-Verbose "config init: $_"
    }
    Invoke-Razochar6e -CommandArgs @("config", "set", "--start", "$Start", "--end", "$End")
}

function Register-Persist {
    if (-not (Test-Path $PersistScript)) {
        Write-WarnStep "Missing $PersistScript"
        return
    }

    if (Test-IsAdmin) {
        & $PersistScript -Start $Start -End $End
        return
    }

    Write-Step "Registering logon scheduled task (UAC)..."
    $ps = (Get-Command powershell.exe -ErrorAction SilentlyContinue).Source
    if (-not $ps) { $ps = (Get-Command pwsh.exe).Source }

    try {
        Start-Process -FilePath $ps -Verb RunAs -Wait -ArgumentList @(
            "-NoProfile",
            "-ExecutionPolicy", "Bypass",
            "-File", $PersistScript,
            "-Start", $Start,
            "-End", $End
        )
    } catch {
        Write-WarnStep "Elevated persist skipped. In Admin PowerShell run:"
        Write-Host "  & `"$Exe`" install-persist --start $Start --end $End"
    }
}

function Ensure-PathHint {
    $binDir = $DestDir
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$binDir*") {
        Write-WarnStep "Add to PATH (User): $binDir"
        Write-Host "  [Environment]::SetEnvironmentVariable('Path', `"$userPath;$binDir`", 'User')"
    }
}

if ($Help) {
    Show-Help
    exit 0
}

Write-Step "Platform: windows"
Write-Step "Repo: $RepoRoot"

if (-not $SkipBuild) {
    Install-Razochar6e
} elseif (-not (Test-Path $Exe)) {
    Write-ErrStep "Binary missing at $Exe (remove -SkipBuild or install first)"
}

Write-Step "Installed: $Exe"
Set-Config

if (-not $NoApply) {
    Write-Step "Applying charge limits (ASUS: usually end-only, e.g. $End%)..."
    try {
        Invoke-Razochar6e -CommandArgs @("set", "--start", "$Start", "--end", "$End", "--save") -Elevate
    } catch {
        Write-WarnStep "set failed — run as Administrator: razochar6e set --start $Start --end $End --save"
    }
}

if (-not $NoPersist) {
    Register-Persist
}

Ensure-PathHint

Write-Step "Running doctor..."
try {
    Invoke-Razochar6e -CommandArgs @("doctor")
} catch {
    Write-WarnStep "doctor reported issues — run: razochar6e probe"
}

Write-Step "Setup complete."
Write-Host ""
Write-Host "Daily use (PowerShell, approve UAC when changing limits):"
Write-Host "  razochar6e status"
Write-Host "  razochar6e set --start $Start --end $End --save"
Write-Host "  razochar6e probe"
Write-Host ""
Write-Host "Binary: $Exe"
