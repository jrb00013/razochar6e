use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize)]
pub struct BatteryStatus {
    pub name: String,
    pub capacity_percent: Option<u8>,
    pub status: Option<String>,
    pub on_ac: Option<bool>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub virtual_battery: bool,
}

pub fn find_batteries() -> Vec<PathBuf> {
    let root = Path::new("/sys/class/power_supply");
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            fs::read_to_string(p.join("type"))
                .map(|t| t.trim() == "Battery")
                .unwrap_or(false)
        })
        .collect()
}

pub fn read_battery(path: &Path) -> BatteryStatus {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("BAT?")
        .to_string();

    let read = |f: &str| -> Option<String> {
        fs::read_to_string(path.join(f))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };

    let capacity_percent = read("capacity").and_then(|s| s.parse().ok());
    let status = read("status");
    let manufacturer = read("manufacturer");
    let model = read("model_name");

    let virtual_battery = model
        .as_deref()
        .map(|m| m.contains("Virtual") || m.contains("Hyper-V"))
        .unwrap_or(false)
        || manufacturer.as_deref() == Some("");

    let on_ac = detect_on_ac();

    BatteryStatus {
        name,
        capacity_percent,
        status,
        on_ac,
        manufacturer,
        model,
        virtual_battery,
    }
}

fn detect_on_ac() -> Option<bool> {
    let root = Path::new("/sys/class/power_supply");
    let entries = fs::read_dir(root).ok()?;
    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        let t = fs::read_to_string(p.join("type")).ok()?;
        if t.trim() != "Mains" && t.trim() != "USB" {
            continue;
        }
        if let Ok(online) = fs::read_to_string(p.join("online")) {
            if online.trim() == "1" {
                return Some(true);
            }
        }
    }
    Some(false)
}
