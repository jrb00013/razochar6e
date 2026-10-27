//! Smart-plug charge/drain cycle: cut AC at end%, restore at start%.

use crate::battery;
use crate::error::{RazError, RazResult};
use crate::kasa::{self, KasaAuth};
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Minimum time after any plug change before AC may be cut again.
pub const DEFAULT_MIN_DWELL_SECS: u64 = 600;
/// Defer an AC cut while GPU utilisation is at/above this percent.
pub const DEFAULT_MAX_LOAD_PCT: u8 = 40;
/// Consecutive battery-read failures before the failsafe forces AC on, so a
/// single WMI blip cannot flip the plug.
const FAILSAFE_AFTER_FAILURES: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    TurnOn,
    TurnOff,
    Hold,
}

/// Hysteresis band: only switch at the edges.
///
/// Upper cut uses 2% slack (`pct >= end - 2`) so firmware/WMI that stick at
/// 78–79% under an 80% charge ceiling still cut AC (user-tuned for ASUS).
pub fn decide(pct: u8, start: u8, end: u8, relay_on: bool) -> Action {
    let cut_at = end.saturating_sub(2);
    if pct >= cut_at && relay_on {
        Action::TurnOff
    } else if pct <= start && !relay_on {
        Action::TurnOn
    } else {
        Action::Hold
    }
}

/// Why an AC cut was postponed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Defer {
    Dwell,
    Load(u8),
}

/// Gate an AC cut. Restoring AC is never gated: it is the safe direction.
///
/// - `since_change`: time since the plug last changed state (`None` = never).
/// - `load_pct`: GPU utilisation, `None` when unreadable (then the cut is allowed).
/// - `max_load_pct == 0` disables the load gate.
pub fn gate_cut(
    since_change: Option<Duration>,
    min_dwell: Duration,
    load_pct: Option<u8>,
    max_load_pct: u8,
) -> Result<(), Defer> {
    if since_change.is_some_and(|d| d < min_dwell) {
        return Err(Defer::Dwell);
    }
    match load_pct {
        Some(l) if max_load_pct > 0 && l >= max_load_pct => Err(Defer::Load(l)),
        _ => Ok(()),
    }
}

/// Whether enough consecutive battery reads have failed to force AC on.
pub fn failsafe_due(consecutive_failures: u32) -> bool {
    consecutive_failures >= FAILSAFE_AFTER_FAILURES
}

#[derive(Default)]
struct CycleState {
    last_change: Option<Instant>,
    battery_failures: u32,
}

pub struct CycleOpts {
    pub host: String,
    pub start: u8,
    pub end: u8,
    pub interval: Duration,
    pub min_dwell: Duration,
    pub max_load_pct: u8,
    pub once: bool,
    pub auth: KasaAuth,
}

/// Parse GPU utilisation text: the highest integer line, clamped to 100.
/// Windows counters can sum above 100 across processes.
pub fn parse_load(text: &str) -> Option<u8> {
    text.lines()
        .filter_map(|l| l.trim().parse::<u32>().ok())
        .max()
        .map(|v| v.min(100) as u8)
}

/// GPU utilisation: `nvidia-smi` first, then the Windows 3D-engine counter.
///
/// `nvidia-smi` often cannot initialise NVML inside WSL, which would leave the load
/// gate silently disabled; the Windows counter works from WSL via powershell.exe.
fn gpu_load_pct() -> Option<u8> {
    nvidia_smi_load_pct().or_else(windows_gpu_load_pct)
}

fn windows_gpu_load_pct() -> Option<u8> {
    let ps = if cfg!(windows) {
        "powershell"
    } else {
        "powershell.exe"
    };
    let script = r#"$ErrorActionPreference='Stop'
$s = (Get-Counter '\GPU Engine(*engtype_3D)\Utilization Percentage').CounterSamples | Measure-Object CookedValue -Sum
[math]::Round($s.Sum)"#;
    let out = std::process::Command::new(ps)
        .args(["-NoProfile", "-Command", script])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_load(&String::from_utf8_lossy(&out.stdout))
}

/// Highest GPU utilisation across NVIDIA GPUs; `None` if nvidia-smi is unavailable.
fn nvidia_smi_load_pct() -> Option<u8> {
    ["nvidia-smi", "nvidia-smi.exe"].iter().find_map(|bin| {
        let out = std::process::Command::new(bin)
            .args([
                "--query-gpu=utilization.gpu",
                "--format=csv,noheader,nounits",
            ])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        parse_load(&String::from_utf8_lossy(&out.stdout))
    })
}

/// Append a line to the persistent cycle log (survives reboots, unlike /tmp).
fn log_event(msg: &str) {
    let Some(dirs) = directories::ProjectDirs::from("", "", "razochar6e") else {
        return;
    };
    let dir = dirs
        .state_dir()
        .unwrap_or_else(|| dirs.data_local_dir())
        .to_path_buf();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("cycle.log"))
    {
        let _ = writeln!(f, "ts={ts} {msg}");
    }
}

pub fn resolve_host(
    explicit: Option<String>,
    config_host: Option<String>,
    auth: &KasaAuth,
) -> RazResult<String> {
    if let Some(h) = explicit.or(config_host) {
        if !h.is_empty() {
            return Ok(h);
        }
    }
    let found = kasa::discover(auth)?;
    match found.as_slice() {
        [only] => Ok(only.host.clone()),
        [] => Err(RazError::Backend {
            backend: "cycle".into(),
            message: "no Kasa plug found — pass --host IP or set kasa_host in config".into(),
        }),
        many => Err(RazError::Backend {
            backend: "cycle".into(),
            message: format!(
                "multiple plugs found ({}); pass --host explicitly: {}",
                many.len(),
                many.iter()
                    .map(|p| p.host.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }),
    }
}

fn tick(opts: &CycleOpts, state: &mut CycleState) -> RazResult<()> {
    let pct = battery::battery_percent()?;
    state.battery_failures = 0;
    let on = kasa::is_on(&opts.host, &opts.auth)?;
    let action = decide(pct, opts.start, opts.end, on);
    match action {
        Action::TurnOff => {
            let load = if opts.max_load_pct > 0 {
                gpu_load_pct()
            } else {
                None
            };
            let since = state.last_change.map(|t| t.elapsed());
            match gate_cut(since, opts.min_dwell, load, opts.max_load_pct) {
                Ok(()) => {
                    println!(
                        "{pct}% at/above cut (≥{}%, end={}%) and outlet ON → cutting AC",
                        opts.end.saturating_sub(2),
                        opts.end
                    );
                    kasa::set_on(&opts.host, false, &opts.auth)?;
                    state.last_change = Some(Instant::now());
                    log_event(&format!("cut AC pct={pct} load={load:?}"));
                }
                Err(why) => {
                    let reason = match why {
                        Defer::Dwell => "min dwell since last plug change".to_string(),
                        Defer::Load(l) => format!("GPU load {l}% ≥ {}%", opts.max_load_pct),
                    };
                    println!("{pct}% at/above cut but deferring AC cut: {reason}");
                    log_event(&format!("defer cut pct={pct} reason={reason}"));
                }
            }
        }
        Action::TurnOn => {
            println!("{pct}% ≤ {}% and outlet OFF → restoring AC", opts.start);
            kasa::set_on(&opts.host, true, &opts.auth)?;
            state.last_change = Some(Instant::now());
            log_event(&format!("restore AC pct={pct}"));
        }
        Action::Hold => {
            println!(
                "{pct}% in band, outlet={} → hold",
                if on { "ON" } else { "OFF" }
            );
        }
    }
    let _ = io::stdout().flush();
    Ok(())
}

fn is_battery_error(e: &RazError) -> bool {
    match e {
        RazError::Backend { backend, .. } => backend == "battery" || backend.starts_with("battery"),
        _ => {
            let s = e.to_string();
            s.contains("battery") || s.contains("WMI")
        }
    }
}

/// When SOC is unknown, never leave AC cut — restore the outlet.
///
/// Returns `true` when the plug was switched on (so the caller can restart the dwell timer).
pub fn failsafe_restore_ac(host: &str, auth: &KasaAuth) -> RazResult<bool> {
    match kasa::is_on(host, auth) {
        Ok(true) => {
            eprintln!("cycle: failsafe — battery unread, outlet already ON");
            Ok(false)
        }
        Ok(false) => {
            eprintln!("cycle: failsafe — battery unread and outlet OFF → forcing AC ON");
            kasa::set_on(host, true, auth)?;
            println!("cycle: failsafe restored plug {host} ON");
            Ok(true)
        }
        Err(read_err) => {
            eprintln!("cycle: failsafe — cannot read plug ({read_err}); forcing AC ON");
            kasa::set_on(host, true, auth)?;
            println!("cycle: failsafe restored plug {host} ON");
            Ok(true)
        }
    }
}

pub fn run(opts: CycleOpts) -> RazResult<()> {
    if opts.start >= opts.end {
        return Err(RazError::InvalidThreshold(format!(
            "start ({}) must be < end ({})",
            opts.start, opts.end
        )));
    }

    println!(
        "cycle: host={} band={}-{}% interval={}s{}",
        opts.host,
        opts.start,
        opts.end,
        opts.interval.as_secs(),
        if opts.once { " (once)" } else { "" }
    );
    let _ = io::stdout().flush();

    let mut state = CycleState::default();
    loop {
        match tick(&opts, &mut state) {
            Ok(()) => {
                if opts.once {
                    break;
                }
            }
            Err(e) => {
                // Never abort the long-running loop on transient WMI / plug errors —
                // that previously left the laptop discharging with no restore.
                eprintln!("cycle: transient error (will retry): {e}");
                let _ = io::stderr().flush();
                // Critical: unknown battery % must not strand us on battery with AC cut.
                if is_battery_error(&e) {
                    state.battery_failures += 1;
                    log_event(&format!(
                        "battery read failed ({}x): {e}",
                        state.battery_failures
                    ));
                    if failsafe_due(state.battery_failures) {
                        match failsafe_restore_ac(&opts.host, &opts.auth) {
                            Ok(true) => {
                                state.last_change = Some(Instant::now());
                                log_event("failsafe restored AC");
                            }
                            Ok(false) => {}
                            Err(fe) => eprintln!("cycle: failsafe restore failed: {fe}"),
                        }
                    }
                }
                if opts.once {
                    return Err(e);
                }
            }
        }
        if opts.once {
            break;
        }
        thread::sleep(opts.interval);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cuts_at_end_when_on() {
        assert_eq!(decide(80, 20, 80, true), Action::TurnOff);
        assert_eq!(decide(95, 20, 80, true), Action::TurnOff);
    }

    #[test]
    fn cuts_at_end_minus_two_slack() {
        // WMI/ASUS often sit at 78–79% when the limit is 80%.
        assert_eq!(decide(78, 20, 80, true), Action::TurnOff);
        assert_eq!(decide(79, 20, 80, true), Action::TurnOff);
        assert_eq!(decide(77, 20, 80, true), Action::Hold);
    }

    #[test]
    fn restores_at_start_when_off() {
        assert_eq!(decide(20, 20, 80, false), Action::TurnOn);
        assert_eq!(decide(10, 20, 80, false), Action::TurnOn);
    }

    #[test]
    fn holds_in_middle() {
        assert_eq!(decide(50, 20, 80, true), Action::Hold);
        assert_eq!(decide(50, 20, 80, false), Action::Hold);
    }

    #[test]
    fn holds_at_end_if_already_off() {
        assert_eq!(decide(85, 20, 80, false), Action::Hold);
        assert_eq!(decide(79, 20, 80, false), Action::Hold);
    }

    #[test]
    fn holds_at_start_if_already_on() {
        assert_eq!(decide(15, 20, 80, true), Action::Hold);
    }

    const DWELL: Duration = Duration::from_secs(600);

    #[test]
    fn cut_deferred_within_dwell() {
        // The 03:09–03:30 chatter: a failsafe restore followed by an immediate re-cut.
        let r = gate_cut(Some(Duration::from_secs(60)), DWELL, None, 40);
        assert_eq!(r, Err(Defer::Dwell));
    }

    #[test]
    fn cut_allowed_after_dwell_or_when_never_changed() {
        assert_eq!(
            gate_cut(Some(Duration::from_secs(601)), DWELL, None, 40),
            Ok(())
        );
        assert_eq!(gate_cut(None, DWELL, None, 40), Ok(()));
    }

    #[test]
    fn cut_deferred_under_gpu_load() {
        assert_eq!(gate_cut(None, DWELL, Some(85), 40), Err(Defer::Load(85)));
        assert_eq!(gate_cut(None, DWELL, Some(40), 40), Err(Defer::Load(40)));
        assert_eq!(gate_cut(None, DWELL, Some(39), 40), Ok(()));
    }

    #[test]
    fn load_gate_disabled_or_unreadable_allows_cut() {
        assert_eq!(gate_cut(None, DWELL, Some(100), 0), Ok(()));
        assert_eq!(gate_cut(None, DWELL, None, 40), Ok(()));
    }

    #[test]
    fn parse_load_takes_max_and_clamps() {
        assert_eq!(parse_load("12\n87\n"), Some(87));
        assert_eq!(parse_load("104\n"), Some(100));
        assert_eq!(parse_load("Failed to initialize NVML: N/A\n"), None);
        assert_eq!(parse_load(""), None);
    }

    #[test]
    fn failsafe_needs_consecutive_failures() {
        assert!(!failsafe_due(1));
        assert!(!failsafe_due(2));
        assert!(failsafe_due(3));
    }

    #[test]
    fn battery_errors_are_detected_for_failsafe() {
        let e = RazError::Backend {
            backend: "battery".into(),
            message: "could not read battery percentage".into(),
        };
        assert!(is_battery_error(&e));
        let e2 = RazError::Backend {
            backend: "kasa".into(),
            message: "timeout".into(),
        };
        assert!(!is_battery_error(&e2));
    }
}
