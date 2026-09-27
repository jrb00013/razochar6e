# Kasa Cycle Implementation Plan

> **For agentic workers:** implement task-by-task; checkboxes track progress.

**Goal:** Add `razochar6e cycle` that polls battery % and toggles a Kasa plug at start/end thresholds.

**Architecture:** Pure decision loop in Rust; plug I/O via native XOR (legacy) or `scripts/kasa_plug.py` + python-kasa (KLAP); battery via sysfs or Windows WMI.

**Tech Stack:** Rust 1.74+, clap, serde/toml, python-kasa (optional runtime dep for KLAP).

## Global Constraints

- Do not store TP-Link passwords in committed files; env/CLI only.
- Skip Hyper-V virtual batteries when reading %.
- If battery % unknown, leave the plug alone (prefer on).

---

### Task 1: Battery capacity helper

**Files:** `src/battery.rs`, wire from `main`/`cycle`

- [ ] Implement `battery_percent() -> RazResult<u8>` with Linux + Windows/WSL paths
- [ ] Unit-testable filter: reject virtual batteries

### Task 2: Kasa clients

**Files:** `src/kasa.rs`, `scripts/kasa_plug.py`

- [ ] Native XOR encrypt/decrypt + `set_relay` / `get_sysinfo` / UDP discover
- [ ] Python helper: discover / on / off / state with KLAP creds
- [ ] Rust wrapper tries XOR first, falls back to python helper

### Task 3: Cycle loop + CLI

**Files:** `src/cycle.rs`, `src/cli.rs`, `src/main.rs`, `src/config.rs`

- [ ] `decide(pct, start, end, on) -> Action` with unit tests
- [ ] `cycle` / `cycle on|off|state` / `--discover`
- [ ] Config fields: `kasa_host`, optional interval defaults

### Task 4: Docs + verify

**Files:** `README.md`, `docs/TROUBLESHOOTING.md`, `CHANGELOG.md`

- [ ] Document setup (host, `KASA_USERNAME`/`KASA_PASSWORD`, `pip install python-kasa`)
- [ ] `cargo test` + `cargo build`
