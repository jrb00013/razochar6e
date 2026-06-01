# Quick start: ASUS ROG + WSL2

Your machine runs **Windows** for the real battery and **WSL** for development. `razochar6e` cannot control charging from Linux sysfs inside WSL (Hyper-V virtual battery only).

## Automated (recommended)

From WSL in the repo:

```bash
./setup.sh
```

This installs Rust/deps, builds `razochar6e`, elevates the Windows host install (UAC), and runs `razochar6e wsl set --start 20 --end 80`.

## 1. Windows only (Administrator)

```powershell
git clone https://github.com/jrb00013/razochar6e.git
cd razochar6e
.\setup.sh
# or: .\scripts\install-windows.ps1 -Start 20 -End 80
razochar6e probe
```

If `windows_asus` is available, limits are applied at the EC. Many ROG models only support **80% vs 100%** on Windows (not a true 20% start).

## 2. WSL (daily use)

After `./setup.sh`:

```bash
razochar6e wsl status
razochar6e wsl set --start 20 --end 80
razochar6e doctor
```

Approve the **UAC** prompt when the host script elevates.

## 3. Optional: full 20–80 band

Dual-boot **native Linux** on the same laptop and use:

```bash
sudo razochar6e set --start 20 --end 80
sudo razochar6e install-persist
```

when `charge_control_start_threshold` exists under `/sys/class/power_supply/BAT*`.

## Verify

- Windows: `razochar6e status` (Admin)
- WSL: `razochar6e wsl status`
- Battery stays near 80% on AC after a full discharge cycle once
