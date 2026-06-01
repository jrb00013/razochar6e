mod backend;
mod cli;
mod completions;
mod config;
mod doctor;
mod error;
mod persist;
mod probe;
mod status;

use backend::{best_backend, Thresholds};
use clap::Parser;
use cli::{Cli, Commands, ConfigCommands, WslCommands};
use error::RazResult;
use probe::{print_probe_human, run_probe};

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
                let path = config::save(&config::AppConfig {
                    start,
                    end,
                    backend: backend.clone(),
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
    }
    Ok(())
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
            let path = config::save(&config::AppConfig {
                start,
                end,
                backend,
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
    #[cfg(not(unix))]
    {
        let _ = cmd;
        return Err(error::RazError::WslBridge(
            "WSL bridge only applies on Linux/WSL".into(),
        ));
    }

    #[cfg(unix)]
    match cmd {
        WslCommands::Probe => backend::wsl_bridge::wsl_probe()?,
        WslCommands::Status => backend::wsl_bridge::wsl_status()?,
        WslCommands::Set { start, end } => backend::wsl_bridge::wsl_set(Thresholds { start, end })?,
    }
    Ok(())
}
