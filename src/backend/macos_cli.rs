//! macOS: delegate to installed SMC tools (`batt`, `battery`, `bclm`) when present.

use crate::backend::{ChargeBackend, Thresholds};
use crate::error::{RazError, RazResult};
use std::process::Command;

#[derive(Clone)]
enum MacTool {
    Batt,
    BatteryMaintain,
    Bclm,
}

pub struct MacOsCliBackend {
    tool: MacTool,
}

impl MacOsCliBackend {
    pub fn discover() -> Option<Self> {
        if which_exists("batt") {
            return Some(Self {
                tool: MacTool::Batt,
            });
        }
        if which_exists("battery") {
            return Some(Self {
                tool: MacTool::BatteryMaintain,
            });
        }
        if which_exists("bclm") {
            return Some(Self {
                tool: MacTool::Bclm,
            });
        }
        None
    }

    pub fn probe_detail() -> (bool, String) {
        if let Some(b) = Self::discover() {
            (
                true,
                format!(
                    "found {:?} — Apple Silicon may only support 80/100 via bclm",
                    b.tool
                ),
            )
        } else {
            (false, "install batt, battery, or bclm (see README)".into())
        }
    }

    fn run_sudo(cmd: &str, args: &[&str]) -> RazResult<String> {
        let out = Command::new("sudo")
            .arg(cmd)
            .args(args)
            .output()
            .map_err(RazError::Io)?;
        if !out.status.success() {
            return Err(RazError::Backend {
                backend: "macos_cli".into(),
                message: format!(
                    "sudo {} {} failed: {}",
                    cmd,
                    args.join(" "),
                    String::from_utf8_lossy(&out.stderr)
                ),
            });
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }
}

fn which_exists(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

impl ChargeBackend for MacOsCliBackend {
    fn id(&self) -> &'static str {
        "macos_cli"
    }

    fn name(&self) -> &'static str {
        "macOS SMC CLI"
    }

    fn set_thresholds(&self, t: Thresholds) -> RazResult<()> {
        t.validate()?;
        match self.tool {
            MacTool::Batt => {
                Self::run_sudo("batt", &["limit", &t.end.to_string()])?;
            }
            MacTool::BatteryMaintain => {
                let range = format!("{}-{}", t.start, t.end);
                Self::run_sudo("battery", &["maintain", &range])?;
            }
            MacTool::Bclm => {
                // Intel: ~3% overshoot; Apple Silicon often 80/100 only
                let write = if t.end <= 83 { 80 } else { 100 };
                Self::run_sudo("bclm", &["write", &write.to_string()])?;
            }
        }
        Ok(())
    }

    fn get_thresholds(&self) -> RazResult<Option<Thresholds>> {
        Ok(None)
    }
}
