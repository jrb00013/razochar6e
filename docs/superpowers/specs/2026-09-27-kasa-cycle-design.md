# Design: `razochar6e cycle` + Kasa smart plug

**Goal:** Autonomously charge/drain the laptop by cutting and restoring AC at the wall via a TP-Link Kasa plug, using the same start/end band as firmware thresholds.

## Behavior

- Poll battery % on an interval (default 60s).
- If charge ≥ `end` (default 80%) and the outlet is on → turn outlet **off** (run on battery).
- If charge ≤ `start` (default 20%) and the outlet is off → turn outlet **on** (charge again).
- Otherwise hold. Never cut AC if battery % cannot be read.

## Plug control

- Prefer local control (no cloud round-trip for the on/off command itself).
- **Modern Kasa (KLAP, e.g. EP10):** drive via `python-kasa` using TP-Link account credentials from `KASA_USERNAME` / `KASA_PASSWORD` (or CLI flags). Host defaults to config `kasa_host` or discovery.
- **Legacy Kasa (TCP 9999 XOR):** native Rust client, no credentials.

## Battery source

- Linux: real sysfs batteries (skip Hyper-V/virtual).
- WSL / Windows: `Win32_Battery.EstimatedChargeRemaining` via PowerShell.

## CLI

```
razochar6e cycle [--host IP] [--start N] [--end M] [--interval SECS] [--once] [--discover]
razochar6e cycle on|off|state   # manual plug control for setup
```

## Out of scope

- Home Assistant, cloud-only plugs, EC/firmware hacks.
