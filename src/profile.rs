//! Power profiles: remote (work from afar), desk (local use), away (sleepcut).

use crate::backend::{self, Thresholds};
use crate::config::{self, AppConfig};
use crate::error::{RazError, RazResult};
use crate::kasa::{self, KasaAuth};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// 10h remote work: plugged in, quiet, never sleep on AC, no drain-cycle.
    Remote,
    /// Local use / gaming: Turbo plan when available, plug on.
    Desk,
    /// Gone: quiet plan; sleepcut cuts Kasa when *you* sleep Windows.
    Away,
}

impl Profile {
    pub fn parse(s: &str) -> RazResult<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "remote" => Ok(Self::Remote),
            "desk" => Ok(Self::Desk),
            "away" => Ok(Self::Away),
            other => Err(RazError::Backend {
                backend: "profile".into(),
                message: format!("unknown profile '{other}' (use remote|desk|away)"),
            }),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Remote => "remote",
            Self::Desk => "desk",
            Self::Away => "away",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProfileOpts {
    pub profile: Profile,
    pub host: Option<String>,
    pub auth: KasaAuth,
    /// Apply charge thresholds from config (default true).
    pub apply_thresholds: bool,
    /// Save active_profile into config (default true).
    pub save: bool,
}

pub fn run(opts: ProfileOpts) -> RazResult<()> {
    let mut cfg = config::load().unwrap_or_default();
    let host = resolve_optional_host(opts.host.clone(), cfg.kasa_host.clone(), &opts.auth);

    println!("profile: applying '{}'", opts.profile.as_str());

    match opts.profile {
        Profile::Remote => apply_remote(&cfg, host.as_deref(), &opts.auth)?,
        Profile::Desk => apply_desk(&cfg, host.as_deref(), &opts.auth)?,
        Profile::Away => apply_away(&cfg, host.as_deref(), &opts.auth)?,
    }

    if opts.apply_thresholds {
        match backend::registry::apply_thresholds(
            Thresholds {
                start: cfg.start,
                end: cfg.end,
            },
            cfg.backend.as_deref(),
        ) {
            Ok(()) => println!("profile: charge band {}–{}% applied", cfg.start, cfg.end),
            Err(e) => eprintln!("profile: charge thresholds skipped: {e}"),
        }
    }

    apply_windows_power(opts.profile)?;

    if opts.save {
        cfg.active_profile = Some(opts.profile.as_str().to_string());
        if let Some(h) = host {
            cfg.kasa_host = Some(h);
        }
        let path = config::save(&cfg)?;
        println!("profile: saved active_profile to {}", path.display());
    }

    print_followups(opts.profile);
    Ok(())
}

pub fn show() -> RazResult<()> {
    let cfg = config::load().unwrap_or_default();
    println!(
        "active_profile: {}",
        cfg.active_profile.as_deref().unwrap_or("(unset)")
    );
    println!(
        "kasa_host: {}",
        cfg.kasa_host.as_deref().unwrap_or("(unset)")
    );
    println!("charge band: {}–{}%", cfg.start, cfg.end);
    println!();
    println!("Profiles:");
    println!("  remote — work from afar: Kasa ON, Silent/Balanced, AC sleep=Never, display 10m");
    println!("  desk   — local use: Kasa ON, Turbo/Performance when available");
    println!("  away   — leave: Silent; run sleepcut so sleep → Kasa OFF");
    Ok(())
}

fn apply_remote(cfg: &AppConfig, host: Option<&str>, auth: &KasaAuth) -> RazResult<()> {
    println!("profile/remote: keep awake + plugged in (no drain cycle)");
    ensure_plug_on(host, auth)?;
    let _ = cfg;
    Ok(())
}

fn apply_desk(cfg: &AppConfig, host: Option<&str>, auth: &KasaAuth) -> RazResult<()> {
    println!("profile/desk: local performance — Kasa ON");
    ensure_plug_on(host, auth)?;
    let _ = cfg;
    Ok(())
}

fn apply_away(cfg: &AppConfig, host: Option<&str>, auth: &KasaAuth) -> RazResult<()> {
    println!("profile/away: quiet plan; sleepcut cuts Kasa when Windows sleeps");
    // Leave plug alone if already on — sleepcut handles OFF on suspend.
    if let Some(h) = host {
        match kasa::is_on(h, auth) {
            Ok(on) => println!(
                "profile/away: plug {h} currently {}",
                if on { "ON" } else { "OFF" }
            ),
            Err(e) => eprintln!("profile/away: could not read plug: {e}"),
        }
    }
    let _ = cfg;
    Ok(())
}

fn ensure_plug_on(host: Option<&str>, auth: &KasaAuth) -> RazResult<()> {
    let Some(h) = host else {
        println!("profile: no kasa_host — skip plug (set --host or config kasa_host)");
        return Ok(());
    };
    kasa::set_on(h, true, auth)?;
    println!("profile: plug {h} ON");
    Ok(())
}

fn resolve_optional_host(
    explicit: Option<String>,
    config_host: Option<String>,
    auth: &KasaAuth,
) -> Option<String> {
    if let Some(h) = explicit.or(config_host) {
        if !h.is_empty() {
            return Some(h);
        }
    }
    match kasa::discover(auth) {
        Ok(found) if found.len() == 1 => Some(found[0].host.clone()),
        _ => None,
    }
}

fn apply_windows_power(profile: Profile) -> RazResult<()> {
    let script = find_profile_script()?;
    let script_arg = to_windows_path_if_wsl(&script)?;
    let ps = powershell_bin();
    let out = Command::new(ps)
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &script_arg,
            "-Profile",
            profile.as_str(),
        ])
        .output()
        .map_err(|e| RazError::Backend {
            backend: "profile".into(),
            message: format!("powershell: {e}"),
        })?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    for line in stdout.lines().filter(|l| !l.trim().is_empty()) {
        println!("profile/win: {line}");
    }
    if !out.status.success() {
        return Err(RazError::Backend {
            backend: "profile".into(),
            message: format!(
                "profile-windows.ps1 failed ({}): {}",
                out.status,
                if stderr.trim().is_empty() {
                    stdout.trim()
                } else {
                    stderr.trim()
                }
            ),
        });
    }
    Ok(())
}

fn print_followups(profile: Profile) {
    match profile {
        Profile::Remote => {
            println!();
            println!("Next (remote day):");
            println!("  • Do NOT run `razochar6e cycle` — stay on AC");
            println!("  • sleepcut may stay running; it only cuts Kasa if Windows *sleeps*");
            println!("  • Prefer Hybrid/Eco GPU in Armoury while remoting");
        }
        Profile::Desk => {
            println!();
            println!("Next (desk): use Armoury Turbo/Ultimate if you need max GPU.");
        }
        Profile::Away => {
            println!();
            println!("Next (away):");
            println!("  • Keep `razochar6e sleepcut` running (sleep → Kasa OFF, wake → ON)");
            println!("  • Sleep Windows when you leave (no auto-sleep from this profile)");
        }
    }
}

fn powershell_bin() -> &'static str {
    if cfg!(windows) {
        "powershell"
    } else {
        "powershell.exe"
    }
}

fn find_profile_script() -> RazResult<PathBuf> {
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = PathBuf::from(manifest).join("scripts/profile-windows.ps1");
        if p.exists() {
            return Ok(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for cand in [
                dir.join("profile-windows.ps1"),
                dir.join("../scripts/profile-windows.ps1"),
                dir.join("../../scripts/profile-windows.ps1"),
            ] {
                if cand.exists() {
                    return Ok(cand);
                }
            }
        }
    }
    for cand in [
        PathBuf::from("scripts/profile-windows.ps1"),
        PathBuf::from("./scripts/profile-windows.ps1"),
    ] {
        if cand.exists() {
            return Ok(cand);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home).join(".local/share/razochar6e/scripts/profile-windows.ps1");
        if p.exists() {
            return Ok(p);
        }
    }
    Err(RazError::Backend {
        backend: "profile".into(),
        message: "scripts/profile-windows.ps1 not found".into(),
    })
}

fn to_windows_path_if_wsl(path: &std::path::Path) -> RazResult<String> {
    if cfg!(windows) {
        return Ok(path.display().to_string());
    }
    let is_wsl = std::env::var_os("WSL_DISTRO_NAME").is_some()
        || std::fs::read_to_string("/proc/version")
            .map(|v| v.to_lowercase().contains("microsoft"))
            .unwrap_or(false);
    if !is_wsl {
        return Ok(path.display().to_string());
    }
    let out = Command::new("wslpath")
        .arg("-w")
        .arg(path)
        .output()
        .map_err(|e| RazError::Backend {
            backend: "profile".into(),
            message: format!("wslpath: {e}"),
        })?;
    if !out.status.success() {
        return Err(RazError::Backend {
            backend: "profile".into(),
            message: format!("wslpath failed: {}", String::from_utf8_lossy(&out.stderr)),
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_names() {
        assert_eq!(Profile::parse("remote").unwrap(), Profile::Remote);
        assert_eq!(Profile::parse("DESK").unwrap(), Profile::Desk);
        assert_eq!(Profile::parse("away").unwrap(), Profile::Away);
        assert!(Profile::parse("turbo").is_err());
    }
}
