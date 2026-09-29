# Changelog

All notable changes to this project are documented in this file.

## [Unreleased]

### Added

- `razochar6e cycle` — charge/drain loop via TP-Link Kasa smart plug (cut AC at end %, restore at start %)
- `razochar6e sleepcut` — cut Kasa on Windows suspend, restore on resume (no auto-sleep)
- Native legacy Kasa XOR client (TCP 9999) plus `scripts/kasa_plug.py` for KLAP/modern firmware (python-kasa)
- Config `kasa_host`; battery % from real Linux sysfs or Windows WMI (WSL-aware)
- `razochar6e benchmark` — logistic regression over candidate charge bands; projected kWh/$ vs 100% float

### Fixed

- `cycle` no longer exits the loop on transient battery/WMI or plug errors (retries next interval)
- Battery % read retries WMI up to 3 times before reporting failure
- Cut AC at `end - 2` slack (e.g. 78% when `--end 80`) so ASUS/WMI stickiness still drains
- Document / ignore the old `.test-fixed-set-runner.ps1` footgun (`Get-Content $log | Out-File -Append $log` → unbounded log growth)

## [0.2.6] - 2026-06-01

### Added

- `./setup.sh` — auto-detect platform (linux/wsl/macos/windows), install deps + Rust, build, configure
- `./setup.ps1` — native Windows one-shot install (prebuilt, config, elevated set, persist, doctor)
- [docs/QUICKSTART-WINDOWS.md](docs/QUICKSTART-WINDOWS.md)
- `install-windows.ps1` flags: `-NoPersist`, `-NoBuild`

## [0.2.4] - 2026-06-01

### Changed

- Windows ASUS control via `scripts/asus-battery-limit.ps1` (IOCTL + WMI); no native Windows crate dep

## [0.2.3] - 2026-06-01

### Fixed

- Windows ASUS backend uses `windows-sys`; unit struct (no `HANDLE` in trait object)
- `WindowsAsusBackend::probe_detail` in registry

## [0.2.2] - 2026-06-01

### Fixed

- Windows build: `Sync` on WMI backend, `GENERIC_READ` import, `cmd_wsl` cfg layout

## [0.2.1] - 2026-06-01

### Fixed

- macOS build: `MacOsCliBackend::probe_detail` and `Debug` on `MacTool`
- Release workflow packaging and macOS aarch64-only artifacts
- Windows `CreateFileW` access flags via `GENERIC_READ | GENERIC_WRITE`

## [0.2.0] - 2026-06-01

### Added

- `doctor`, `apply`, `config` subcommands
- `~/.config/razochar6e/config.toml` support
- Shell completions (`razochar6e completions`)
- Windows ASUS WMI fallback backend
- CI (fmt, clippy, test, multi-OS build)
- Install scripts, Makefile, vendor/troubleshooting docs
- GitHub issue templates and release workflow

### Changed

- CLI split into `cli` module; expanded README

## [0.1.0] - 2026-06-01

### Added

- Initial release: probe, set, status, clear, persist, WSL bridge
- Backends: `linux_sysfs`, `windows_asus`, `macos_cli`, `wsl_bridge`
