# Install (or remove) Windows logon autostart for razochar6e 20-80% Kasa cycle.
#
# Default: start via WSL (recommended on this ROG + WSL setup).
# Native:  -Native if you have a Windows build of razochar6e.exe with cycle support.
#
# Usage (PowerShell):
#   .\scripts\install-cycle-autostart.ps1
#   .\scripts\install-cycle-autostart.ps1 -Uninstall
#   .\scripts\install-cycle-autostart.ps1 -Distro Ubuntu -StartNow
#   .\scripts\install-cycle-autostart.ps1 -Native -ExePath "$env:LOCALAPPDATA\Programs\razochar6e\razochar6e.exe"
param(
    [string]$Distro = 'Ubuntu',
    [string]$WslHome = '/home/josep',
    [string]$TaskName = 'razochar6e-cycle',
    [int]$Start = 20,
    [int]$End = 80,
    [int]$Interval = 60,
    [string]$HostIp = '192.168.1.88',
    [string]$ExePath = '',
    [switch]$Native,
    [switch]$StartNow,
    [switch]$Uninstall
)

$ErrorActionPreference = 'Stop'

function Ensure-KasaEnv([string]$Path) {
    $dir = Split-Path -Parent $Path
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
    }
    if (-not (Test-Path $Path)) {
        @"
# razochar6e Kasa credentials - do not commit
KASA_USERNAME='you@example.com'
KASA_PASSWORD='your-password'
KASA_HOST='$HostIp'
CYCLE_START=$Start
CYCLE_END=$End
CYCLE_INTERVAL=$Interval
"@ | Set-Content -Path $Path -Encoding UTF8
        Write-Host "Created template $Path - edit with your Kasa app login."
    }
}

if ($Uninstall) {
    Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue
    Write-Host "Removed scheduled task: $TaskName"
    exit 0
}

$action = $null
$argDisplay = ''

if ($Native) {
    if (-not $ExePath) {
        $ExePath = Join-Path $env:LOCALAPPDATA 'Programs\razochar6e\razochar6e.exe'
    }
    if (-not (Test-Path $ExePath)) {
        throw "Native exe not found: $ExePath (build Windows release or pass -ExePath)"
    }
    $winData = Join-Path $env:LOCALAPPDATA 'razochar6e'
    $envFile = Join-Path $winData 'kasa.env'
    Ensure-KasaEnv $envFile

    # Wrapper that loads env then runs cycle in background via Start-Process
    $wrapper = Join-Path $winData 'start-cycle.ps1'
    @"
`$ErrorActionPreference = 'Stop'
`$envFile = '$envFile'
Get-Content `$envFile | ForEach-Object {
  if (`$_ -match '^\s*#' -or `$_ -match '^\s*$') { return }
  if (`$_ -match "^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*'([^']*)'\s*`$") {
    Set-Item -Path ("Env:" + `$Matches[1]) -Value `$Matches[2]
  } elseif (`$_ -match '^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.+)\s*`$') {
    Set-Item -Path ("Env:" + `$Matches[1]) -Value `$Matches[2].Trim().Trim('"')
  }
}
`$hostIp = if (`$env:KASA_HOST) { `$env:KASA_HOST } else { '$HostIp' }
`$start = if (`$env:CYCLE_START) { `$env:CYCLE_START } else { '$Start' }
`$end = if (`$env:CYCLE_END) { `$env:CYCLE_END } else { '$End' }
`$interval = if (`$env:CYCLE_INTERVAL) { `$env:CYCLE_INTERVAL } else { '$Interval' }
`$log = Join-Path `$env:LOCALAPPDATA 'razochar6e\cycle.log'
Get-Process -Name razochar6e -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Process -FilePath '$ExePath' -ArgumentList @('cycle','--host',`$hostIp,'--start',`$start,'--end',`$end,'--interval',`$interval) `
  -WindowStyle Hidden -RedirectStandardOutput `$log -RedirectStandardError `$log
"@ | Set-Content -Path $wrapper -Encoding UTF8

    $ps = (Get-Command powershell.exe).Source
    $action = New-ScheduledTaskAction -Execute $ps -Argument "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$wrapper`""
    $argDisplay = "powershell -File $wrapper"
} else {
    # WSL path (default on this machine)
    $wslScript = "$WslHome/.local/share/razochar6e/cycle-daemon.sh"
    $wslEnv = "$WslHome/.local/share/razochar6e/kasa.env"
    $repoScript = "$WslHome/projects/razochar6e/scripts/cycle-daemon.sh"
    $repoInstall = "$WslHome/projects/razochar6e/scripts/install-cycle-autostart.sh"

    $wsl = (Get-Command wsl.exe -ErrorAction Stop).Source

    # Ensure WSL daemon wrapper + env template (no systemd — Windows task owns start)
    & $wsl -d $Distro -e bash -lc "bash '$repoInstall' --files-only >/tmp/razochar6e-autostart-install.log 2>&1" | Out-Null

    # Delay so LAN / WSL are up after login
    $arg = "-d $Distro -e bash -lc `"sleep 25; bash '$wslScript'`""
    $action = New-ScheduledTaskAction -Execute $wsl -Argument $arg
    $argDisplay = "wsl $arg"
    Write-Host "WSL env file (edit if needed): $wslEnv"
    Write-Host "WSL daemon: $wslScript (from $repoScript)"
}

$trigger = New-ScheduledTaskTrigger -AtLogOn -User $env:USERNAME
$settings = New-ScheduledTaskSettingsSet `
    -AllowStartIfOnBatteries `
    -DontStopIfGoingOnBatteries `
    -StartWhenAvailable `
    -ExecutionTimeLimit ([TimeSpan]::Zero) `
    -RestartCount 3 `
    -RestartInterval (New-TimeSpan -Minutes 1)
$principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive -RunLevel Limited

Register-ScheduledTask -TaskName $TaskName -Action $action -Trigger $trigger `
    -Settings $settings -Principal $principal -Force | Out-Null

# Also stash host defaults in user env (password stays in kasa.env)
[Environment]::SetEnvironmentVariable('KASA_HOST', $HostIp, 'User')

Write-Host "Installed logon task: $TaskName"
Write-Host "  runs: $argDisplay"
Get-ScheduledTask -TaskName $TaskName | Format-List TaskName, State

if ($StartNow) {
    Write-Host "Starting task now..."
    Start-ScheduledTask -TaskName $TaskName
}
