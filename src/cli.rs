use clap::{CommandFactory, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "razochar6e",
    version,
    author,
    about = "Battery charge scheduling — stop at end %, resume below start %",
    long_about = "Set firmware-backed charge thresholds so your laptop stops charging above \
                  an upper limit (default 80%) and resumes below a lower limit (default 20%). \
                  Supports Linux sysfs, Windows ASUS/ROG, macOS SMC tools, and WSL→Windows bridge."
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

pub fn build_cli() -> clap::Command {
    Cli::command()
}
