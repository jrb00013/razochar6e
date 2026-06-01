//! ASUS / ROG charge limit via ATKACPI DeviceIoControl (same path as Electrolite / MyASUS).
//!
//! Many ASUS laptops only honor discrete limits (e.g. 80 vs 100) on Windows; start-at-20%
//! is not available through this interface — use Linux sysfs when dual-booting.

use crate::backend::{ChargeBackend, Thresholds};
use crate::error::{RazError, RazResult};
use std::mem;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::IO::DeviceIoControl;

const DEVICE_PATH: &str = "\\\\.\\ATKACPI";
const IOCTL_ASUS_BATTERY: u32 = 0x0022_240C;
const ASUS_DEVICE_ID: u32 = 0x0012_0057;

/// Input layout used by ASUS ATKACPI DEVS for battery charge limit.
#[repr(C)]
struct AsusChargeInput {
    device_id: u32,
    value: u32,
}

pub struct WindowsAsusBackend {
    handle: HANDLE,
    last_end: Option<u8>,
}

impl WindowsAsusBackend {
    pub fn open() -> Option<Self> {
        let path: Vec<u16> = DEVICE_PATH
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let handle = unsafe {
            CreateFileW(
                PCWSTR(path.as_ptr()),
                0xC000_0000, // GENERIC_READ | GENERIC_WRITE
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        };
        let handle = handle.ok()?;
        if handle.is_invalid() {
            return None;
        }
        Some(Self {
            handle,
            last_end: None,
        })
    }

    fn ioctl_set_limit(&self, percent: u8) -> RazResult<()> {
        let input = AsusChargeInput {
            device_id: ASUS_DEVICE_ID,
            value: u32::from(percent),
        };
        let mut bytes_returned = 0u32;
        let ok = unsafe {
            DeviceIoControl(
                self.handle,
                IOCTL_ASUS_BATTERY,
                Some(&input as *const _ as *const _),
                mem::size_of::<AsusChargeInput>() as u32,
                None,
                0,
                Some(&mut bytes_returned),
                None,
            )
        };
        if ok.is_err() {
            return Err(RazError::Backend {
                backend: "windows_asus".into(),
                message: format!(
                    "DeviceIoControl failed for limit {percent}%: {:?}",
                    ok.err()
                ),
            });
        }
        Ok(())
    }

    pub fn probe_detail() -> (bool, String) {
        match Self::open() {
            Some(_) => (
                true,
                format!("{DEVICE_PATH} IOCTL 0x{IOCTL_ASUS_BATTERY:08X} (end threshold; discrete % on many models)"),
            ),
            None => (false, format!("cannot open {DEVICE_PATH} (Admin + ASUS driver required)")),
        }
    }
}

impl Drop for WindowsAsusBackend {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }
}

impl ChargeBackend for WindowsAsusBackend {
    fn id(&self) -> &'static str {
        "windows_asus"
    }

    fn name(&self) -> &'static str {
        "Windows ASUS ATKACPI"
    }

    fn set_thresholds(&self, t: Thresholds) -> RazResult<()> {
        t.validate()?;
        if t.start > 0 {
            eprintln!(
                "note: windows_asus cannot set start threshold {}; only end {}% is sent to EC",
                t.start, t.end
            );
        }
        self.ioctl_set_limit(t.end)?;
        self.last_end = Some(t.end);
        Ok(())
    }

    fn get_thresholds(&self) -> RazResult<Option<Thresholds>> {
        Ok(self.last_end.map(|end| Thresholds { start: 0, end }))
    }
}
