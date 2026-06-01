//! Invoke the Windows host `razochar6e.exe` from WSL via PowerShell.

use crate::backend::{ChargeBackend, Thresholds};
use crate::error::{RazError, RazResult};
use std::process::Command;

const HOST_SCRIPT: &str = "scripts/razochar6e-host.ps1";

pub struct WslBridgeBackend;

impl WslBridgeBackend {
    pub fn probe_detail() -> (bool, String) {
        if !is_wsl() {
            return (false, "not running under WSL".into());
        }
        match Command::new("powershell.exe").arg("-?").output() {
            Ok(o) if o.status.success() || !o.stdout.is_empty() => (
                true,
                "powershell.exe available — use `razochar6e wsl` subcommands".into(),
            ),
            _ => (false, "powershell.exe not found".into()),
        }
    }

    fn run_host(args: &[&str]) -> RazResult<String> {
        let script = find_host_script()?;
        let mut cmd_args = vec!["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", &script];
        cmd_args.extend(args);

        let out = Command::new("powershell.exe")
            .args(&cmd_args)
            .output()
            .map_err(|e| RazError::WslBridge(format!("powershell.exe: {e}")))?;

        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();

        if !out.status.success() {
            return Err(RazError::WslBridge(format!(
                "host command failed ({}): {stderr}{stdout}",
                out.status
            )));
        }
        Ok(stdout)
    }
}

fn is_wsl() -> bool {
    std::env::var("WSL_DISTRO_NAME").is_ok()
        || std::fs::read_to_string("/proc/version")
            .map(|v| v.to_lowercase().contains("microsoft"))
            .unwrap_or(false)
}

fn find_host_script() -> RazResult<String> {
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = format!("{manifest}/{HOST_SCRIPT}");
        if std::path::Path::new(&p).exists() {
            return windows_path(&p);
        }
    }
    for candidate in [
        "./scripts/razochar6e-host.ps1",
        "../razochar6e/scripts/razochar6e-host.ps1",
    ] {
        if std::path::Path::new(candidate).exists() {
            return windows_path(candidate);
        }
    }
    Err(RazError::WslBridge(format!(
        "host script not found; expected {HOST_SCRIPT} in repo"
    )))
}

fn windows_path(unix_path: &str) -> RazResult<String> {
    let out = Command::new("wslpath")
        .arg("-w")
        .arg(unix_path)
        .output()
        .map_err(|e| RazError::WslBridge(format!("wslpath: {e}")))?;
    if !out.status.success() {
        return Err(RazError::WslBridge("wslpath failed".into()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

impl ChargeBackend for WslBridgeBackend {
    fn id(&self) -> &'static str {
        "wsl_bridge"
    }

    fn name(&self) -> &'static str {
        "WSL → Windows host"
    }

    fn set_thresholds(&self, t: Thresholds) -> RazResult<()> {
        t.validate()?;
        let out = WslBridgeBackend::run_host(&["set", &t.start.to_string(), &t.end.to_string()])?;
        print!("{out}");
        Ok(())
    }

    fn get_thresholds(&self) -> RazResult<Option<Thresholds>> {
        Ok(None)
    }
}

pub fn wsl_probe() -> RazResult<()> {
    let out = WslBridgeBackend::run_host(&["probe"])?;
    print!("{out}");
    Ok(())
}

pub fn wsl_status() -> RazResult<()> {
    let out = WslBridgeBackend::run_host(&["status"])?;
    print!("{out}");
    Ok(())
}

pub fn wsl_set(t: Thresholds) -> RazResult<()> {
    let b = WslBridgeBackend;
    b.set_thresholds(t)
}
