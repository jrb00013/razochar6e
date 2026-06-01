# Host-side helper for WSL: runs razochar6e on Windows (elevated when needed).
param(
    [Parameter(Position = 0)]
    [ValidateSet("probe", "status", "set")]
    [string]$Command = "probe",
    [Parameter(Position = 1)]
    [int]$Start = 20,
    [Parameter(Position = 2)]
    [int]$End = 80
)

$ErrorActionPreference = "Stop"

function Find-Razochar6e {
    $root = Split-Path $PSScriptRoot -Parent
    $candidates = @(
        "$env:LOCALAPPDATA\Programs\razochar6e\razochar6e.exe",
        "$root\target\release\razochar6e.exe",
        "$root\target\debug\razochar6e.exe",
        "$env:ProgramFiles\razochar6e\razochar6e.exe"
    )
    foreach ($p in $candidates) {
        if (Test-Path $p) { return (Resolve-Path $p).Path }
    }
    $cmd = Get-Command razochar6e -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    throw "razochar6e.exe not found. Run ./setup.sh on Windows or WSL first."
}

function Invoke-Razochar6e {
    param([string[]]$CommandArgs)

    if ($null -eq $CommandArgs -or $CommandArgs.Count -eq 0) {
        throw "Invoke-Razochar6e: empty CommandArgs"
    }

    $exe = Find-Razochar6e
    $argLine = ($CommandArgs -join " ")
    $isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
        [Security.Principal.WindowsBuiltInRole]::Administrator
    )

    if ($isAdmin) {
        & $exe @CommandArgs
        if ($LASTEXITCODE -and $LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        return
    }

    # probe/status often work without admin; set needs Admin on ASUS
    if ($Command -eq "probe" -or $Command -eq "status") {
        & $exe @CommandArgs
        if ($LASTEXITCODE -and $LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        return
    }

    Write-Host "Requesting elevation for: $exe $argLine"
    $proc = Start-Process -FilePath $exe -ArgumentList $CommandArgs -Verb RunAs -Wait -PassThru
    if ($proc.ExitCode -ne 0) { exit $proc.ExitCode }
}

switch ($Command) {
    "probe" {
        Invoke-Razochar6e -CommandArgs @("probe", "--json")
    }
    "status" {
        Invoke-Razochar6e -CommandArgs @("status")
    }
    "set" {
        Invoke-Razochar6e -CommandArgs @("set", "--start", "$Start", "--end", "$End")
    }
}
