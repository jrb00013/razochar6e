mod backend;
mod battery;
mod benchmark;
mod cli;
mod completions;
mod config;
mod cycle;
mod doctor;
mod error;
mod kasa;
mod persist;
mod probe;
mod sleepcut;
mod status;

use backend::{best_backend, Thresholds};
use clap::Parser;
use cli::{Cli, Commands, ConfigCommands, CycleAction, WslCommands};
use error::RazResult;
use probe::{print_probe_human, run_probe};
use std::time::Duration;

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> RazResult<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Probe { json } => {
            let report = run_probe();
            if json {
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
            } else {
                print_probe_human(&report);
            }
        }
        Commands::Doctor => {
            std::process::exit(doctor::run());
        }
        Commands::Apply { backend } => {
            let cfg = config::load()?;
            backend::registry::apply_thresholds(
                cfg.thresholds(),
                backend.as_deref().or(cfg.backend.as_deref()),
            )?;
        }
        Commands::Set {
            start,
            end,
            backend,
            save,
        } => {
            let t = Thresholds { start, end };
            if save {
                let existing = config::load().unwrap_or_default();
                let path = config::save(&config::AppConfig {
                    start,
                    end,
                    backend: backend.clone(),
                    kasa_host: existing.kasa_host,
                })?;
                println!("Saved config to {}", path.display());
            }
            backend::registry::apply_thresholds(t, backend.as_deref())?;
        }
        Commands::Status => cmd_status()?,
        Commands::Clear { backend } => {
            let b = match backend.as_deref() {
                Some(id) => backend::registry::backend_by_id(id)?,
                None => best_backend()?,
            };
            b.clear()?;
            println!("Charge limits cleared (0–100%) via {}", b.name());
        }
        Commands::Config(cmd) => cmd_config(cmd)?,
        Commands::InstallPersist { start, end } => persist::install(start, end)?,
        Commands::UninstallPersist => persist::uninstall()?,
        Commands::Completions { shell } => completions::generate_for(shell)?,
        Commands::Wsl(cmd) => cmd_wsl(cmd)?,
        Commands::Cycle {
            action,
            host,
            start,
            end,
            interval,
            once,
            discover,
            username,
            password,
            save,
        } => cmd_cycle(
            action, host, start, end, interval, once, discover, username, password, save,
        )?,
        Commands::Sleepcut {
            host,
            username,
            password,
            no_restore,
            save,
        } => cmd_sleepcut(host, username, password, no_restore, save)?,
        Commands::Benchmark {
            rate,
            capacity_wh,
            daily_wh,
            hours_away,
            samples,
            apply,
        } => {
            let opts = benchmark::BenchmarkOpts {
                rate_per_kwh: rate,
                capacity_wh,
                daily_wh,
                hours_away,
                samples,
                apply,
            };
            let report = benchmark::run_benchmark(opts.clone())?;
            benchmark::print_report(&report, &opts);
        }
    }
    Ok(())
}

fn cmd_cycle(
    action: Option<CycleAction>,
    host: Option<String>,
    start: u8,
    end: u8,
    interval: u64,
    once: bool,
    discover: bool,
    username: Option<String>,
    password: Option<String>,
    save: bool,
) -> RazResult<()> {
    let auth = kasa::KasaAuth::from_env_and_opts(username, password);
    let cfg = config::load().unwrap_or_default();

    if discover {
        let plugs = kasa::discover(&auth)?;
        if plugs.is_empty() {
            println!("No Kasa plugs discovered on the LAN.");
            println!("Tip: pass --host IP, and for KLAP plugs set KASA_USERNAME / KASA_PASSWORD.");
            return Ok(());
        }
        for p in plugs {
            println!(
                "{}  alias={:?} model={:?} on={:?}",
                p.host, p.alias, p.model, p.is_on
            );
        }
        return Ok(());
    }

    let host = cycle::resolve_host(host, cfg.kasa_host.clone(), &auth)?;

    if save {
        let path = config::save(&config::AppConfig {
            start,
            end,
            backend: cfg.backend.clone(),
            kasa_host: Some(host.clone()),
        })?;
        println!("Saved kasa_host={host} to {}", path.display());
    }

    match action {
        Some(CycleAction::On) => {
            kasa::set_on(&host, true, &auth)?;
            println!("Plug {host} ON");
        }
        Some(CycleAction::Off) => {
            kasa::set_on(&host, false, &auth)?;
            println!("Plug {host} OFF");
        }
        Some(CycleAction::State) => {
            let on = kasa::is_on(&host, &auth)?;
            println!("{}", serde_json::json!({ "host": host, "is_on": on }));
        }
        None => cycle::run(cycle::CycleOpts {
            host,
            start,
            end,
            interval: Duration::from_secs(interval.max(1)),
            once,
            auth,
        })?,
    }
    Ok(())
}

fn cmd_sleepcut(
    host: Option<String>,
    username: Option<String>,
    password: Option<String>,
    no_restore: bool,
    save: bool,
) -> RazResult<()> {
    let auth = kasa::KasaAuth::from_env_and_opts(username, password);
    let cfg = config::load().unwrap_or_default();
    let host = cycle::resolve_host(host, cfg.kasa_host.clone(), &auth)?;
    if save {
        let path = config::save(&config::AppConfig {
            start: cfg.start,
            end: cfg.end,
            backend: cfg.backend.clone(),
            kasa_host: Some(host.clone()),
        })?;
        println!("Saved kasa_host={host} to {}", path.display());
    }
    sleepcut::run(sleepcut::SleepcutOpts {
        host,
        auth,
        restore_on_wake: !no_restore,
    })
}

fn cmd_config(cmd: ConfigCommands) -> RazResult<()> {
    match cmd {
        ConfigCommands::Init => {
            let path = config::init_example()?;
            println!("Created {}", path.display());
        }
        ConfigCommands::Show => {
            if let Some(p) = config::config_path() {
                println!("Path: {}", p.display());
                if p.exists() {
                    print!("{}", std::fs::read_to_string(&p)?);
                } else {
                    println!("(file does not exist — run `razochar6e config init`)");
                }
            } else {
                println!("Config directory unavailable on this platform.");
            }
        }
        ConfigCommands::Set {
            start,
            end,
            backend,
        } => {
            let existing = config::load().unwrap_or_default();
            let path = config::save(&config::AppConfig {
                start,
                end,
                backend,
                kasa_host: existing.kasa_host,
            })?;
            println!("Updated {}", path.display());
        }
    }
    Ok(())
}

fn cmd_status() -> RazResult<()> {
    let batteries = status::find_batteries();
    if batteries.is_empty() {
        println!("No batteries found.");
    }
    for path in &batteries {
        let s = status::read_battery(path);
        println!(
            "{}: {}% status={:?} AC={:?} model={:?}",
            s.name,
            s.capacity_percent.unwrap_or(0),
            s.status,
            s.on_ac,
            s.model
        );
    }

    if let Ok(backend) = best_backend() {
        println!("Backend: {} [{}]", backend.name(), backend.id());
        match backend.get_thresholds()? {
            Some(t) => println!("Thresholds: start={}% end={}%", t.start, t.end),
            None => println!("Thresholds: (not readable from hardware)"),
        }
    } else {
        println!("No charge-limit backend available on this host.");
        #[cfg(unix)]
        if std::env::var("WSL_DISTRO_NAME").is_ok() {
            println!("Try: razochar6e wsl status");
        }
    }

    if let Ok(cfg) = config::load() {
        if let Some(p) = config::config_path() {
            if p.exists() {
                println!(
                    "Config: start={}% end={}% ({})",
                    cfg.start,
                    cfg.end,
                    p.display()
                );
            }
        }
    }
    Ok(())
}

fn cmd_wsl(cmd: WslCommands) -> RazResult<()> {
    #[cfg(unix)]
    {
        match cmd {
            WslCommands::Probe => backend::wsl_bridge::wsl_probe()?,
            WslCommands::Status => backend::wsl_bridge::wsl_status()?,
            WslCommands::Set { start, end } => {
                backend::wsl_bridge::wsl_set(Thresholds { start, end })?
            }
        }
        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = cmd;
        Err(error::RazError::WslBridge(
            "WSL bridge only applies on Linux/WSL".into(),
        ))
    }
}
