use crate::backend::Thresholds;
use crate::error::{RazError, RazResult};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const DEFAULT_START: u8 = 20;
pub const DEFAULT_END: u8 = 80;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_start")]
    pub start: u8,
    #[serde(default = "default_end")]
    pub end: u8,
    #[serde(default)]
    pub backend: Option<String>,
}

fn default_start() -> u8 {
    DEFAULT_START
}

fn default_end() -> u8 {
    DEFAULT_END
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            start: DEFAULT_START,
            end: DEFAULT_END,
            backend: None,
        }
    }
}

impl AppConfig {
    pub fn thresholds(&self) -> Thresholds {
        Thresholds {
            start: self.start,
            end: self.end,
        }
    }

    pub fn validate(&self) -> RazResult<()> {
        self.thresholds().validate()
    }
}

pub fn config_dir() -> Option<PathBuf> {
    ProjectDirs::from("com", "razochar6e", "razochar6e").map(|d| d.config_dir().to_path_buf())
}

pub fn config_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("config.toml"))
}

pub fn load() -> RazResult<AppConfig> {
    let Some(path) = config_path() else {
        return Ok(AppConfig::default());
    };
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| RazError::Backend {
        backend: "config".into(),
        message: format!("read {}: {e}", path.display()),
    })?;
    let cfg: AppConfig = toml::from_str(&raw).map_err(|e| RazError::Backend {
        backend: "config".into(),
        message: format!("parse {}: {e}", path.display()),
    })?;
    cfg.validate()?;
    Ok(cfg)
}

pub fn save(cfg: &AppConfig) -> RazResult<PathBuf> {
    cfg.validate()?;
    let dir = config_dir().ok_or_else(|| RazError::Backend {
        backend: "config".into(),
        message: "cannot resolve config directory".into(),
    })?;
    fs::create_dir_all(&dir).map_err(RazError::Io)?;
    let path = dir.join("config.toml");
    let body = format!(
        "# razochar6e — battery charge band while on AC\n# Docs: https://github.com/jrb00013/razochar6e\n\n{}",
        toml::to_string_pretty(cfg).map_err(|e| RazError::Backend {
            backend: "config".into(),
            message: e.to_string(),
        })?
    );
    fs::write(&path, body).map_err(RazError::Io)?;
    Ok(path)
}

pub fn init_example() -> RazResult<PathBuf> {
    save(&AppConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_thresholds_valid() {
        AppConfig::default().validate().unwrap();
    }

    #[test]
    fn rejects_inverted_band() {
        let cfg = AppConfig {
            start: 90,
            end: 80,
            backend: None,
        };
        assert!(cfg.validate().is_err());
    }
}
