# Quick start: Windows (ASUS ROG / native)

Use this when you run **PowerShell on Windows**, not WSL. WSL users should use `./setup.sh` instead.

## One command

Open **PowerShell** in the repo (Run as Administrator recommended for first setup):

```powershell
git clone https://github.com/jrb00013/razochar6e.git
cd razochar6e
Set-ExecutionPolicy -Scope Process Bypass
.\setup.ps1
```

Custom limits:

```powershell
.\setup.ps1 -Start 25 -End 85
```

Approve **UAC** when prompted for `set` and `install-persist`.

## Without cloning

```powershell
Set-ExecutionPolicy -Scope Process Bypass
irm https://raw.githubusercontent.com/jrb00013/razochar6e/main/setup.ps1 | iex
```

## Install location

| Item | Path |
|------|------|
| Binary | `%LOCALAPPDATA%\Programs\razochar6e\razochar6e.exe` |
| Scripts | `%LOCALAPPDATA%\Programs\razochar6e\scripts\` |
| Config | `%APPDATA%\razochar6e\config.toml` (via `directories` crate) |

Add the Programs folder to your user **PATH** if `razochar6e` is not found in a new shell.

## Daily commands

```powershell
razochar6e probe
razochar6e doctor
razochar6e status
razochar6e set --start 20 --end 80 --save
razochar6e apply
razochar6e clear
```

`set` needs **Administrator** on most ASUS/ROG machines (ATKACPI / WMI).

## What to expect on ROG

- **End limit** (e.g. 80% or 85%) is what Windows usually enforces.
- **Start limit** (e.g. 20%) is stored in config but often **ignored** on `windows_asus`.
- `status` may show **no sysfs batteries** and **thresholds not readable** — normal on Windows.

## Options

| Flag | Meaning |
|------|---------|
| `-FromSource` | `cargo build --release` (slow first time) |
| `-NoApply` | Install only, do not call `set` |
| `-NoPersist` | Skip logon scheduled task |
| `-SkipBuild` | Use existing install under Programs |

## WSL on the same machine

After Windows setup, from WSL:

```bash
./setup.sh --skip-host   # Linux binary only, host already installed
razochar6e wsl set --start 25 --end 85
```

See [QUICKSTART-ROG-WSL.md](QUICKSTART-ROG-WSL.md).
