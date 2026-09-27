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

## `cycle` stopped and never restored AC

Older builds **exited the loop** if a single Windows WMI battery read failed, so the plug
stayed off while the pack kept draining. Current builds log
`cycle: transient error (will retry): …` and keep polling. Upgrade / rebuild, then:

```bash
razochar6e cycle on --host <ip>   # restore AC immediately if needed
razochar6e cycle --host <ip>      # restart the loop

## `cycle` stuck at 79% with outlet still ON

Windows/ASUS often report **78–79%** forever when the firmware charge limit is 80%, so a
strict `>= 80` cut never fires. Current builds cut when `pct >= end - 2` (e.g. **78%** for
`--end 80`).
```

## Kasa auth fails even with the correct password (EP10 / KLAP lv2)

Stock `python-kasa` `Discover` maps `IOT.SMARTPLUGSWITCH` + KLAP to **KlapTransport v1**
hashes. EP10 firmware with `lv: 2` / `new_klap` needs **KlapTransportV2** hashes while
still using the IoT protocol. `scripts/kasa_plug.py` forces that combo.

Debug signature when the wrong transport is used:
`Device response did not match our challenge` on handshake1, even though
`owner` = MD5(email) matches your account.
