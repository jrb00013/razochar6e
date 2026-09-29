use clap::{CommandFactory, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "razochar6e",
    version,
    author,
    about = "Battery charge scheduling — stop at end %, resume below start %",
    long_about = "Set firmware-backed charge thresholds so your laptop stops charging above \
                  an upper limit (default 80%) and resumes below a lower limit (default 20%). \
                  Supports Linux sysfs, Windows ASUS/ROG, macOS SMC tools, and WSL→Windows bridge. \
                  Optional `cycle` drives a TP-Link Kasa smart plug to cut/restore AC for a full \
                  charge–drain band. `sleepcut` cuts the plug when *you* sleep Windows and \
                  restores it on wake (no auto-sleep)."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Detect OS, batteries, and available charge-limit backends
    Probe {
        #[arg(long)]
        json: bool,
    },
    /// Run health checks (exit 1 if issues found)
    Doctor,
    /// Apply thresholds from ~/.config/razochar6e/config.toml
    Apply {
        #[arg(long)]
        backend: Option<String>,
    },
    /// Apply charge thresholds (requires root / Admin / supported hardware)
    Set {
        #[arg(long, default_value_t = crate::config::DEFAULT_START)]
        start: u8,
        #[arg(long, default_value_t = crate::config::DEFAULT_END)]
        end: u8,
        #[arg(long)]
        backend: Option<String>,
        /// Save values to config file
        #[arg(long)]
        save: bool,
    },
    /// Show battery status and current thresholds when readable
    Status,
    /// Reset to full charging (start=0, end=100)
    Clear {
        #[arg(long)]
        backend: Option<String>,
    },
    /// Manage ~/.config/razochar6e/config.toml
    #[command(subcommand)]
    Config(ConfigCommands),
    /// Install boot/login persistence for thresholds
    InstallPersist {
        #[arg(long, default_value_t = crate::config::DEFAULT_START)]
        start: u8,
        #[arg(long, default_value_t = crate::config::DEFAULT_END)]
        end: u8,
    },
    /// Remove persistence unit/task
    UninstallPersist,
    /// Generate shell completions
    Completions {
        #[arg(value_enum)]
        shell: crate::completions::ShellKind,
    },
    /// WSL: control Windows host battery via PowerShell bridge
    #[command(subcommand)]
    Wsl(WslCommands),
    /// Charge/drain cycle via Kasa smart plug (cut AC at end%, restore at start%)
    Cycle {
        #[command(subcommand)]
        action: Option<CycleAction>,
        /// Kasa plug IP (or set `kasa_host` in config / discover)
        #[arg(long, global = true)]
        host: Option<String>,
        #[arg(long, global = true, default_value_t = crate::config::DEFAULT_START)]
        start: u8,
        #[arg(long, global = true, default_value_t = crate::config::DEFAULT_END)]
        end: u8,
        /// Seconds between polls
        #[arg(long, global = true, default_value_t = 60)]
        interval: u64,
        /// Run a single poll+action then exit
        #[arg(long, global = true)]
        once: bool,
        /// List plugs on the LAN and exit
        #[arg(long, global = true)]
        discover: bool,
        /// TP-Link account email (or env KASA_USERNAME) for KLAP plugs
        #[arg(long, global = true, env = "KASA_USERNAME")]
        username: Option<String>,
        /// TP-Link account password (or env KASA_PASSWORD)
        #[arg(long, global = true, env = "KASA_PASSWORD")]
        password: Option<String>,
        /// Save `kasa_host` (and start/end) into config
        #[arg(long, global = true)]
        save: bool,
    },
    /// Cut Kasa AC when Windows sleeps; restore on wake (you sleep the PC — no auto-sleep)
    Sleepcut {
        /// Kasa plug IP (or `kasa_host` in config / discover)
        #[arg(long)]
        host: Option<String>,
        /// TP-Link account email (or env KASA_USERNAME) for KLAP plugs
        #[arg(long, env = "KASA_USERNAME")]
        username: Option<String>,
        /// TP-Link account password (or env KASA_PASSWORD)
        #[arg(long, env = "KASA_PASSWORD")]
        password: Option<String>,
        /// Leave the plug off after wake (default: restore ON)
        #[arg(long)]
        no_restore: bool,
        /// Save `kasa_host` into config
        #[arg(long)]
        save: bool,
    },
    /// Logistic-regression benchmark: recommend start/end for $/power vs stranding risk
    Benchmark {
        /// Electricity rate in $/kWh
        #[arg(long, default_value_t = 0.16)]
        rate: f64,
        /// Battery capacity in Wh
        #[arg(long, default_value_t = 90.0)]
        capacity_wh: f64,
        /// Estimated daily energy drawn from the pack (Wh)
        #[arg(long, default_value_t = 40.0)]
        daily_wh: f64,
        /// Hours per day the machine must run without AC
        #[arg(long, default_value_t = 2.0)]
        hours_away: f64,
        /// Synthetic samples for the logistic fit
        #[arg(long, default_value_t = 400)]
        samples: usize,
        /// Write recommended start/end into config
        #[arg(long)]
        apply: bool,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Write default config.toml
    Init,
    /// Print config path and contents
    Show,
    /// Set start/end in config without applying to hardware
    Set {
        #[arg(long, default_value_t = crate::config::DEFAULT_START)]
        start: u8,
        #[arg(long, default_value_t = crate::config::DEFAULT_END)]
        end: u8,
        #[arg(long)]
        backend: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum WslCommands {
    Probe,
    Status,
    Set {
        #[arg(long, default_value_t = crate::config::DEFAULT_START)]
        start: u8,
        #[arg(long, default_value_t = crate::config::DEFAULT_END)]
        end: u8,
    },
}

#[derive(Subcommand)]
pub enum CycleAction {
    /// Turn the plug on (restore AC)
    On,
    /// Turn the plug off (cut AC)
    Off,
    /// Print plug on/off state as JSON
    State,
}

pub fn build_cli() -> clap::Command {
    Cli::command()
}
