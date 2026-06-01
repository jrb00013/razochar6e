# Troubleshooting

## `probe` shows no backends

**Linux:** Your kernel driver may not expose charge thresholds. Check BIOS for “battery care” / “charge limit” and try a newer kernel.

**Windows:** Run PowerShell **as Administrator**. Confirm ASUS ATKACPI driver is installed (MyASUS / Armoury Crate once).

**WSL:** Expected for sysfs — use `razochar6e wsl probe` after Windows install.

## Set succeeded but battery stays at 100%

Many systems **do not discharge** when you lower the stop threshold. Use the machine on battery until it drops below the band, then plug in again.

## WSL `wsl set` fails

1. Build on Windows: `cargo build --release` in repo
2. Run `.\scripts\install-windows.ps1` as Admin
3. Approve UAC when the host script elevates
4. Ensure `powershell.exe` works from WSL: `powershell.exe -Command "echo ok"`

## ASUS only accepts 80 or 100 on Windows

Normal for many ROG models. Full **20% start** may require dual-boot Linux with `charge_control_start_threshold`.

## Permission denied on Linux

```bash
sudo razochar6e set --start 20 --end 80
```

Or install [deploy/99-razochar6e-charge.rules](../deploy/99-razochar6e-charge.rules) and add your user to `plugdev`.

## `doctor` exits 1

Informational on unsupported hosts. Read printed `[warn]` / `[fail]` lines and `razochar6e probe --json`.
