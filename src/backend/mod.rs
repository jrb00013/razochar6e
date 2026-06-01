pub mod linux_sysfs;
pub mod registry;

#[cfg(target_os = "macos")]
pub mod macos_cli;

#[cfg(windows)]
pub mod windows_asus;

#[cfg(windows)]
pub mod windows_wmi;

#[cfg(unix)]
pub mod wsl_bridge;

use crate::error::{RazError, RazResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Thresholds {
    pub start: u8,
    pub end: u8,
}

impl Thresholds {
    pub fn validate(&self) -> RazResult<()> {
        if self.start >= self.end {
            return Err(RazError::InvalidThreshold(format!(
                "start ({}) must be less than end ({})",
                self.start, self.end
            )));
        }
        if self.end > 100 || self.start > 99 {
            return Err(RazError::InvalidThreshold(
                "start must be 0–99, end must be 1–100".into(),
            ));
        }
        Ok(())
    }
}

pub trait ChargeBackend: Send + Sync {
    #[allow(dead_code)]
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn set_thresholds(&self, t: Thresholds) -> RazResult<()>;
    fn get_thresholds(&self) -> RazResult<Option<Thresholds>>;
    #[allow(dead_code)]
    fn clear(&self) -> RazResult<()> {
        self.set_thresholds(Thresholds { start: 0, end: 100 })
    }
}

pub fn best_backend() -> RazResult<Box<dyn ChargeBackend>> {
    registry::select_best().ok_or(RazError::NoBackend)
}
