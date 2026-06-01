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

log() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m!!>\033[0m %s\n' "$*" >&2; }
die() { printf '\033[1;31mERR>\033[0m %s\n' "$*" >&2; exit 1; }

usage() {
  cat <<'EOF'
Usage: ./setup.sh [options]

Auto-detects Linux, WSL, macOS, or Windows (Git Bash) and:
  - installs build dependencies + Rust (via rustup if needed)
  - builds and installs razochar6e
  - initializes config and runs doctor
  - applies 20–80% thresholds when supported (may need sudo / Admin)

Options:
  --start N       Lower charge threshold (default: 20)
  --end N         Upper charge threshold (default: 80)
  --prefix DIR    Install binary to DIR/bin (default: ~/.local)
  --debug         Debug build instead of --release
  --no-apply      Skip applying charge limits after install
  --no-persist    Skip install-persist / Windows scheduled task
  --skip-host     WSL only: do not install Windows host helper
  --skip-build    Skip cargo build (binary must already exist)
  -h, --help      Show this help

Examples:
  ./setup.sh
  ./setup.sh --start 25 --end 85 --no-persist
  curl -fsSL https://raw.githubusercontent.com/jrb00013/razochar6e/main/setup.sh | bash
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --start) START="$2"; shift 2 ;;
    --end) END="$2"; shift 2 ;;
    --prefix) PREFIX="$2"; shift 2 ;;
    --debug) RELEASE=0; shift ;;
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
  log "Checking macOS build tools..."
  if ! xcode-select -p >/dev/null 2>&1; then
    warn "Xcode Command Line Tools not found — run: xcode-select --install"
  fi
  if have_cmd brew; then
    log "Optional SMC tools (install any one for macos_cli backend):"
    echo "  brew install batt    # https://github.com/charlie0129/batt"
    echo "  # or: battery, bclm — see docs/VENDORS.md"
  else
    warn "Homebrew not found — optional: install batt/battery/bclm for charge limits"
  fi
}

install_windows_host_via_powershell() {
  local win_root ps1
  if ! have_cmd powershell.exe && ! have_cmd pwsh.exe; then
    warn "powershell.exe not found — install Windows host manually (Admin):"
    echo "  .\\scripts\\install-windows.ps1 -Start $START -End $END"
    return 1
  fi

  local ps_exe="powershell.exe"
  have_cmd powershell.exe || ps_exe="pwsh.exe"

  if have_cmd wslpath; then
    win_root="$(wslpath -w "$REPO_ROOT")"
    ps1="${win_root}\\scripts\\install-windows.ps1"
  else
    warn "wslpath unavailable — run install-windows.ps1 on Windows as Administrator"
    return 1
  fi

  log "Installing Windows host helper (UAC prompt may appear)..."
  "$ps_exe" -NoProfile -Command \
    "Start-Process -FilePath '$ps_exe' -Verb RunAs -Wait -ArgumentList @(
      '-NoProfile','-ExecutionPolicy','Bypass',
      '-File','\"$ps1\"',
      '-Start',$START,
      '-End',$END
    )" || {
      warn "Elevated Windows install failed or was cancelled."
      warn "Run as Administrator in PowerShell:"
      echo "  cd '$win_root'; .\\scripts\\install-windows.ps1 -Start $START -End $END"
      return 1
    }
  return 0
}

run_windows_setup() {
  local ps_exe="powershell.exe"
  have_cmd powershell.exe || ps_exe="pwsh.exe"
  if ! have_cmd "$ps_exe"; then
    die "PowerShell required on Windows — open PowerShell as Administrator and run scripts/install-windows.ps1"
  fi

  log "Windows detected — running install-windows.ps1 (run as Administrator for full install)..."
  local ps1="$REPO_ROOT/scripts/install-windows.ps1"
  [[ -f "$ps1" ]] || die "Missing $ps1"

  install_rust

  local ps_args=(-Start "$START" -End "$END")
  [[ "$PERSIST" -eq 0 ]] && ps_args+=(-NoPersist)
  [[ "$SKIP_BUILD" -eq 1 ]] && ps_args+=(-NoBuild)

  "$ps_exe" -NoProfile -ExecutionPolicy Bypass -File "$ps1" "${ps_args[@]}"

  if [[ "$APPLY" -eq 1 ]]; then
    local win_dest="${LOCALAPPDATA:-$HOME/AppData/Local}/Programs/razochar6e/razochar6e.exe"
    if [[ -f "$win_dest" ]]; then
      "$win_dest" config init 2>/dev/null || true
      "$win_dest" set --start "$START" --end "$END" --save 2>/dev/null || \
        warn "Apply limits: run Admin PowerShell → razochar6e set --start $START --end $END"
    fi
  fi

  export PATH="${LOCALAPPDATA:-}/Programs/razochar6e:$PATH"
  log "Run: razochar6e probe"
}

build_and_install_unix() {
  local args=()
  [[ "$RELEASE" -eq 1 ]] && args+=(--release)
  [[ "$SKIP_BUILD" -eq 0 ]] && "$REPO_ROOT/scripts/install.sh" "${args[@]}" --prefix "$PREFIX"
  export PATH="$PREFIX/bin:$PATH"
}

path_has_sysfs_thresholds() {
  local f
  for f in /sys/class/power_supply/BAT*/charge_control_end_threshold; do
    [[ -e "$f" ]] && return 0
  done
  for f in /sys/class/power_supply/BAT*/charge_stop_threshold; do
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
        "$razo" wsl set --start "$START" --end "$END" || warn "wsl set failed — complete Windows host install first"
      fi
      ;;
    linux)
      if path_has_sysfs_thresholds; then
        if [[ "$APPLY" -eq 1 ]]; then
          log "Applying sysfs thresholds (sudo)..."
          sudo "$razo" set --start "$START" --end "$END" --save
        fi
        if [[ "$PERSIST" -eq 1 ]]; then
          sudo "$razo" install-persist --start "$START" --end "$END" || warn "install-persist failed"
        fi
      else
        warn "No charge_control_* sysfs on this machine — probe only"
        "$razo" probe || true
      fi
      ;;
    macos)
      if [[ "$APPLY" -eq 1 ]]; then
        log "Applying via macOS backend (sudo may be required)..."
        sudo "$razo" set --start "$START" --end "$END" --save 2>/dev/null \
          || "$razo" set --start "$START" --end "$END" --save \
          || warn "set failed — install batt/battery/bclm (see docs/VENDORS.md)"
      fi
      if [[ "$PERSIST" -eq 1 ]]; then
        sudo "$razo" install-persist --start "$START" --end "$END" 2>/dev/null \
          || warn "install-persist needs sudo"
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
    warn "Add to your shell rc:  export PATH=\"$PREFIX/bin:\$PATH\""
  fi
}

main() {
  PLATFORM="$(detect_platform)"
  log "Platform: $PLATFORM"
  log "Repo: $REPO_ROOT"

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
      if [[ "$SKIP_HOST" -eq 0 ]]; then
        install_windows_host_via_powershell || true
      fi
      build_and_install_unix
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
      die "Unsupported platform (uname: $(uname -a 2>/dev/null || true)). Set RAZOCHAR6E_PLATFORM=linux|wsl|macos|windows"
      ;;
  esac

  log "Setup complete."
  case "$PLATFORM" in
    wsl)
      echo "  razochar6e wsl status"
      echo "  razochar6e wsl set --start $START --end $END"
      ;;
    windows)
      echo "  razochar6e probe"
      echo "  razochar6e status"
      ;;
    *)
      echo "  razochar6e probe"
      echo "  razochar6e status"
      ;;
  esac
}

main "$@"
