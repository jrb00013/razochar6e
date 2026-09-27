# Troubleshooting

## `setup.sh` looks frozen at `Building ... 49/61`

This is **normal** on the first compile (Windows or `--from-source`). Rust spends a long time on proc-macros (`serde_derive`, `clap_derive`, `thiserror-impl`) with little console movement.

**Fix (recommended):** use the default fast path — do **not** pass `--from-source`:

```bash
./setup.sh
```

That downloads a prebuilt binary from GitHub Releases (~30 seconds).

If you already started a compile, let it finish once (can take 5–20 minutes), or Ctrl+C and re-run `./setup.sh` without `--from-source`.

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

## Kasa `cycle` can't talk to the plug

1. Confirm the laptop **charger** is plugged into the Kasa (not only the Kasa into the wall).
2. `razochar6e cycle --discover` — your EP10/etc. should appear. Note the IP.
3. Modern Kasa (KLAP, HTTP port 80) needs the same account as the Kasa app:

   ```bash
   pip install python-kasa
   export KASA_USERNAME='you@example.com'
   export KASA_PASSWORD='…'
   razochar6e cycle state --host 192.168.x.x
   ```

4. From WSL, the plug must be reachable on the LAN (mirrored networking or host route). If discovery only works in Windows Python, pass `--host` explicitly from WSL.
5. Legacy plugs (TCP 9999) need no credentials — if connect to `:9999` works, the built-in XOR client is used.
