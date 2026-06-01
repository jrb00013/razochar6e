# Changelog

All notable changes to this project are documented in this file.

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
