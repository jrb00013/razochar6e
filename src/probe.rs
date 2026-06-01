use crate::backend::registry;
use crate::status::{find_batteries, read_battery};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ProbeReport {
    pub os: String,
    pub arch: String,
    pub wsl: bool,
    pub batteries: Vec<crate::status::BatteryStatus>,
    pub backends: Vec<BackendProbe>,
    pub recommended: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BackendProbe {
    pub id: String,
    pub name: String,
    pub available: bool,
    pub detail: String,
    pub supports_start_threshold: bool,
    pub supports_end_threshold: bool,
}

pub fn run_probe() -> ProbeReport {
    let wsl = is_wsl();
    let mut notes = Vec::new();

    if wsl {
        notes.push(
            "Running under WSL: sysfs shows a virtual battery. Use `razochar6e wsl probe` or the Windows helper for host control.".into(),
        );
    }

    let batteries: Vec<_> = find_batteries().iter().map(|p| read_battery(p)).collect();

    for bat in &batteries {
        if bat.virtual_battery {
            notes.push(format!(
                "Battery {} appears virtual ({:?}); charge limits may not apply to the physical pack.",
                bat.name, bat.model
            ));
        }
    }

    let backends: Vec<BackendProbe> = registry::probe_all()
        .into_iter()
        .map(|p| BackendProbe {
            id: p.kind.id().to_string(),
            name: p.kind.name().to_string(),
            available: p.available,
            detail: p.detail,
            supports_start_threshold: p.supports_start,
            supports_end_threshold: p.supports_end,
        })
        .collect();

    let recommended = backends.iter().find(|b| b.available).map(|b| b.id.clone());

    if recommended.is_none() && wsl {
        notes.push(
            "Install razochar6e on Windows (Admin) and use `razochar6e wsl set` from WSL.".into(),
        );
    }

    ProbeReport {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        wsl,
        batteries,
        backends,
        recommended,
        notes,
    }
}

pub fn sysfs_threshold_paths() -> Vec<(String, bool, bool)> {
    let mut out = Vec::new();
    for bat in find_batteries() {
        let start = bat.join("charge_control_start_threshold");
        let end = bat.join("charge_control_end_threshold");
        let legacy_start = bat.join("charge_start_threshold");
        let legacy_end = bat.join("charge_stop_threshold");
        let has_start = start.exists() || legacy_start.exists();
        let has_end = end.exists() || legacy_end.exists();
        if has_start || has_end {
            let name = bat
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("BAT?")
                .to_string();
            out.push((name, has_start, has_end));
        }
    }
    out
}

fn is_wsl() -> bool {
    if std::env::var("WSL_DISTRO_NAME").is_ok() {
        return true;
    }
    std::fs::read_to_string("/proc/version")
        .map(|v| v.to_lowercase().contains("microsoft"))
        .unwrap_or(false)
}

pub fn print_probe_human(report: &ProbeReport) {
    println!("razochar6e probe");
    println!("  OS: {} ({})", report.os, report.arch);
    println!("  WSL: {}", report.wsl);
    println!();
    println!("Batteries:");
    if report.batteries.is_empty() {
        println!("  (none)");
    }
    for b in &report.batteries {
        println!(
            "  {} — {}% {:?} model={:?}{}",
            b.name,
            b.capacity_percent
                .map(|c| c.to_string())
                .unwrap_or_else(|| "?".into()),
            b.status,
            b.model,
            if b.virtual_battery { " [virtual]" } else { "" }
        );
    }
    println!();
    println!("Backends:");
    for b in &report.backends {
        let mark = if b.available { "yes" } else { "no " };
        println!("  [{mark}] {} ({}) — {}", b.name, b.id, b.detail);
        if b.available {
            println!(
                "         start_threshold={} end_threshold={}",
                b.supports_start_threshold, b.supports_end_threshold
            );
        }
    }
    if let Some(r) = &report.recommended {
        println!();
        println!("Recommended: {r}");
    }
    if !report.notes.is_empty() {
        println!();
        println!("Notes:");
        for n in &report.notes {
            println!("  - {n}");
        }
    }

    #[cfg(unix)]
    {
        let paths = sysfs_threshold_paths();
        if !paths.is_empty() {
            println!();
            println!("Sysfs threshold files:");
            for (name, start, end) in paths {
                println!("  {name}: start={start} end={end}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_serializes() {
        let r = run_probe();
        let _ = serde_json::to_string(&r).unwrap();
    }
}
