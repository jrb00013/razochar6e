//! Battery capacity across Linux sysfs and Windows/WSL (WMI).

use crate::error::{RazError, RazResult};
use crate::status::{find_batteries, read_battery};

/// Best-effort battery charge percentage for the cycle loop.
pub fn battery_percent() -> RazResult<u8> {
    if let Some(pct) = linux_real_battery_percent() {
        return Ok(pct);
    }
    if let Some(pct) = windows_battery_percent() {
        return Ok(pct);
    }
    Err(RazError::Backend {
        backend: "battery".into(),
        message: "could not read battery percentage (no real sysfs battery and Windows WMI failed)"
            .into(),
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

    let out = std::process::Command::new(ps)
        .args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_Battery | Select-Object -First 1 -ExpandProperty EstimatedChargeRemaining)",
        ])
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
    trimmed.parse().ok()
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
