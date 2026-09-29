# Emits one JSON line per Windows power event for razochar6e sleepcut.
# EventType 4 = Entering Suspend, 7 = Resume from Suspend.
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

Register-WmiEvent -Query 'SELECT * FROM Win32_PowerManagementEvent' -SourceIdentifier RazSleepCut | Out-Null
Write-Output '{"event":"ready"}'
[Console]::Out.Flush()

try {
    while ($true) {
        $ev = Wait-Event -SourceIdentifier RazSleepCut
        try {
            $t = [int]$ev.SourceEventArgs.NewEvent.EventType
            $name = switch ($t) {
                4 { 'suspend' }
                7 { 'resume' }
                default { $null }
            }
            if ($null -ne $name) {
                Write-Output ("{`"event`":`"$name`",`"type`":$t}")
                [Console]::Out.Flush()
            }
        } finally {
            Remove-Event -EventIdentifier $ev.EventIdentifier -ErrorAction SilentlyContinue
        }
    }
} finally {
    Unregister-Event -SourceIdentifier RazSleepCut -ErrorAction SilentlyContinue
    Get-Event -SourceIdentifier RazSleepCut -ErrorAction SilentlyContinue | Remove-Event -ErrorAction SilentlyContinue
}
