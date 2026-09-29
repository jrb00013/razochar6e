# razochar6e

[![CI](https://github.com/jrb00013/razochar6e/actions/workflows/ci.yml/badge.svg)](https://github.com/jrb00013/razochar6e/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.74%2B-orange.svg)](https://www.rust-lang.org/)

**Firmware-backed battery charge scheduling** for laptops: stop charging above an upper limit (default **80%**), resume below a lower limit (default **20%**). One binary for Linux, Windows, macOS, and WSL→Windows host control.

Built for ASUS ROG, ThinkPad, Dell, Framework, HP (where supported), and MacBooks (via SMC tools).

## Why

Keeping a lithium pack pegged at 100% on AC ages it fast. Many machines expose **charge start/stop thresholds** in firmware — `razochar6e` sets them once (or at boot) instead of babysitting a notification app.

## Quick start

**One command** — detects Linux / WSL / macOS / Windows, installs deps, builds, configures:

```bash
git clone https://github.com/jrb00013/razochar6e.git
cd razochar6e
./setup.sh          # Linux, WSL, macOS, Git Bash
```

**Windows (PowerShell):**

```powershell
.\setup.ps1
.\setup.ps1 -Start 25 -End 85
```

Or without cloning:

```bash
curl -fsSL https://raw.githubusercontent.com/jrb00013/razochar6e/main/setup.sh | bash
```

```powershell
irm https://raw.githubusercontent.com/jrb00013/razochar6e/main/setup.ps1 | iex
```

Options: `./setup.sh --help` · `.\setup.ps1 -Help` · `--start 25 --end 85` · WSL: `--skip-host`

| Platform | What `setup.sh` does |
|----------|----------------------|
| **Linux** | apt/dnf/pacman deps, rustup, build, `sudo set` + persist if sysfs exists |
| **WSL** | Same + elevated `install-windows.ps1` on host, then `wsl set` |
| **macOS** | Xcode CLI check, rustup, build; optional `batt`/`battery`/`bclm` |
| **Windows** | `setup.ps1` → prebuilt, config, elevated `set`, scheduled task |

Manual: [docs/QUICKSTART-WINDOWS.md](docs/QUICKSTART-WINDOWS.md) · [docs/QUICKSTART-ROG-WSL.md](docs/QUICKSTART-ROG-WSL.md) · `scripts/install.sh` · `scripts/install-windows.ps1`

## Commands

| Command | Description |
|---------|-------------|
| `probe` | Batteries + backends (`--json` for scripts) |
| `doctor` | Health check (exit 1 if unsupported) |
| `set --start N --end M` | Apply thresholds (`--save` writes config) |
| `apply` | Apply from `~/.config/razochar6e/config.toml` |
| `status` | Capacity, AC, active thresholds |
| `clear` | Reset to 0–100% |
| `config init` / `show` / `set` | Manage config file |
| `install-persist` | systemd / Task Scheduler / launchd |
| `completions bash\|fish\|zsh\|…` | Shell completions |
| `wsl probe\|status\|set` | Control Windows host from WSL |
| `cycle` | Smart-plug charge/drain loop (Kasa) |
| `cycle on\|off\|state` | Manual Kasa outlet control |
| `cycle --discover` | List Kasa plugs on the LAN |
| `sleepcut` | Cut Kasa when *you* sleep Windows; restore on wake |
| `benchmark` | Logistic model: recommend start/end for $/power vs stranding |

```bash
razochar6e --help
```

### Kasa charge/drain cycle

Firmware thresholds stop *charging* above your upper limit, but the pack won't *drain* while AC is present. With a Kasa plug on the charger:

```bash
pip install python-kasa          # needed for modern KLAP plugs (EP10, etc.)
export KASA_USERNAME='you@example.com'   # same as Kasa app
export KASA_PASSWORD='…'

razochar6e cycle --discover
razochar6e cycle --host 192.168.1.88 --save   # remember host in config
razochar6e cycle --once                      # one poll+action
razochar6e cycle                             # continuous loop (Ctrl+C to stop)
```

At ≥ `end`% the outlet turns **off**; at ≤ `start`% it turns **on**. Legacy TCP-9999 plugs need no credentials; KLAP plugs need the env vars above.

### Sleepcut (cut AC when *you* sleep)

Does **not** auto-sleep the laptop. You sleep Windows as usual; a watcher cuts the Kasa outlet on suspend and turns it back on at resume:

```bash
export KASA_USERNAME='you@example.com'
export KASA_PASSWORD='…'
razochar6e sleepcut --host 192.168.1.88 --save
# Ctrl+C to stop
```

Overnight sleep on battery naturally drops a few % of charge (e.g. high-70s → high-60s). That is the useful shallow rest — **better than a forced awake 79→69 cycle**, which burns ~0.1 full-cycle of pack life per pass for almost no bill savings (recharge heat waste) and only a tiny calendar-aging win vs just sitting under an 80% firmware limit.

### Benchmark (optimal band)

Fits a small **logistic regression** over synthetic usage samples and scores candidate `(start, end)` bands for P(acceptable day) vs projected $/yr:

```bash
razochar6e benchmark
razochar6e benchmark --rate 0.18 --capacity-wh 90 --daily-wh 40 --hours-away 3
razochar6e benchmark --apply    # write recommended start/end to config
```

## Config

`~/.config/razochar6e/config.toml` (or platform equivalent):

```toml
start = 20
end = 80
# backend = "linux_sysfs"   # optional force
# kasa_host = "192.168.1.88"
```
```bash
razochar6e config init
razochar6e apply
```

## Backends

| ID | Platform | Start @ 20%? | End @ 80%? |
|----|----------|--------------|------------|
| `linux_sysfs` | Linux | Usually yes | Yes |
| `windows_asus` | Windows ASUS/ROG | Often no | Yes (sometimes 80/100 only) |
| `macos_cli` | macOS (`batt` / `battery` / `bclm`) | Varies | Varies |
| `wsl_bridge` | WSL | Via host | Via host |

See [docs/VENDORS.md](docs/VENDORS.md) for per-OEM notes.

## Install options

| Method | Command |
|--------|---------|
| Cargo | `cargo install --git https://github.com/jrb00013/razochar6e` |
| Script (Linux) | `scripts/install.sh` |
| Make | `make install` |
| Windows | `scripts/install-windows.ps1` |

## Permissions

- **Linux**: `sudo` or [udev rule](deploy/99-razochar6e-charge.rules) for sysfs writes
- **Windows**: Administrator for ATKACPI / WMI
- **macOS**: `sudo` for SMC tools

## Docs

- [Windows quick start](docs/QUICKSTART-WINDOWS.md)
- [ASUS ROG + WSL quick start](docs/QUICKSTART-ROG-WSL.md)
- [Vendor matrix](docs/VENDORS.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)
- [Contributing](CONTRIBUTING.md)
- [Changelog](CHANGELOG.md)

## Related projects

[TLP](https://github.com/linrunner/tlp), [asusctl](https://gitlab.com/asus-linux/asusctl), [Electrolite](https://github.com/alikarbasicom/electrolite), [batt](https://github.com/charlie0129/batt)

## License

MIT — see [LICENSE](LICENSE).
