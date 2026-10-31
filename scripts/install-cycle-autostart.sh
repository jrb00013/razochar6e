#!/usr/bin/env bash
# Install (or remove) login autostart for razochar6e 20–80% Kasa cycle.
# Linux: systemd --user unit
# macOS: LaunchAgent
#
# Usage:
#   ./scripts/install-cycle-autostart.sh           # install + start now
#   ./scripts/install-cycle-autostart.sh --uninstall
#   CYCLE_START=20 CYCLE_END=80 KASA_HOST=192.168.1.88 ./scripts/install-cycle-autostart.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DAEMON_SRC="$ROOT/scripts/cycle-daemon.sh"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/razochar6e"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/razochar6e"
ENV_FILE="$DATA_DIR/kasa.env"
DAEMON_DST="$DATA_DIR/cycle-daemon.sh"
UNIT_NAME="razochar6e-cycle"
LAUNCHD_LABEL="com.razochar6e.cycle"
UNINSTALL=0
FILES_ONLY=0

for arg in "$@"; do
  case "$arg" in
    --uninstall|-u) UNINSTALL=1 ;;
    --files-only) FILES_ONLY=1 ;;
    -h|--help)
      sed -n '2,14p' "$0"
      exit 0
      ;;
  esac
done

die() { echo "error: $*" >&2; exit 1; }
info() { echo "$*"; }

os="$(uname -s)"
case "$os" in
  Linux|Darwin) ;;
  *) die "unsupported OS '$os' (use install-cycle-autostart.ps1 on Windows)" ;;
esac

[[ -f "$DAEMON_SRC" ]] || die "missing $DAEMON_SRC"

mkdir -p "$DATA_DIR" "$CONFIG_DIR"
chmod 700 "$DATA_DIR" 2>/dev/null || true

# Copy runner into data dir so login units survive repo moves
install -m 755 "$DAEMON_SRC" "$DAEMON_DST"
# Patch ROOT detection: daemon resolves relative to scripts/; from data dir point at repo
# Keep using RAZOCHAR6E_BIN / PATH; write a tiny wrapper that sets ROOT.
cat >"$DAEMON_DST" <<EOF
#!/usr/bin/env bash
export RAZOCHAR6E_ROOT="$ROOT"
export RAZOCHAR6E_BIN="\${RAZOCHAR6E_BIN:-$ROOT/target/release/razochar6e}"
export RAZOCHAR6E_KASA_ENV="\${RAZOCHAR6E_KASA_ENV:-$ENV_FILE}"
exec bash "$DAEMON_SRC" "\$@"
EOF
chmod 755 "$DAEMON_DST"

if [[ ! -f "$ENV_FILE" ]]; then
  umask 077
  cat >"$ENV_FILE" <<'ENVEOF'
# razochar6e Kasa credentials — chmod 600, do not commit
KASA_USERNAME='you@example.com'
KASA_PASSWORD='your-password'
KASA_HOST='192.168.1.88'
CYCLE_START=20
CYCLE_END=80
CYCLE_INTERVAL=60
ENVEOF
  chmod 600 "$ENV_FILE"
  info "Created template $ENV_FILE — edit with your Kasa app login before relying on autostart."
else
  chmod 600 "$ENV_FILE" 2>/dev/null || true
fi

uninstall_linux() {
  systemctl --user disable --now "$UNIT_NAME.service" 2>/dev/null || true
  rm -f "$HOME/.config/systemd/user/${UNIT_NAME}.service"
  systemctl --user daemon-reload 2>/dev/null || true
  info "Removed systemd user unit ${UNIT_NAME}.service"
}

uninstall_macos() {
  local plist="$HOME/Library/LaunchAgents/${LAUNCHD_LABEL}.plist"
  launchctl bootout "gui/$(id -u)/${LAUNCHD_LABEL}" 2>/dev/null || true
  launchctl unload "$plist" 2>/dev/null || true
  rm -f "$plist"
  info "Removed LaunchAgent ${LAUNCHD_LABEL}"
}

if [[ "$UNINSTALL" -eq 1 ]]; then
  case "$os" in
    Linux) uninstall_linux ;;
    Darwin) uninstall_macos ;;
  esac
  exit 0
fi

# Prefer release binary; build if missing
if [[ ! -x "$ROOT/target/release/razochar6e" ]]; then
  if command -v cargo >/dev/null 2>&1; then
    info "Building release binary..."
    (cd "$ROOT" && cargo build --release -q)
  else
    die "no release binary at $ROOT/target/release/razochar6e and cargo not found"
  fi
fi

if [[ "$FILES_ONLY" -eq 1 ]]; then
  info "Files only: $DAEMON_DST + $ENV_FILE (no systemd/launchd)"
  exit 0
fi

install_linux() {
  local unit_dir="$HOME/.config/systemd/user"
  mkdir -p "$unit_dir"
  cat >"$unit_dir/${UNIT_NAME}.service" <<EOF
[Unit]
Description=razochar6e Kasa 20-80 charge/drain cycle
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
Environment=RAZOCHAR6E_KASA_ENV=$ENV_FILE
Environment=RAZOCHAR6E_BIN=$ROOT/target/release/razochar6e
# cycle-daemon.sh stays in the foreground by default (use --background only for ad-hoc nohup).
ExecStart=$DAEMON_DST
Restart=on-failure
RestartSec=15

[Install]
WantedBy=default.target
EOF
  systemctl --user daemon-reload
  systemctl --user enable --now "${UNIT_NAME}.service"
  # Linger so it survives logout on machines that support it (optional)
  if command -v loginctl >/dev/null 2>&1; then
    loginctl enable-linger "$USER" 2>/dev/null || true
  fi
  info "Installed systemd --user unit: ${UNIT_NAME}.service (enable --now)"
  systemctl --user --no-pager status "${UNIT_NAME}.service" | head -15 || true
}

install_macos() {
  local plist="$HOME/Library/LaunchAgents/${LAUNCHD_LABEL}.plist"
  mkdir -p "$HOME/Library/LaunchAgents"
  cat >"$plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>${LAUNCHD_LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>/bin/bash</string>
    <string>${DAEMON_DST}</string>
  </array>
  <key>EnvironmentVariables</key>
  <dict>
    <key>RAZOCHAR6E_KASA_ENV</key><string>${ENV_FILE}</string>
    <key>RAZOCHAR6E_BIN</key><string>${ROOT}/target/release/razochar6e</string>
  </dict>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>StandardOutPath</key><string>/tmp/razochar6e-cycle.log</string>
  <key>StandardErrorPath</key><string>/tmp/razochar6e-cycle.log</string>
</dict></plist>
EOF
  launchctl bootout "gui/$(id -u)/${LAUNCHD_LABEL}" 2>/dev/null || true
  launchctl bootstrap "gui/$(id -u)" "$plist" 2>/dev/null \
    || launchctl load -w "$plist"
  info "Installed LaunchAgent: $plist (RunAtLoad + KeepAlive)"
}

case "$os" in
  Linux) install_linux ;;
  Darwin) install_macos ;;
esac

info ""
info "Credentials file: $ENV_FILE (chmod 600)"
info "Daemon wrapper:   $DAEMON_DST"
info "Log:              /tmp/razochar6e-cycle.log"
info "Uninstall:        $0 --uninstall"
