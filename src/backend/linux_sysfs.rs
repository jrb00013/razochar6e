use crate::backend::{ChargeBackend, Thresholds};
use crate::error::{RazError, RazResult};
use crate::status::find_batteries;
use std::fs;
use std::path::{Path, PathBuf};

pub struct LinuxSysfsBackend {
    battery: PathBuf,
    start_path: PathBuf,
    end_path: PathBuf,
}

impl LinuxSysfsBackend {
    pub fn discover() -> Option<Self> {
        for bat in find_batteries() {
            if let Some(backend) = Self::from_battery(&bat) {
                return Some(backend);
            }
        }
        None
    }

    fn from_battery(bat: &Path) -> Option<Self> {
        let start = Self::resolve_path(bat, "charge_control_start_threshold")
            .or_else(|| Self::resolve_path(bat, "charge_start_threshold"))?;
        let end = Self::resolve_path(bat, "charge_control_end_threshold")
            .or_else(|| Self::resolve_path(bat, "charge_stop_threshold"))?;
        Some(Self {
            battery: bat.to_path_buf(),
            start_path: start,
            end_path: end,
        })
    }

    fn resolve_path(bat: &Path, file: &str) -> Option<PathBuf> {
        let p = bat.join(file);
        if p.exists() {
            Some(p)
        } else {
            None
        }
    }

    fn write_threshold(path: &Path, value: u8) -> RazResult<()> {
        fs::write(path, format!("{value}\n")).map_err(|e| RazError::Backend {
            backend: "linux_sysfs".into(),
            message: format!("write {}: {e}", path.display()),
        })
    }

    fn read_threshold(path: &Path) -> RazResult<Option<u8>> {
        if !path.exists() {
            return Ok(None);
        }
        let s = fs::read_to_string(path).map_err(|e| RazError::Backend {
            backend: "linux_sysfs".into(),
            message: format!("read {}: {e}", path.display()),
        })?;
        let v: u8 = s.trim().parse().map_err(|_| RazError::Backend {
            backend: "linux_sysfs".into(),
            message: format!("parse {}: {:?}", path.display(), s),
        })?;
        Ok(Some(v))
    }

    pub fn probe_detail() -> (bool, String) {
        match Self::discover() {
            Some(b) => (
                true,
                format!(
                    "BAT {} — start={}, end={}",
                    b.battery
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("?"),
                    b.start_path.display(),
                    b.end_path.display()
                ),
            ),
            None => (
                false,
                "no charge_control_* or charge_*_threshold sysfs files".into(),
            ),
        }
    }
}

impl ChargeBackend for LinuxSysfsBackend {
    fn id(&self) -> &'static str {
        "linux_sysfs"
    }

    fn name(&self) -> &'static str {
        "Linux sysfs charge thresholds"
    }

    fn set_thresholds(&self, t: Thresholds) -> RazResult<()> {
        t.validate()?;
        Self::write_threshold(&self.start_path, t.start)?;
        Self::write_threshold(&self.end_path, t.end)?;
        Ok(())
    }

    fn get_thresholds(&self) -> RazResult<Option<Thresholds>> {
        let start = Self::read_threshold(&self.start_path)?;
        let end = Self::read_threshold(&self.end_path)?;
        match (start, end) {
            (Some(start), Some(end)) => Ok(Some(Thresholds { start, end })),
            _ => Ok(None),
        }
    }
}
