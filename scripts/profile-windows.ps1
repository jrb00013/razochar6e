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
        # Quiet + reachable: never sleep on AC; screen can blank.
        Set-PreferredScheme @('Silent', 'Balanced')
        powercfg /change standby-timeout-ac 0
        powercfg /change monitor-timeout-ac 10
        powercfg /change hibernate-timeout-ac 0
        powercfg /change standby-timeout-dc 30
        powercfg /change monitor-timeout-dc 5
        Write-Output 'remote: AC sleep=Never, display-off=10m, DC sleep=30m'
    }
    'desk' {
        Set-PreferredScheme @('Turbo', 'Performance')
        powercfg /change standby-timeout-ac 30
        powercfg /change monitor-timeout-ac 15
        powercfg /change hibernate-timeout-ac 0
        Write-Output 'desk: Turbo/Performance, AC sleep=30m, display-off=15m'
    }
    'away' {
        Set-PreferredScheme @('Silent', 'Balanced')
        powercfg /change monitor-timeout-ac 5
        Write-Output 'away: Silent/Balanced, display-off=5m (sleep when you sleep it -> sleepcut cuts Kasa)'
    }
}

Write-Output ("active: " + (powercfg /GetActiveScheme))
