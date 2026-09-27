//! Battery capacity across Linux sysfs and Windows/WSL (WMI).

use crate::error::{RazError, RazResult};
use crate::status::{find_batteries, read_battery};
use std::thread;
use std::time::Duration;

/// Best-effort battery charge percentage for the cycle loop.
pub fn battery_percent() -> RazResult<u8> {
    battery_percent_with_retries(3, Duration::from_millis(400))
}

fn battery_percent_with_retries(attempts: u32, delay: Duration) -> RazResult<u8> {
    let last = String::from("Windows WMI returned empty/failed");
    for i in 0..attempts {
        if let Some(pct) = linux_real_battery_percent() {
            return Ok(pct);
        }
        if let Some(pct) = windows_battery_percent() {
            return Ok(pct);
        }
        if i + 1 < attempts {
            thread::sleep(delay);
        }
    }
    Err(RazError::Backend {
        backend: "battery".into(),
        message: format!(
            "could not read battery percentage after {attempts} tries (no real sysfs battery; {last})"
        ),
    })
}

fn linux_real_battery_percent() -> Option<u8> {
    for path in find_batteries() {
        let s = read_battery(&path);
        if s.virtual_battery {
            continue;
        }
        if let Some(pct) = s.capacity_percent {
            return Some(pct);
        }
    }
    None
}

fn windows_battery_percent() -> Option<u8> {
    let ps = if cfg!(windows) {
        "powershell"
    } else if std::env::var_os("WSL_DISTRO_NAME").is_some()
        || std::fs::read_to_string("/proc/version")
            .map(|v| v.to_lowercase().contains("microsoft"))
            .unwrap_or(false)
    {
        "powershell.exe"
    } else {
        return None;
    };

    let script = r#"
$ErrorActionPreference = 'Stop'
$b = Get-CimInstance Win32_Battery | Select-Object -First 1
if ($null -eq $b) { exit 2 }
$p = $b.EstimatedChargeRemaining
if ($null -eq $p -or "$p" -eq '') { exit 3 }
Write-Output $p
"#;

    let out = std::process::Command::new(ps)
        .args(["-NoProfile", "-Command", script])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    // CIM sometimes prints extra lines; take the last integer token.
    trimmed
        .lines()
        .rev()
        .find_map(|line| line.trim().parse::<u8>().ok())
}

#[cfg(test)]
mod tests {
    use crate::status::BatteryStatus;

    fn is_usable(s: &BatteryStatus) -> bool {
        !s.virtual_battery && s.capacity_percent.is_some()
    }

    #[test]
    fn rejects_virtual_battery() {
        let s = BatteryStatus {
            name: "BAT1".into(),
            capacity_percent: Some(50),
            status: None,
            on_ac: Some(false),
            manufacturer: Some("Microsoft".into()),
            model: Some("Microsoft Hyper-V Virtual Battery".into()),
            virtual_battery: true,
        };
        assert!(!is_usable(&s));
    }
}
