use clap::ValueEnum;
use clap_complete::{generate, shells};
use std::io;

#[derive(Clone, ValueEnum)]
pub enum ShellKind {
    Bash,
    Elvish,
    Fish,
    PowerShell,
    Zsh,
}

pub fn generate_for(shell: ShellKind) -> io::Result<()> {
    let mut cmd = crate::cli::build_cli();
    let bin_name = cmd.get_name().to_string();
    match shell {
        ShellKind::Bash => generate(shells::Bash, &mut cmd, bin_name, &mut io::stdout()),
        ShellKind::Elvish => generate(shells::Elvish, &mut cmd, bin_name, &mut io::stdout()),
        ShellKind::Fish => generate(shells::Fish, &mut cmd, bin_name, &mut io::stdout()),
        ShellKind::PowerShell => {
            generate(shells::PowerShell, &mut cmd, bin_name, &mut io::stdout())
        }
        ShellKind::Zsh => generate(shells::Zsh, &mut cmd, bin_name, &mut io::stdout()),
    }
    Ok(())
}
