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

### ASUS field notes (verified on a ROG Strix G18, model `G815LW`)

Real-hardware findings from debugging `windows_asus`, recorded so they
aren't re-discovered the hard way:

- The IOCTL path (`\\.\ATKACPI`) failed on this model with
  `ERROR_INSUFFICIENT_BUFFER` ("the data area passed to a system call is
  too small") — the in-buffer layout `run_script`/the driver expects
  differs from the 8-byte `(DevID, percent)` struct the script currently
  builds. Not pursued further (would require reverse-engineering the
  exact `ATKACPI` IOCTL struct layout, which is undocumented).
- **WMI call style matters a lot.** `DEVS`/`DSTS` on `AsusAtkWmi_WMNB`
  rejected every attempt built via explicit named-parameter binding —
  `Invoke-CimMethod -Arguments @{...}`, `[wmiclass]::InvokeMethod` with
  `GetMethodParameters()`, and raw COM `IWbemServices::ExecMethod` with
  `IWbemClassObject::Put()` by name — all three failed identically with
  `WBEM_E_INVALID_PARAMETER` (`0x8004102F`), even for the simplest
  possible call. But the classic dynamic-dispatch style —
  `(Get-WmiObject -Namespace root/WMI -Class AsusAtkWmi_WMNB).DEVS(devid, percent)`
  (a live instance, method called positionally, not by named property
  assignment) — **succeeded with no error**, and this is exactly what the
  existing `Set-LimitWmi` in `scripts/asus-battery-limit.ps1` already
  does. Likely explanation: the method's `Device_ID`/`Control_status`
  parameters carry `[ID]` qualifiers that encode positional slots in the
  packed argument layout, and PowerShell's WMI object adapter binds by
  that position, while the explicit-parameter-object approaches don't
  necessarily preserve the same ordering.
  **Caveat:** a successful WMI call and a successful *read-back* via
  `DSTS(0x00120057)` were both confirmed, but the returned status word
  (`0x890000` observed) is a bit-packed value whose exact layout wasn't
  decoded — treat this as a promising lead confirmed to at least not
  error, not as proven confirmation that it changes real EC charging
  behavior at the requested percentage. Worth a follow-up session with
  proper bit-layout reference material before relying on it as primary.
- **The `ASUSOptimization` INI file is not reliable input config, and
  writing to it races the service.** `ASUSOptimization` periodically
  flushes its own internal state back into
  `C:\ProgramData\ASUS\ASUS System Control Interface\AsusOptimization\Customization.ini`.
  Writing the file and then restarting the service (write → stop → start)
  can silently lose the write within ~3 seconds if the restart lands
  mid-flush. **Stop the service first, then write, then start** — verified
  stable through a 10s settle with this ordering, reverted within 3s with
  the other. `windows_asus.rs`'s INI fallback uses the corrected order.
  Also note this file is best treated as a *display cache* MyASUS/Armoury
  Crate read for their own UI (confirmed: MyASUS's "Battery Care Mode"
  notification reflected a value set this way) rather than confirmed
  proof of independent EC-level enforcement — every verification path
  available here routes back through ASUS's own software stack, so
  nothing checked so far is fully independent ground truth.

## macOS (`macos_cli`)

Wraps, in order of preference:

1. `batt limit N`
2. `battery maintain LOW-HIGH`
3. `bclm write N` (often 80/100 only on Apple Silicon)

Disable **Optimized Battery Charging** when using third-party tools.

## WSL

Linux side sees a **virtual** Hyper-V battery. Use `razochar6e wsl` + Windows binary — see [TROUBLESHOOTING.md](TROUBLESHOOTING.md).
