# Apply Windows power settings for razochar6e power profiles.
# Usage: profile-windows.ps1 -Profile remote|desk|away
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('remote', 'desk', 'away')]
    [string]$Profile
)

$ErrorActionPreference = 'Continue'

function Find-SchemeGuid([string]$preferredName) {
    $list = powercfg /L 2>$null
    foreach ($line in $list) {
        if ($line -match 'Power Scheme GUID:\s+([0-9a-fA-F-]+)\s+\(([^)]+)\)') {
            if ($Matches[2] -eq $preferredName) {
                return $Matches[1]
            }
        }
    }
    return $null
}

function Set-PreferredScheme([string[]]$names) {
    foreach ($n in $names) {
        $guid = Find-SchemeGuid $n
        if ($null -ne $guid) {
            powercfg /setactive $guid | Out-Null
            Write-Output "scheme: $n ($guid)"
            return
        }
    }
    Write-Output ("scheme: none of [" + ($names -join ', ') + "] found - leaving current plan")
}

switch ($Profile) {
    'remote' {
        # Quiet + reachable. Never sleep on AC *or* DC — remoting on battery
        # (or a blip off AC) must not nap the box after 30 minutes.
        Set-PreferredScheme @('Silent', 'Balanced')
        powercfg /change standby-timeout-ac 0
        powercfg /change standby-timeout-dc 0
        powercfg /change monitor-timeout-ac 10
        powercfg /change monitor-timeout-dc 5
        powercfg /change hibernate-timeout-ac 0
        powercfg /change hibernate-timeout-dc 0
        Write-Output 'remote: AC+DC sleep=Never, display-off=10m AC / 5m DC'
    }
    'desk' {
        # Desk / 20–80 Kasa cycle: cutting AC puts the laptop on DC, so AC and
        # DC sleep must match — otherwise the box naps mid-drain on battery.
        Set-PreferredScheme @('Turbo', 'Performance')
        powercfg /change standby-timeout-ac 120
        powercfg /change standby-timeout-dc 120
        powercfg /change hibernate-timeout-ac 0
        powercfg /change hibernate-timeout-dc 0
        powercfg /change monitor-timeout-ac 15
        powercfg /change monitor-timeout-dc 15
        powercfg /SETACVALUEINDEX SCHEME_CURRENT SUB_SLEEP HYBRIDSLEEP 0 2>$null
        powercfg /SETDCVALUEINDEX SCHEME_CURRENT SUB_SLEEP HYBRIDSLEEP 0 2>$null
        powercfg /SETACTIVE SCHEME_CURRENT | Out-Null
        Write-Output 'desk: Turbo/Performance, AC+DC sleep=2h, display-off=15m'
    }
    'away' {
        Set-PreferredScheme @('Silent', 'Balanced')
        powercfg /change monitor-timeout-ac 5
        Write-Output 'away: Silent/Balanced, display-off=5m (sleep when you sleep it -> sleepcut cuts Kasa)'
    }
}

Write-Output ("active: " + (powercfg /GetActiveScheme))
