//! Smart-plug charge/drain cycle: cut AC at end%, restore at start%.

use crate::battery;
use crate::error::{RazError, RazResult};
use crate::kasa::{self, KasaAuth};
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    TurnOn,
    TurnOff,
    Hold,
}

/// Hysteresis band: only switch at the edges.
pub fn decide(pct: u8, start: u8, end: u8, relay_on: bool) -> Action {
    if pct >= end && relay_on {
        Action::TurnOff
    } else if pct <= start && !relay_on {
        Action::TurnOn
    } else {
        Action::Hold
    }
}

pub struct CycleOpts {
    pub host: String,
    pub start: u8,
    pub end: u8,
    pub interval: Duration,
    pub once: bool,
    pub auth: KasaAuth,
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

fn tick(opts: &CycleOpts) -> RazResult<()> {
    let pct = battery::battery_percent()?;
    let on = kasa::is_on(&opts.host, &opts.auth)?;
    let action = decide(pct, opts.start, opts.end, on);
    match action {
        Action::TurnOff => {
            println!("{pct}% ≥ {}% and outlet ON → cutting AC", opts.end);
            kasa::set_on(&opts.host, false, &opts.auth)?;
        }
        Action::TurnOn => {
            println!("{pct}% ≤ {}% and outlet OFF → restoring AC", opts.start);
            kasa::set_on(&opts.host, true, &opts.auth)?;
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

    loop {
        match tick(&opts) {
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
    }

    #[test]
    fn holds_at_start_if_already_on() {
        assert_eq!(decide(15, 20, 80, true), Action::Hold);
    }
}
