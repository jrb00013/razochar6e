//! ASUS / ROG on Windows — IOCTL (ATKACPI) with WMI fallback via PowerShell.

use crate::backend::{ChargeBackend, Thresholds};
use crate::error::{RazError, RazResult};
use std::path::PathBuf;
use std::process::Command;

pub struct WindowsAsusBackend;

impl WindowsAsusBackend {
    pub fn open() -> Option<Self> {
        if Self::probe_detail().0 {
            Some(Self)
        } else {
            None
        }
    }

    pub fn probe_detail() -> (bool, String) {
        let script = script_path();
        if !script.exists() {
            return (false, format!("missing script: {}", script.display()));
        }
        let wmi_ok = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "try { Get-CimClass -Namespace root/WMI -ClassName AsusAtkWmi_WMNB -EA Stop | Out-Null; exit 0 } catch { exit 1 }",
            ])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if wmi_ok {
            (true, "ASUS WMI (AsusAtkWmi_WMNB) + ATKACPI script".into())
        } else {
            (
                true,
                format!(
                    "{} present (ATKACPI may still work with Admin)",
                    script.display()
                ),
            )
        }
    }

    fn run_script(percent: u8) -> RazResult<()> {
        let script = script_path();
        let out = Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                script.to_str().ok_or_else(|| RazError::Backend {
                    backend: "windows_asus".into(),
                    message: "non-UTF8 script path".into(),
                })?,
                "-Percent",
                &percent.to_string(),
            ])
            .output()
            .map_err(RazError::Io)?;
        if !out.status.success() {
            return Err(RazError::Backend {
                backend: "windows_asus".into(),
                message: format!(
                    "asus-battery-limit.ps1 failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                ),
            });
        }
        Ok(())
    }
}

fn script_path() -> PathBuf {
    let name = "asus-battery-limit.ps1";
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for candidate in [
                dir.join("scripts").join(name),
                dir.join(name),
                dir.parent().map(|p| p.join("scripts").join(name)),
            ]
            .into_iter()
            .flatten()
            {
                if candidate.exists() {
                    return candidate;
                }
            }
        }
    }
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = PathBuf::from(manifest).join("scripts").join(name);
        if p.exists() {
            return p;
        }
    }
    PathBuf::from("scripts").join(name)
}

impl ChargeBackend for WindowsAsusBackend {
    fn id(&self) -> &'static str {
        "windows_asus"
    }

    fn name(&self) -> &'static str {
        "Windows ASUS (PowerShell ATKACPI / WMI)"
    }

    fn set_thresholds(&self, t: Thresholds) -> RazResult<()> {
        t.validate()?;
        if t.start > 0 {
            eprintln!(
                "note: windows_asus only sets end limit {}; start {} ignored on most models",
                t.end, t.start
            );
        }
        Self::run_script(t.end)
    }

    fn get_thresholds(&self) -> RazResult<Option<Thresholds>> {
        Ok(None)
    }
}
