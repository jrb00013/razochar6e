//! WMI fallback for ASUS charge limit when ATKACPI IOCTL is unavailable.

use crate::backend::{ChargeBackend, Thresholds};
use crate::error::{RazError, RazResult};
use std::process::Command;

pub struct WindowsWmiAsusBackend;

impl WindowsWmiAsusBackend {
    pub fn open() -> Option<Self> {
        if Self::probe_detail().0 {
            Some(Self)
        } else {
            None
        }
    }

    pub fn probe_detail() -> (bool, String) {
        let script = r#"
try {
  Get-CimClass -Namespace root/WMI -ClassName AsusAtkWmi_WMNB -ErrorAction Stop | Out-Null
  exit 0
} catch { exit 1 }
"#;
        let ok = Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            (
                true,
                "WMI AsusAtkWmi_WMNB available (fallback when ATKACPI missing)".into(),
            )
        } else {
            (false, "AsusAtkWmi_WMNB WMI class not found".into())
        }
    }

    fn set_limit(&self, percent: u8) -> RazResult<()> {
        let script = format!(
            r#"
$val = {percent}
$result = (Get-WmiObject -Namespace root/WMI -Class AsusAtkWmi_WMNB).DEVS(0x00120057, $val)
if ($null -eq $result) {{ throw "DEVS returned null" }}
"#
        );
        let out = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .map_err(RazError::Io)?;
        if !out.status.success() {
            return Err(RazError::Backend {
                backend: "windows_wmi_asus".into(),
                message: format!("WMI DEVS failed: {}", String::from_utf8_lossy(&out.stderr)),
            });
        }
        Ok(())
    }
}

impl ChargeBackend for WindowsWmiAsusBackend {
    fn id(&self) -> &'static str {
        "windows_wmi_asus"
    }

    fn name(&self) -> &'static str {
        "Windows ASUS WMI (AsusAtkWmi_WMNB)"
    }

    fn set_thresholds(&self, t: Thresholds) -> RazResult<()> {
        t.validate()?;
        if t.start > 0 {
            eprintln!(
                "note: windows_wmi_asus only sets end limit {}; start {} ignored",
                t.end, t.start
            );
        }
        self.set_limit(t.end)
    }

    fn get_thresholds(&self) -> RazResult<Option<Thresholds>> {
        Ok(None)
    }
}
