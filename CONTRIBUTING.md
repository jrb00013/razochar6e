# Contributing to razochar6e

Thanks for helping improve battery charge scheduling for everyone.

## Development

```bash
git clone https://github.com/jrb00013/razochar6e.git
cd razochar6e
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all
```

## Adding a backend

1. Implement `ChargeBackend` in `src/backend/your_backend.rs`
2. Register in `src/backend/registry.rs` (`probe_all`, `select_best`, `backend_by_id`)
3. Document sysfs/IOCTL/WMI paths in `docs/VENDORS.md`
4. Add probe tests where possible (mock or feature-gated)

## Pull requests

- One logical change per PR
- Run `make check` before opening
- Note your laptop model + OS in the PR if testing hardware

## Reporting hardware

When filing an issue, include:

```bash
razochar6e probe --json
```

On WSL, also run `razochar6e wsl probe` after installing the Windows binary.
