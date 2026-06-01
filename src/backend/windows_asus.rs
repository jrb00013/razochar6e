//! ASUS / ROG charge limit via ATKACPI DeviceIoControl (same path as Electrolite / MyASUS).

use crate::backend::{ChargeBackend, Thresholds};
use crate::error::{RazError, RazResult};
use std::mem;
use windows_sys::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::IO::DeviceIoControl;

const DEVICE_PATH: &str = "\\\\.\\ATKACPI";
const IOCTL_ASUS_BATTERY: u32 = 0x0022_240C;
const ASUS_DEVICE_ID: u32 = 0x0012_0057;

#[repr(C)]
struct AsusChargeInput {
    device_id: u32,
    value: u32,
}

pub struct WindowsAsusBackend;

impl WindowsAsusBackend {
    pub fn open() -> Option<Self> {
        if open_device().is_ok() {
            Some(Self)
        } else {
            None
        }
    }

    pub fn probe_detail() -> (bool, String) {
        match open_device() {
            Ok(handle) => {
                unsafe { CloseHandle(handle) };
                (
                    true,
                    format!(
                        "{DEVICE_PATH} IOCTL 0x{IOCTL_ASUS_BATTERY:08X} (end threshold; discrete % on many models)"
                    ),
                )
            }
            Err(_) => (
                false,
                format!("cannot open {DEVICE_PATH} (Admin + ASUS driver required)"),
            ),
        }
    }

    fn set_limit(percent: u8) -> RazResult<()> {
        let handle = open_device().map_err(|msg| RazError::Backend {
            backend: "windows_asus".into(),
            message: msg,
        })?;
        let result = ioctl_set_limit(handle, percent);
        unsafe { CloseHandle(handle) };
        result
    }
}

fn open_device() -> Result<HANDLE, String> {
    let path: Vec<u16> = DEVICE_PATH
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            0,
        )
    };
    if handle == usize::MAX as HANDLE || handle == 0 {
        Err(format!("CreateFileW failed for {DEVICE_PATH}"))
    } else {
        Ok(handle)
    }
}

fn ioctl_set_limit(handle: HANDLE, percent: u8) -> RazResult<()> {
    let input = AsusChargeInput {
        device_id: ASUS_DEVICE_ID,
        value: u32::from(percent),
    };
    let mut bytes_returned = 0u32;
    let ok = unsafe {
        DeviceIoControl(
            handle,
            IOCTL_ASUS_BATTERY,
            &input as *const _ as *const _,
            mem::size_of::<AsusChargeInput>() as u32,
            std::ptr::null_mut(),
            0,
            &mut bytes_returned,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(RazError::Backend {
            backend: "windows_asus".into(),
            message: format!("DeviceIoControl failed for limit {percent}%"),
        });
    }
    Ok(())
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
        Self::set_limit(t.end)
    }

    fn get_thresholds(&self) -> RazResult<Option<Thresholds>> {
        Ok(None)
    }
}
