# Install (or remove) a Windows *system service* that auto-starts the
# razochar6e 20–80% Kasa charge/drain cycle at boot.
#
# Default: WinSW service → wsl.exe → cycle-daemon.sh (ROG + WSL setup).
# Native:  -Native if you have a Windows build of razochar6e.exe with `cycle`.
#
# Requires Administrator (service create/start). UAC prompt if not elevated.
#
# Usage (PowerShell):
#   .\scripts\install-cycle-autostart.ps1
#   .\scripts\install-cycle-autostart.ps1 -StartNow
#   .\scripts\install-cycle-autostart.ps1 -Uninstall
#   .\scripts\install-cycle-autostart.ps1 -Distro Ubuntu -StartNow
#   .\scripts\install-cycle-autostart.ps1 -Native -ExePath "$env:LOCALAPPDATA\Programs\razochar6e\razochar6e.exe"
param(
    [string]$Distro = 'Ubuntu',
    [string]$WslHome = '/home/josep',
    [string]$ServiceName = 'razochar6e-cycle',
    [string]$TaskName = 'razochar6e-cycle', # legacy logon task; removed on install
    [int]$Start = 20,
    [int]$End = 80,
    [int]$Interval = 60,
    [string]$HostIp = '192.168.1.88',
    [string]$ExePath = '',
    [string]$WinSWUrl = 'https://github.com/winsw/winsw/releases/download/v2.12.0/WinSW-x64.exe',
    [switch]$Native,
    [switch]$StartNow,
    [switch]$Uninstall,
    [switch]$NoElevate
)

$ErrorActionPreference = 'Stop'

function Test-IsAdmin {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    $p = New-Object Security.Principal.WindowsPrincipal($id)
    return $p.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

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

function Get-ServiceDir {
    return Join-Path $env:ProgramData 'razochar6e'
}

function Get-WinSWPaths {
    $dir = Get-ServiceDir
    return @{
        Dir    = $dir
        Exe    = Join-Path $dir "$ServiceName.exe"
        Xml    = Join-Path $dir "$ServiceName.xml"
        LogDir = Join-Path $dir 'logs'
    }
}

function Stop-LegacyTask {
    Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue
}

function Uninstall-CycleService {
    $paths = Get-WinSWPaths
    Stop-LegacyTask
    if (Test-Path $paths.Exe) {
        try { & $paths.Exe stop | Out-Null } catch {}
        try { & $paths.Exe uninstall | Out-Null } catch {}
    }
    # sc delete fallback if WinSW binary was removed first
    $svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    if ($svc) {
        Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
        sc.exe delete $ServiceName | Out-Null
    }
    Write-Host "Removed Windows service: $ServiceName"
}

if ($Uninstall) {
    if (-not (Test-IsAdmin) -and -not $NoElevate) {
        $self = $MyInvocation.MyCommand.Path
        Start-Process -FilePath (Get-Command powershell.exe).Source -Verb RunAs -Wait `
            -ArgumentList @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $self, '-Uninstall', '-NoElevate')
        exit $LASTEXITCODE
    }
    Uninstall-CycleService
    exit 0
}

# Elevate for service install
if (-not (Test-IsAdmin) -and -not $NoElevate) {
    $self = $MyInvocation.MyCommand.Path
    $pass = @(
        '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $self,
        '-Distro', $Distro, '-WslHome', $WslHome, '-ServiceName', $ServiceName,
        '-Start', "$Start", '-End', "$End", '-Interval', "$Interval",
        '-HostIp', $HostIp, '-WinSWUrl', $WinSWUrl, '-NoElevate'
    )
    if ($Native) { $pass += '-Native' }
    if ($ExePath) { $pass += @('-ExePath', $ExePath) }
    if ($StartNow) { $pass += '-StartNow' }
    Write-Host "Elevating to install Windows service '$ServiceName' (UAC)..."
    $p = Start-Process -FilePath (Get-Command powershell.exe).Source -Verb RunAs -Wait -PassThru `
        -ArgumentList $pass
    exit $p.ExitCode
}

$paths = Get-WinSWPaths
New-Item -ItemType Directory -Force -Path $paths.Dir | Out-Null
New-Item -ItemType Directory -Force -Path $paths.LogDir | Out-Null

# Replace any prior logon-task install
Stop-LegacyTask
Uninstall-CycleService

$executable = ''
$arguments = ''
$argDisplay = ''

if ($Native) {
    if (-not $ExePath) {
        $ExePath = Join-Path $env:LOCALAPPDATA 'Programs\razochar6e\razochar6e.exe'
    }
    if (-not (Test-Path $ExePath)) {
        throw "Native exe not found: $ExePath (build Windows release with cycle support, or omit -Native)"
    }
    $winData = Join-Path $env:LOCALAPPDATA 'razochar6e'
    $envFile = Join-Path $winData 'kasa.env'
    Ensure-KasaEnv $envFile

    $wrapper = Join-Path $paths.Dir 'start-cycle-native.ps1'
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
& '$ExePath' cycle --host `$hostIp --start `$start --end `$end --interval `$interval
"@ | Set-Content -Path $wrapper -Encoding UTF8

    $executable = (Get-Command powershell.exe).Source
    $arguments = "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$wrapper`""
    $argDisplay = "powershell -File $wrapper"
} else {
    $wslScript = "$WslHome/.local/share/razochar6e/cycle-daemon.sh"
    $wslEnv = "$WslHome/.local/share/razochar6e/kasa.env"
    $repoInstall = "$WslHome/projects/razochar6e/scripts/install-cycle-autostart.sh"
    $wsl = (Get-Command wsl.exe -ErrorAction Stop).Source

    # Sync daemon wrapper + env template into the WSL data dir
    & $wsl -d $Distro -e bash -lc "bash '$repoInstall' --files-only >/tmp/razochar6e-autostart-install.log 2>&1"
    if ($LASTEXITCODE -ne 0) {
        throw "WSL files-only install failed (exit $LASTEXITCODE); see /tmp/razochar6e-autostart-install.log in distro $Distro"
    }

    # Stay attached in the foreground so the service (and wsl.exe) keep the cycle alive.
    # Short sleep lets LAN / WSL networking come up after boot.
    $bashLc = "sleep 20; exec bash '$wslScript'"
    $executable = $wsl
    $arguments = "-d $Distro -e bash -lc `"$bashLc`""
    $argDisplay = "wsl $arguments"
    Write-Host "WSL env file (edit if needed): $wslEnv"
    Write-Host "WSL daemon: $wslScript"
}

# Fetch WinSW once into ProgramData (service-aware wrapper)
if (-not (Test-Path $paths.Exe)) {
    Write-Host "Downloading WinSW service wrapper..."
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    Invoke-WebRequest -Uri $WinSWUrl -OutFile $paths.Exe -UseBasicParsing
}

$xml = @"
<service>
  <id>$ServiceName</id>
  <name>razochar6e Cycle (20-80)</name>
  <description>razochar6e Kasa charge/drain cycle: restore AC at ${Start}%, cut near ${End}%. Auto-starts at boot.</description>
  <executable>$executable</executable>
  <arguments>$arguments</arguments>
  <startmode>Automatic</startmode>
  <delayedAutoStart>true</delayedAutoStart>
  <onfailure action="restart" delay="15 sec"/>
  <onfailure action="restart" delay="30 sec"/>
  <onfailure action="restart" delay="60 sec"/>
  <resetfailure>1 hour</resetfailure>
  <stoptimeout>20 sec</stoptimeout>
  <logpath>$($paths.LogDir)</logpath>
  <log mode="roll-by-size">
    <sizeThreshold>2048</sizeThreshold>
    <keepFiles>4</keepFiles>
  </log>
</service>
"@
# WinSW XML must sit beside the renamed exe
Set-Content -Path $paths.Xml -Value $xml -Encoding UTF8

& $paths.Exe install
if ($LASTEXITCODE -ne 0) {
    throw "WinSW install failed (exit $LASTEXITCODE)"
}

# Automatic (Delayed) — sc to be sure
sc.exe config $ServiceName start= delayed-auto | Out-Null

[Environment]::SetEnvironmentVariable('KASA_HOST', $HostIp, 'Machine')
[Environment]::SetEnvironmentVariable('KASA_HOST', $HostIp, 'User')

Write-Host "Installed Windows service: $ServiceName (Automatic Delayed Start)"
Write-Host "  runs: $argDisplay"
Write-Host "  files: $($paths.Dir)"
Get-Service -Name $ServiceName | Format-List Name, Status, StartType

$shouldStart = $true
if (-not $StartNow) {
    # Default: start now so 20–80 is live after install
    $shouldStart = $true
}
if ($shouldStart) {
    Write-Host "Starting service..."
    Start-Service -Name $ServiceName
    Start-Sleep -Seconds 3
    Get-Service -Name $ServiceName | Format-List Name, Status, StartType
}
