//! ASUS / ROG on Windows — IOCTL (ATKACPI) with WMI fallback via PowerShell,
//! falling back further to the ASUSOptimization INI mechanism when neither
//! of those is reachable (see below for why that's needed on some models).

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
            let mut candidates = vec![dir.join("scripts").join(name), dir.join(name)];
            if let Some(parent) = dir.parent() {
                candidates.push(parent.join("scripts").join(name));
            }
            for candidate in candidates {
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

// --- ASUSOptimization INI fallback ---
//
// ASUS's ATK WMI interface (`AsusAtkWmi_WMNB`, method `DEVS`, device ID
// 0x00120057 — the same interface and DevID Linux's `asus-wmi` kernel
// driver uses) is real and callable, but on some models (verified: ROG
// Strix G18) its ACPI-WMI security descriptor rejects every caller except
// Armoury Crate's own SYSTEM service — confirmed by an identical
// WBEM_E_INVALID_PARAMETER from three independent, fully-elevated
// invocation stacks (raw COM, legacy .NET WMI, CIM cmdlets), even for the
// simplest possible call. That rules out a parameter/binding bug; it's an
// intentional access restriction on those models.
//
// Rather than escalate to raw EC port I/O (real risk of EC desync or
// hardware damage — not attempted here), this drives the exact mechanism
// Armoury Crate's own "ASUSOptimization" service already uses: a plain,
// world-writable INI file it polls, applied by restarting that service.
// This is the real electrical charge-cutoff (the EC genuinely stops
// delivering charge current at the threshold), just applied without the
// GUI running — verified live: MyASUS's own Battery Care Mode
// notification reported the limit set this way.
const ASUS_INI_PATH: &str =
    r"C:\ProgramData\ASUS\ASUS System Control Interface\AsusOptimization\Customization.ini";
const ASUS_SERVICE_NAME: &str = "ASUSOptimization";
const ASUS_INI_SECTION: &str = "[BatteryHealthCharging]";

/// Overridable via `RAZOCHAR6E_ASUS_INI` so tests can point at a temp file
/// instead of the real system path.
fn ini_path() -> String {
    std::env::var("RAZOCHAR6E_ASUS_INI").unwrap_or_else(|_| ASUS_INI_PATH.to_string())
}

fn read_ini(path: &str) -> RazResult<String> {
    std::fs::read_to_string(path).map_err(RazError::Io)
}

/// Rewrites the `value=` line under `[BatteryHealthCharging]` to `pct`,
/// leaving every other line untouched.
fn set_ini_value(content: &str, pct: u8) -> String {
    let mut out = Vec::with_capacity(content.lines().count());
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_section = trimmed.eq_ignore_ascii_case(ASUS_INI_SECTION);
            out.push(line.to_string());
        } else if in_section && trimmed.starts_with("value=") {
            out.push(format!("value={pct}"));
        } else {
            out.push(line.to_string());
        }
    }
    out.join("\r\n")
}

fn get_ini_value(content: &str) -> Option<u8> {
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_section = trimmed.eq_ignore_ascii_case(ASUS_INI_SECTION);
        } else if in_section {
            if let Some(v) = trimmed.strip_prefix("value=") {
                return v.trim().parse().ok();
            }
        }
    }
    None
}

/// Skipped when `RAZOCHAR6E_SKIP_SERVICE_RESTART` is set, so tests
/// exercise the INI-rewrite logic without touching a real service.
fn service_restart_skipped() -> bool {
    std::env::var("RAZOCHAR6E_SKIP_SERVICE_RESTART").is_ok()
}

fn stop_asus_optimization_service() -> RazResult<()> {
    if service_restart_skipped() {
        return Ok(());
    }
    // best-effort — sc.exe returns non-zero if the service is already
    // stopped, which isn't an error for our purposes.
    let _ = Command::new("sc.exe")
        .args(["stop", ASUS_SERVICE_NAME])
        .output()
        .map_err(RazError::Io)?;
    std::thread::sleep(std::time::Duration::from_secs(2));
    Ok(())
}

fn start_asus_optimization_service() -> RazResult<()> {
    if service_restart_skipped() {
        return Ok(());
    }
    let start = Command::new("sc.exe")
        .args(["start", ASUS_SERVICE_NAME])
        .output()
        .map_err(RazError::Io)?;
    if !start.status.success() {
        return Err(RazError::Backend {
            backend: "windows_asus".into(),
            message: format!(
                "sc.exe start {ASUS_SERVICE_NAME} failed: {}",
                String::from_utf8_lossy(&start.stderr)
            ),
        });
    }
    Ok(())
}

fn set_limit_via_ini(pct: u8) -> RazResult<()> {
    // Order matters: ASUSOptimization periodically flushes its own
    // internal state back into this file. Writing while it's running
    // races that flush — the service can clobber our write within
    // seconds even though the write itself reports success. Stopping it
    // first eliminates the race: nothing else can write the file while
    // it's down, so the value we write is guaranteed to be what the
    // service reads back on start. Verified empirically — write-then-
    // restart silently reverted within ~3s; stop-then-write-then-start
    // held stable through a 10s settle.
    let path = ini_path();
    let backup_path = format!("{path}.razochar6e-backup");
    if !std::path::Path::new(&backup_path).exists() {
        std::fs::copy(&path, &backup_path).map_err(RazError::Io)?;
    }
    stop_asus_optimization_service()?;
    let content = read_ini(&path)?;
    let updated = set_ini_value(&content, pct);
    std::fs::write(&path, updated).map_err(RazError::Io)?;
    start_asus_optimization_service()
}

fn get_limit_via_ini() -> RazResult<Option<u8>> {
    let content = read_ini(&ini_path())?;
    Ok(get_ini_value(&content))
}

impl ChargeBackend for WindowsAsusBackend {
    fn id(&self) -> &'static str {
        "windows_asus"
    }

    fn name(&self) -> &'static str {
        "Windows ASUS (PowerShell ATKACPI / WMI, INI fallback)"
    }

    fn set_thresholds(&self, t: Thresholds) -> RazResult<()> {
        t.validate()?;
        if t.start > 0 {
            eprintln!(
                "note: windows_asus only sets end limit {}; start {} ignored on most models",
                t.end, t.start
            );
        }
        match Self::run_script(t.end) {
            Ok(()) => Ok(()),
            Err(script_err) => set_limit_via_ini(t.end).map_err(|ini_err| RazError::Backend {
                backend: "windows_asus".into(),
                message: format!(
                    "IOCTL/WMI failed ({script_err}); INI fallback also failed ({ini_err})"
                ),
            }),
        }
    }

    fn get_thresholds(&self) -> RazResult<Option<Thresholds>> {
        match get_limit_via_ini() {
            Ok(Some(end)) => Ok(Some(Thresholds { start: 0, end })),
            Ok(None) => Ok(None),
            Err(_) => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_INI: &str = "[Main]\r\nlegal=1\r\n[BatteryHealthCharging]\r\nversion=3\r\nvalue=80\r\n[TaskFirst]\r\nvalue=0\r\n";

    #[test]
    fn set_ini_value_rewrites_only_target_section() {
        let updated = set_ini_value(SAMPLE_INI, 60);
        assert!(updated.contains("[BatteryHealthCharging]\r\nversion=3\r\nvalue=60"));
        assert!(updated.contains("[TaskFirst]\r\nvalue=0"));
    }

    #[test]
    fn get_ini_value_reads_target_section_only() {
        assert_eq!(get_ini_value(SAMPLE_INI), Some(80));
    }

    #[test]
    fn get_ini_value_none_when_section_missing() {
        assert_eq!(get_ini_value("[Main]\r\nlegal=1\r\n"), None);
    }
}
