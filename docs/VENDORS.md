# Vendor support matrix

`razochar6e` only works where the OEM exposes **charge start/stop** control to the OS. There is no generic “block the charger” API on unsupported hardware.

## Linux (`linux_sysfs`)

Writes to:

- `/sys/class/power_supply/BAT*/charge_control_start_threshold`
- `/sys/class/power_supply/BAT*/charge_control_end_threshold`

Legacy ThinkPad names (`charge_start_threshold` / `charge_stop_threshold`) are detected automatically.

| Vendor | Typical support | Notes |
|--------|-----------------|-------|
| Lenovo ThinkPad | Excellent | Documented in kernel `thinkpad-acpi` |
| Dell | Good (newer) | Model-dependent |
| ASUS / ROG | Good | Some models only accept 40/60/80/100 |
| Framework | Varies | May need `framework_tool` on some gens |
| HP | Poor | Often missing sysfs; use Windows OEM tools |
| System76 | Good | `system76_acpi` |
| MSI / Toshiba / Huawei / LG / Fujitsu | Varies | See kernel `platform/x86` drivers |

Probe:

```bash
ls /sys/class/power_supply/BAT*/charge_control_end_threshold
```

## Windows

| Vendor | Backend | Custom 20–80? |
|--------|---------|----------------|
| ASUS / ROG | `windows_asus` (PowerShell IOCTL + WMI) | End often yes; start usually firmware-only |
| Lenovo | Not yet | Use Lenovo Vantage / conservation mode |
| Dell | Not yet | Dell Power Manager / BIOS |
| HP | Not yet | HP Command Center / BIOS |

ASUS IOCTL: `\\.\ATKACPI`, code `0x0022240C`, device `0x00120057`.

## macOS (`macos_cli`)

Wraps, in order of preference:

1. `batt limit N`
2. `battery maintain LOW-HIGH`
3. `bclm write N` (often 80/100 only on Apple Silicon)

Disable **Optimized Battery Charging** when using third-party tools.

## WSL

Linux side sees a **virtual** Hyper-V battery. Use `razochar6e wsl` + Windows binary — see [TROUBLESHOOTING.md](TROUBLESHOOTING.md).
