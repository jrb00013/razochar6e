#!/usr/bin/env bash
# razochar6e — one-shot setup: detect platform, install deps, build, configure.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
START="${START:-20}"
END="${END:-80}"
RELEASE=1
APPLY=1
PERSIST=1
SKIP_HOST=0
SKIP_BUILD=0
FROM_SOURCE=0

log() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m!!>\033[0m %s\n' "$*" >&2; }
die() { printf '\033[1;31mERR>\033[0m %s\n' "$*" >&2; exit 1; }

usage() {
  cat <<'EOF'
Usage: ./setup.sh [options]

Auto-detects Linux, WSL, macOS, or Windows (Git Bash) and:
  - installs build dependencies + Rust (via rustup if needed)
  - installs razochar6e (prebuilt download by default — fast)
  - initializes config and runs doctor
  - applies 20–80% thresholds when supported (may need sudo / Admin)

Options:
  --start N         Lower charge threshold (default: 20)
  --end N           Upper charge threshold (default: 80)
  --prefix DIR      Install binary to DIR/bin (default: ~/.local)
  --from-source     Compile with cargo (slow: 5–20 min first time on Windows)
  --debug           Debug build (only with --from-source)
  --no-apply        Skip applying charge limits after install
  --no-persist      Skip install-persist / Windows scheduled task
  --skip-host       WSL only: do not install Windows host helper
  --skip-build      Skip install (binary must already exist)
  -h, --help        Show this help

Why did it look frozen?
  First `cargo build` on Windows/WSL pauses a long time on proc-macros
  (serde_derive, clap_derive around step 49/61). That is normal.
  Default mode downloads a release binary instead (~30 seconds).

Examples:
  ./setup.sh                    # fast: download prebuilt
  ./setup.sh --from-source      # compile locally
  curl -fsSL https://raw.githubusercontent.com/jrb00013/razochar6e/main/setup.sh | bash
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --start) START="$2"; shift 2 ;;
    --end) END="$2"; shift 2 ;;
    --prefix) PREFIX="$2"; shift 2 ;;
    --from-source) FROM_SOURCE=1; shift ;;
    --debug) RELEASE=0; FROM_SOURCE=1; shift ;;
    --no-apply) APPLY=0; shift ;;
    --no-persist) PERSIST=0; shift ;;
    --skip-host) SKIP_HOST=1; shift ;;
    --skip-build) SKIP_BUILD=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "Unknown option: $1 (try --help)" ;;
  esac
done

detect_platform() {
  if [[ "${RAZOCHAR6E_PLATFORM:-}" != "" ]]; then
    echo "$RAZOCHAR6E_PLATFORM"
    return
  fi
  local uname_s
  uname_s="$(uname -s 2>/dev/null || echo unknown)"

  case "$uname_s" in
    Darwin) echo "macos"; return ;;
    MINGW*|MSYS*|CYGWIN*) echo "windows"; return ;;
    Linux)
      if [[ -f /proc/version ]] && grep -qi microsoft /proc/version; then
        echo "wsl"
      else
        echo "linux"
      fi
      return
      ;;
  esac

  if [[ -n "${WSL_DISTRO_NAME:-}" ]]; then
    echo "wsl"
    return
  fi

  if [[ "${OS:-}" == "Windows_NT" ]]; then
    echo "windows"
    return
  fi

  echo "unknown"
}

have_cmd() { command -v "$1" >/dev/null 2>&1; }

ensure_cargo_path() {
  if have_cmd cargo; then
    return
  fi
  # shellcheck source=/dev/null
  [[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"
}

install_rust() {
  if [[ "$FROM_SOURCE" -eq 0 ]]; then
    return
  fi
  ensure_cargo_path
  if have_cmd cargo; then
    log "Rust already installed: $(rustc --version)"
    return
  fi

  log "Installing Rust via rustup..."
  if ! have_cmd curl; then
    die "curl is required to install rustup"
  fi
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
  # shellcheck source=/dev/null
  source "$HOME/.cargo/env"

  if ! have_cmd cargo; then
    die "Rust install failed — open a new shell and re-run ./setup.sh"
  fi
  log "Rust installed: $(rustc --version)"
}

install_linux_deps() {
  if [[ "$FROM_SOURCE" -eq 0 ]]; then
    if have_cmd curl; then
      return
    fi
  fi
  if have_cmd gcc && have_cmd make && have_cmd curl; then
    log "Build tools already present (gcc, make, curl)"
    return
  fi
  log "Installing Linux build dependencies..."
  if have_cmd apt-get; then
    sudo apt-get update -qq
    sudo apt-get install -y --no-install-recommends \
      build-essential curl pkg-config ca-certificates
  elif have_cmd dnf; then
    sudo dnf install -y gcc make curl pkg-config
  elif have_cmd pacman; then
    sudo pacman -Sy --needed --noconfirm base-devel curl pkgconf
  elif have_cmd zypper; then
    sudo zypper install -y gcc make curl pkg-config
  elif have_cmd apk; then
    sudo apk add build-base curl pkgconfig
  else
    warn "Unknown package manager — ensure gcc, make, and curl are installed"
  fi
}

install_macos_deps() {
  if [[ "$FROM_SOURCE" -eq 0 ]]; then
    return
  fi
  log "Checking macOS build tools..."
  if ! xcode-select -p >/dev/null 2>&1; then
    warn "Xcode Command Line Tools not found — run: xcode-select --install"
  fi
}

install_prebuilt_unix() {
  chmod +x "$REPO_ROOT/scripts/install-prebuilt.sh"
  "$REPO_ROOT/scripts/install-prebuilt.sh" "${1:-}"
  export PATH="$PREFIX/bin:$PATH"
}

build_and_install_unix() {
  if [[ "$SKIP_BUILD" -eq 1 ]]; then
    export PATH="$PREFIX/bin:$PATH"
    return
  fi
  if [[ "$FROM_SOURCE" -eq 0 ]]; then
    log "Installing prebuilt binary (fast — no compile)..."
    install_prebuilt_unix
    return
  fi
  log "Building from source (this takes a while on first run)..."
  local args=()
  [[ "$RELEASE" -eq 1 ]] && args+=(--release)
  "$REPO_ROOT/scripts/install.sh" "${args[@]}" --prefix "$PREFIX"
  export PATH="$PREFIX/bin:$PATH"
}

install_windows_host_prebuilt() {
  local ps_exe="powershell.exe"
  have_cmd powershell.exe || ps_exe="pwsh.exe"
  if ! have_cmd "$ps_exe"; then
    return 1
  fi

  log "Installing Windows host from prebuilt release (no cargo on Windows)..."
  local tag ver url tmpdir win_tmp
  tag="$(curl -fsSL https://api.github.com/repos/jrb00013/razochar6e/releases/latest \
    | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -1)"
  ver="${tag#v}"
  url="https://github.com/jrb00013/razochar6e/releases/download/${tag}/razochar6e-${ver}-windows-x86_64.tar.gz"
  tmpdir="$(mktemp -d)"
  curl -fL --progress-bar "$url" -o "$tmpdir/win.tar.gz"
  tar -xzf "$tmpdir/win.tar.gz" -C "$tmpdir"

  if have_cmd wslpath; then
    win_tmp="$(wslpath -w "$tmpdir")"
  else
    return 1
  fi

  "$ps_exe" -NoProfile -Command "
    \$dest = \"\$env:LOCALAPPDATA\\Programs\\razochar6e\"
    New-Item -ItemType Directory -Force -Path \$dest | Out-Null
    Copy-Item '${win_tmp}\\razochar6e.exe' (Join-Path \$dest 'razochar6e.exe') -Force
    New-Item -ItemType Directory -Force -Path (Join-Path \$dest 'scripts') | Out-Null
    Copy-Item '${win_tmp}\\scripts\\*' (Join-Path \$dest 'scripts') -Force -ErrorAction SilentlyContinue
    Write-Host \"Installed host binary to \$dest\"
  "

  if [[ "$PERSIST" -eq 1 ]]; then
    "$ps_exe" -NoProfile -Command "
      Start-Process -FilePath '$ps_exe' -Verb RunAs -Wait -ArgumentList @(
        '-NoProfile','-ExecutionPolicy','Bypass','-Command',
        \"& \\\"\$env:LOCALAPPDATA\\Programs\\razochar6e\\razochar6e.exe\\\" install-persist --start $START --end $END\"
      )
    " || warn "Elevated persist skipped — run install-persist in Admin PowerShell"
  fi
  rm -rf "$tmpdir"
}

install_windows_host_via_powershell() {
  local win_root ps1
  if ! have_cmd powershell.exe && ! have_cmd pwsh.exe; then
    warn "powershell.exe not found — install Windows host manually (Admin)"
    return 1
  fi

  if [[ "$FROM_SOURCE" -eq 0 ]]; then
    install_windows_host_prebuilt && return 0
  fi

  local ps_exe="powershell.exe"
  have_cmd powershell.exe || ps_exe="pwsh.exe"

  if have_cmd wslpath; then
    win_root="$(wslpath -w "$REPO_ROOT")"
    ps1="${win_root}\\scripts\\install-windows.ps1"
  else
    return 1
  fi

  log "Windows host: compiling on Windows (slow — 5–20 min first time)..."
  log "Tip: cancel and re-run without --from-source to download prebuilt instead"
  "$ps_exe" -NoProfile -Command \
    "Start-Process -FilePath '$ps_exe' -Verb RunAs -Wait -ArgumentList @(
      '-NoProfile','-ExecutionPolicy','Bypass',
      '-File','\"$ps1\"',
      '-Start',$START,
      '-End',$END
    )" || {
      warn "Elevated Windows install failed — try: install-windows-host-prebuilt or ./setup.sh without --from-source"
      return 1
    }
}

run_windows_setup() {
  local ps_exe="powershell.exe"
  have_cmd powershell.exe || ps_exe="pwsh.exe"
  if ! have_cmd "$ps_exe"; then
    die "PowerShell required on Windows"
  fi

  if [[ "$FROM_SOURCE" -eq 0 ]]; then
    log "Fast install: downloading prebuilt Windows binary..."
    install_windows_host_prebuilt || die "Prebuilt Windows install failed"
  else
    install_rust
    local ps1="$REPO_ROOT/scripts/install-windows.ps1"
    log "Compiling on Windows (5–20 min first time — not frozen at step 49/61)..."
    local ps_args=(-Start "$START" -End "$END")
    [[ "$PERSIST" -eq 0 ]] && ps_args+=(-NoPersist)
    [[ "$SKIP_BUILD" -eq 1 ]] && ps_args+=(-NoBuild)
    "$ps_exe" -NoProfile -ExecutionPolicy Bypass -File "$ps1" "${ps_args[@]}"
  fi

  if [[ "$APPLY" -eq 1 ]]; then
    local win_dest="${LOCALAPPDATA:-$HOME/AppData/Local}/Programs/razochar6e/razochar6e.exe"
    [[ -f "$win_dest" ]] && "$win_dest" config init 2>/dev/null || true
  fi

  export PATH="${LOCALAPPDATA:-}/Programs/razochar6e:$PATH"
}

path_has_sysfs_thresholds() {
  local f
  for f in /sys/class/power_supply/BAT*/charge_control_end_threshold; do
    [[ -e "$f" ]] && return 0
  done
  return 1
}

apply_limits_unix() {
  local razo="$PREFIX/bin/razochar6e"
  have_cmd razochar6e && razo="razochar6e"

  if ! have_cmd "$razo"; then
    warn "razochar6e not on PATH"
    return
  fi

  "$razo" config init 2>/dev/null || "$razo" config set --start "$START" --end "$END"

  case "$PLATFORM" in
    wsl)
      if [[ "$APPLY" -eq 1 ]]; then
        log "Applying limits via WSL → Windows host..."
        "$razo" wsl set --start "$START" --end "$END" || warn "wsl set failed — finish Windows host install first"
      fi
      ;;
    linux)
      if path_has_sysfs_thresholds && [[ "$APPLY" -eq 1 ]]; then
        log "Applying sysfs thresholds (sudo)..."
        sudo "$razo" set --start "$START" --end "$END" --save
        [[ "$PERSIST" -eq 1 ]] && sudo "$razo" install-persist --start "$START" --end "$END" || true
      fi
      ;;
    macos)
      if [[ "$APPLY" -eq 1 ]]; then
        sudo "$razo" set --start "$START" --end "$END" --save 2>/dev/null \
          || "$razo" set --start "$START" --end "$END" --save || true
      fi
      ;;
  esac
}

post_install() {
  local razo="$PREFIX/bin/razochar6e"
  have_cmd razochar6e && razo="razochar6e"
  log "Running doctor..."
  "$razo" doctor || true
  if [[ ":$PATH:" != *":$PREFIX/bin:"* ]]; then
    warn "Add to shell rc: export PATH=\"$PREFIX/bin:\$PATH\""
  fi
}

main() {
  PLATFORM="$(detect_platform)"
  log "Platform: $PLATFORM"
  log "Repo: $REPO_ROOT"
  if [[ "$FROM_SOURCE" -eq 0 ]]; then
    log "Mode: prebuilt download (use --from-source to compile)"
  else
    warn "Mode: compile from source — first build is slow; do not Ctrl+C at step 49/61"
  fi

  case "$PLATFORM" in
    linux)
      install_linux_deps
      install_rust
      build_and_install_unix
      apply_limits_unix
      post_install
      ;;
    wsl)
      install_linux_deps
      install_rust
      build_and_install_unix
      if [[ "$SKIP_HOST" -eq 0 ]]; then
        install_windows_host_via_powershell || true
      fi
      apply_limits_unix
      post_install
      ;;
    macos)
      install_macos_deps
      install_rust
      build_and_install_unix
      apply_limits_unix
      post_install
      ;;
    windows)
      run_windows_setup
      ;;
    *)
      die "Unsupported platform"
      ;;
  esac

  log "Setup complete."
}

main "$@"
