//! Cut Kasa AC when Windows sleeps; restore on wake.
//!
//! Does **not** auto-sleep the machine — only reacts when *you* sleep it.
//! Overnight on-battery sleep naturally sheds a few % of SOC (the useful
//! shallow rest), without a forced 79→69 awake cycle.

use crate::error::{RazError, RazResult};
use crate::kasa::{self, KasaAuth};
use serde::Deserialize;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct SleepcutOpts {
    pub host: String,
    pub auth: KasaAuth,
    /// Turn the plug back on when Windows resumes (default true).
    pub restore_on_wake: bool,
}

#[derive(Debug, Deserialize)]
struct PowerEvent {
    event: String,
    #[allow(dead_code)]
    #[serde(default)]
    r#type: Option<u32>,
}

pub fn run(opts: SleepcutOpts) -> RazResult<()> {
    let script = find_watch_script()?;
    let script_win = to_windows_path(&script)?;

    println!(
        "sleepcut: watching Windows suspend/resume → Kasa {} (restore_on_wake={})",
        opts.host, opts.restore_on_wake
    );
    println!("sleepcut: sleep the laptop yourself; this only cuts/restores the outlet.");

    let mut child = Command::new(powershell_bin())
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &script_win,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| RazError::Backend {
            backend: "sleepcut".into(),
            message: format!("spawn powershell watch: {e}"),
        })?;

    let stdout = child.stdout.take().ok_or_else(|| RazError::Backend {
        backend: "sleepcut".into(),
        message: "powershell watch has no stdout".into(),
    })?;

    let reader = BufReader::new(stdout);
    let mut ready = false;
    let mut last_action = Instant::now() - Duration::from_secs(60);

    for line in reader.lines() {
        let line = line.map_err(RazError::Io)?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let ev: PowerEvent = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(_) => {
                eprintln!("sleepcut: ignore non-json line: {line}");
                continue;
            }
        };
        match ev.event.as_str() {
            "ready" => {
                ready = true;
                println!("sleepcut: power-event watcher ready");
            }
            "suspend" => {
                if !ready {
                    continue;
                }
                // Debounce duplicate suspend notifications.
                if last_action.elapsed() < Duration::from_secs(2) {
                    continue;
                }
                last_action = Instant::now();
                match kasa::set_on(&opts.host, false, &opts.auth) {
                    Ok(()) => println!("sleepcut: suspend → plug OFF"),
                    Err(e) => eprintln!("sleepcut: suspend cut failed: {e}"),
                }
            }
            "resume" => {
                if !ready || !opts.restore_on_wake {
                    continue;
                }
                if last_action.elapsed() < Duration::from_secs(2) {
                    continue;
                }
                last_action = Instant::now();
                match kasa::set_on(&opts.host, true, &opts.auth) {
                    Ok(()) => println!("sleepcut: resume → plug ON"),
                    Err(e) => eprintln!("sleepcut: resume restore failed: {e}"),
                }
            }
            other => eprintln!("sleepcut: unknown event {other}"),
        }
    }

    let status = child.wait().map_err(RazError::Io)?;
    if !status.success() {
        return Err(RazError::Backend {
            backend: "sleepcut".into(),
            message: format!("powershell watch exited {status}"),
        });
    }
    Ok(())
}

fn powershell_bin() -> &'static str {
    if cfg!(windows) {
        "powershell"
    } else {
        "powershell.exe"
    }
}

fn find_watch_script() -> RazResult<PathBuf> {
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = PathBuf::from(manifest).join("scripts/sleepcut-watch.ps1");
        if p.exists() {
            return Ok(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for cand in [
                dir.join("sleepcut-watch.ps1"),
                dir.join("../scripts/sleepcut-watch.ps1"),
                dir.join("../../scripts/sleepcut-watch.ps1"),
            ] {
                if cand.exists() {
                    return Ok(cand);
                }
            }
        }
    }
    for cand in [
        PathBuf::from("scripts/sleepcut-watch.ps1"),
        PathBuf::from("./scripts/sleepcut-watch.ps1"),
    ] {
        if cand.exists() {
            return Ok(cand);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home).join(".local/share/razochar6e/scripts/sleepcut-watch.ps1");
        if p.exists() {
            return Ok(p);
        }
    }
    Err(RazError::Backend {
        backend: "sleepcut".into(),
        message: "scripts/sleepcut-watch.ps1 not found".into(),
    })
}

fn to_windows_path(unix_or_win: &std::path::Path) -> RazResult<String> {
    if cfg!(windows) {
        return Ok(unix_or_win.display().to_string());
    }
    let is_wsl = std::env::var_os("WSL_DISTRO_NAME").is_some()
        || std::fs::read_to_string("/proc/version")
            .map(|v| v.to_lowercase().contains("microsoft"))
            .unwrap_or(false);
    if !is_wsl {
        return Ok(unix_or_win.display().to_string());
    }
    let out = Command::new("wslpath")
        .arg("-w")
        .arg(unix_or_win)
        .output()
        .map_err(|e| RazError::Backend {
            backend: "sleepcut".into(),
            message: format!("wslpath: {e}"),
        })?;
    if !out.status.success() {
        return Err(RazError::Backend {
            backend: "sleepcut".into(),
            message: format!("wslpath failed: {}", String::from_utf8_lossy(&out.stderr)),
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_suspend_event() {
        let e: PowerEvent = serde_json::from_str(r#"{"event":"suspend","type":4}"#).unwrap();
        assert_eq!(e.event, "suspend");
        assert_eq!(e.r#type, Some(4));
    }
}
