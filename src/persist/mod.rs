use crate::backend::Thresholds;
use crate::error::RazResult;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

const SERVICE_NAME: &str = "razochar6e-thresholds";
const UNIT_FILENAME: &str = "razochar6e-thresholds.service";

pub fn install(start: u8, end: u8) -> RazResult<()> {
    let t = Thresholds { start, end };
    t.validate()?;

    #[cfg(target_os = "linux")]
    {
        install_systemd(&t)
    }

    #[cfg(windows)]
    {
        install_windows_task(&t)
    }

    #[cfg(target_os = "macos")]
    {
        install_launchd(&t)
    }

    #[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
    {
        Err(crate::error::RazError::Backend {
            backend: "persist".into(),
            message: "persistence not implemented for this OS".into(),
        })
    }
}

pub fn uninstall() -> RazResult<()> {
    #[cfg(target_os = "linux")]
    return uninstall_systemd();

    #[cfg(windows)]
    return uninstall_windows_task();

    #[cfg(target_os = "macos")]
    return uninstall_launchd();

    #[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
    Err(crate::error::RazError::Backend {
        backend: "persist".into(),
        message: "persistence not implemented for this OS".into(),
    })
}

#[cfg(target_os = "linux")]
fn install_systemd(t: &Thresholds) -> RazResult<()> {
    let exe = std::env::current_exe().map_err(crate::error::RazError::Io)?;
    let unit_path = PathBuf::from("/etc/systemd/system").join(UNIT_FILENAME);
    let unit = format!(
        r#"[Unit]
Description=razochar6e battery charge thresholds ({}%-{}%)
After=multi-user.target

[Service]
Type=oneshot
ExecStart={} set --start {} --end {}
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
"#,
        t.start,
        t.end,
        exe.display(),
        t.start,
        t.end
    );

    write_root_file(&unit_path, &unit)?;
    run_systemctl(&["daemon-reload"])?;
    run_systemctl(&["enable", "--now", SERVICE_NAME])?;
    println!("Installed systemd unit: {}", unit_path.display());
    Ok(())
}

#[cfg(target_os = "linux")]
fn uninstall_systemd() -> RazResult<()> {
    let _ = run_systemctl(&["disable", "--now", SERVICE_NAME]);
    let unit_path = PathBuf::from("/etc/systemd/system").join(UNIT_FILENAME);
    if unit_path.exists() {
        std::fs::remove_file(&unit_path).map_err(crate::error::RazError::Io)?;
    }
    run_systemctl(&["daemon-reload"])?;
    println!("Removed systemd unit {UNIT_FILENAME}");
    Ok(())
}

#[cfg(target_os = "linux")]
fn run_systemctl(args: &[&str]) -> RazResult<()> {
    let status = Command::new("systemctl").args(args).status()?;
    if !status.success() {
        return Err(crate::error::RazError::Backend {
            backend: "systemd".into(),
            message: format!("systemctl {} failed", args.join(" ")),
        });
    }
    Ok(())
}

#[cfg(windows)]
fn install_windows_task(t: &Thresholds) -> RazResult<()> {
    let exe = std::env::current_exe().map_err(crate::error::RazError::Io)?;
    let tr = format!(
        r#""{}" set --start {} --end {}"#,
        exe.display(),
        t.start,
        t.end
    );
    let ps = format!(
        r#"
$action = New-ScheduledTaskAction -Execute '{}' -Argument 'set --start {} --end {}'
$trigger = New-ScheduledTaskTrigger -AtLogOn
$principal = New-ScheduledTaskPrincipal -UserId 'SYSTEM' -RunLevel Highest
Register-ScheduledTask -TaskName '{}' -Action $action -Trigger $trigger -Principal $principal -Force
"#,
        exe.display(),
        t.start,
        t.end,
        SERVICE_NAME
    );
    run_powershell(&ps)?;
    println!("Registered scheduled task: {SERVICE_NAME}");
    Ok(())
}

#[cfg(windows)]
fn uninstall_windows_task() -> RazResult<()> {
    run_powershell(&format!(
        "Unregister-ScheduledTask -TaskName '{}' -Confirm:$false -ErrorAction SilentlyContinue",
        SERVICE_NAME
    ))?;
    println!("Removed scheduled task {SERVICE_NAME}");
    Ok(())
}

#[cfg(windows)]
fn run_powershell(script: &str) -> RazResult<()> {
    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .status()?;
    if !status.success() {
        return Err(crate::error::RazError::Backend {
            backend: "windows_task".into(),
            message: "PowerShell scheduled task command failed (run as Administrator)".into(),
        });
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn install_launchd(t: &Thresholds) -> RazResult<()> {
    let exe = std::env::current_exe().map_err(crate::error::RazError::Io)?;
    let plist_path = PathBuf::from("/Library/LaunchDaemons/com.razochar6e.thresholds.plist");
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>com.razochar6e.thresholds</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
    <string>set</string>
    <string>--start</string><string>{}</string>
    <string>--end</string><string>{}</string>
  </array>
  <key>RunAtLoad</key><true/>
</dict></plist>
"#,
        exe.display(),
        t.start,
        t.end
    );
    write_root_file(&plist_path, &plist)?;
    Command::new("launchctl")
        .args(["load", "-w"])
        .arg(&plist_path)
        .status()?;
    println!("Installed launchd plist: {}", plist_path.display());
    Ok(())
}

#[cfg(target_os = "macos")]
fn uninstall_launchd() -> RazResult<()> {
    let label = "com.razochar6e.thresholds";
    let _ = Command::new("launchctl")
        .args(["unload", "-w", label])
        .status();
    let plist_path = PathBuf::from("/Library/LaunchDaemons/com.razochar6e.thresholds.plist");
    if plist_path.exists() {
        fs::remove_file(&plist_path).map_err(crate::error::RazError::Io)?;
    }
    println!("Removed launchd plist");
    Ok(())
}

fn write_root_file(path: &std::path::Path, contents: &str) -> RazResult<()> {
    if path.exists() {
        fs::write(path, contents).map_err(crate::error::RazError::Io)?;
        return Ok(());
    }
    let mut child = Command::new("sudo")
        .arg("tee")
        .arg(path)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(crate::error::RazError::Io)?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(contents.as_bytes())?;
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(crate::error::RazError::Backend {
            backend: "persist".into(),
            message: format!("sudo tee {} failed", path.display()),
        });
    }
    Ok(())
}
