use crate::backend::linux_sysfs::LinuxSysfsBackend;
use crate::backend::{best_backend, ChargeBackend, Thresholds};
use crate::error::{RazError, RazResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    LinuxSysfs,
    #[cfg(windows)]
    WindowsAsus,
    #[cfg(windows)]
    WindowsWmiAsus,
    #[cfg(target_os = "macos")]
    MacOsCli,
    #[cfg(unix)]
    WslBridge,
}

impl BackendKind {
    pub fn id(self) -> &'static str {
        match self {
            Self::LinuxSysfs => "linux_sysfs",
            #[cfg(windows)]
            Self::WindowsAsus => "windows_asus",
            #[cfg(windows)]
            Self::WindowsWmiAsus => "windows_wmi_asus",
            #[cfg(target_os = "macos")]
            Self::MacOsCli => "macos_cli",
            #[cfg(unix)]
            Self::WslBridge => "wsl_bridge",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::LinuxSysfs => "Linux sysfs",
            #[cfg(windows)]
            Self::WindowsAsus => "Windows ASUS ATKACPI",
            #[cfg(windows)]
            Self::WindowsWmiAsus => "Windows ASUS WMI",
            #[cfg(target_os = "macos")]
            Self::MacOsCli => "macOS SMC CLI wrapper",
            #[cfg(unix)]
            Self::WslBridge => "WSL → Windows host bridge",
        }
    }
}

pub struct BackendProbeResult {
    pub kind: BackendKind,
    pub available: bool,
    pub detail: String,
    pub supports_start: bool,
    pub supports_end: bool,
}

pub fn probe_all() -> Vec<BackendProbeResult> {
    let mut out = Vec::new();

    let (avail, detail) = LinuxSysfsBackend::probe_detail();
    out.push(BackendProbeResult {
        kind: BackendKind::LinuxSysfs,
        available: avail,
        detail,
        supports_start: avail,
        supports_end: avail,
    });

    #[cfg(windows)]
    {
        let (avail, detail) = super::windows_asus::probe_detail();
        out.push(BackendProbeResult {
            kind: BackendKind::WindowsAsus,
            available: avail,
            detail,
            supports_start: false,
            supports_end: avail,
        });
        let (wmi_avail, wmi_detail) = super::windows_wmi::WindowsWmiAsusBackend::probe_detail();
        out.push(BackendProbeResult {
            kind: BackendKind::WindowsWmiAsus,
            available: wmi_avail && !avail,
            detail: if avail {
                format!("skipped (ATKACPI preferred): {wmi_detail}")
            } else {
                wmi_detail
            },
            supports_start: false,
            supports_end: wmi_avail,
        });
    }

    #[cfg(target_os = "macos")]
    {
        let (avail, detail) = super::macos_cli::MacOsCliBackend::probe_detail();
        out.push(BackendProbeResult {
            kind: BackendKind::MacOsCli,
            available: avail,
            detail,
            supports_start: avail,
            supports_end: avail,
        });
    }

    #[cfg(unix)]
    {
        let (avail, detail) = super::wsl_bridge::WslBridgeBackend::probe_detail();
        out.push(BackendProbeResult {
            kind: BackendKind::WslBridge,
            available: avail,
            detail,
            supports_start: false,
            supports_end: true,
        });
    }

    out
}

pub fn select_best() -> Option<Box<dyn ChargeBackend>> {
    #[cfg(not(windows))]
    {
        if let Some(b) = LinuxSysfsBackend::discover() {
            return Some(Box::new(b));
        }
    }

    #[cfg(windows)]
    {
        if let Some(b) = super::windows_asus::WindowsAsusBackend::open() {
            return Some(Box::new(b));
        }
        if let Some(b) = super::windows_wmi::WindowsWmiAsusBackend::open() {
            return Some(Box::new(b));
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(b) = super::macos_cli::MacOsCliBackend::discover() {
            return Some(Box::new(b));
        }
    }

    None
}

pub fn backend_by_id(id: &str) -> RazResult<Box<dyn ChargeBackend>> {
    match id {
        "linux_sysfs" => LinuxSysfsBackend::discover()
            .map(|b| Box::new(b) as Box<dyn ChargeBackend>)
            .ok_or(RazError::NoBackend),
        #[cfg(windows)]
        "windows_asus" => super::windows_asus::WindowsAsusBackend::open()
            .map(|b| Box::new(b) as Box<dyn ChargeBackend>)
            .ok_or(RazError::NoBackend),
        #[cfg(windows)]
        "windows_wmi_asus" => super::windows_wmi::WindowsWmiAsusBackend::open()
            .map(|b| Box::new(b) as Box<dyn ChargeBackend>)
            .ok_or(RazError::NoBackend),
        #[cfg(target_os = "macos")]
        "macos_cli" => super::macos_cli::MacOsCliBackend::discover()
            .map(|b| Box::new(b) as Box<dyn ChargeBackend>)
            .ok_or(RazError::NoBackend),
        _ => Err(RazError::Backend {
            backend: id.to_string(),
            message: "unknown or unavailable on this platform".into(),
        }),
    }
}

pub fn apply_thresholds(t: Thresholds, backend_id: Option<&str>) -> RazResult<()> {
    t.validate()?;
    let backend: Box<dyn ChargeBackend> = match backend_id {
        Some(id) => backend_by_id(id)?,
        None => best_backend()?,
    };
    backend.set_thresholds(t)?;
    println!(
        "Applied via {}: charge when below {}%, stop above {}%",
        backend.name(),
        t.start,
        t.end
    );
    Ok(())
}
