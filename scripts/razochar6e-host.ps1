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
    $candidates = @(
        "$PSScriptRoot\..\target\release\razochar6e.exe",
        "$PSScriptRoot\..\target\debug\razochar6e.exe",
        "$env:LOCALAPPDATA\Programs\razochar6e\razochar6e.exe",
        "$env:ProgramFiles\razochar6e\razochar6e.exe"
    )
    foreach ($p in $candidates) {
        if (Test-Path $p) { return (Resolve-Path $p).Path }
    }
    $cmd = Get-Command razochar6e -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    throw "razochar6e.exe not found. Build on Windows: cargo build --release"
}

function Invoke-Razochar6e {
    param([string[]]$Args)
    $exe = Find-Razochar6e
    $argLine = ($Args -join " ")
    $isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
        [Security.Principal.WindowsBuiltInRole]::Administrator
    )
    if ($isAdmin) {
        & $exe @Args
    } else {
        Write-Host "Requesting elevation for: $exe $argLine"
        Start-Process -FilePath $exe -ArgumentList $Args -Verb RunAs -Wait
    }
}

switch ($Command) {
    "probe" {
        Invoke-Razochar6e @("probe", "--json")
    }
    "status" {
        Invoke-Razochar6e @("status")
    }
    "set" {
        Invoke-Razochar6e @("set", "--start", "$Start", "--end", "$End")
    }
}
