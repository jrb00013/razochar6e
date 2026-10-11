#!/usr/bin/env bash
# Run razochar6e cycle (default 20–80%) in the foreground or as a nohup daemon.
# Used by install-cycle-autostart.{sh,ps1}. Credentials: env or kasa.env.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${RAZOCHAR6E_BIN:-}"
if [[ -z "$BIN" ]]; then
  if [[ -x "$ROOT/target/release/razochar6e" ]]; then
    BIN="$ROOT/target/release/razochar6e"
  elif command -v razochar6e >/dev/null 2>&1; then
    BIN="$(command -v razochar6e)"
  else
    echo "error: razochar6e binary not found (build release or set RAZOCHAR6E_BIN)" >&2
    exit 1
  fi
fi

DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/razochar6e"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/razochar6e"
ENV_FILE="${RAZOCHAR6E_KASA_ENV:-$DATA_DIR/kasa.env}"
LOG="${RAZOCHAR6E_CYCLE_LOG:-/tmp/razochar6e-cycle.log}"

# Load secrets (never commit kasa.env)
if [[ -f "$ENV_FILE" ]]; then
  set -a
  # shellcheck disable=SC1090
  . "$ENV_FILE"
  set +a
fi

# Task Scheduler / nohup often strip PATH — keep absolute basics.
export PATH="/usr/local/bin:/usr/bin:/bin:${PATH:-}"

# On WSL, force Windows Python for KLAP (LAN reachable from Windows, not always from Linux).
if [[ -z "${KASA_PYTHON:-}" ]]; then
  if [[ -n "${WSL_DISTRO_NAME:-}" || -f /proc/version ]] && grep -qi microsoft /proc/version 2>/dev/null; then
    for cand in \
      /mnt/c/Users/*/AppData/Local/Programs/Python/Python3*/python.exe \
      "$(command -v python.exe 2>/dev/null || true)"; do
      if [[ -n "$cand" && -x "$cand" ]]; then
        export KASA_PYTHON="$cand"
        break
      fi
    done
  fi
fi

HOST="${KASA_HOST:-}"
if [[ -z "$HOST" && -f "$CONFIG_DIR/config.toml" ]]; then
  HOST="$(grep -E '^\s*kasa_host\s*=' "$CONFIG_DIR/config.toml" | head -1 | sed -E 's/.*=\s*"?([^"]+)"?.*/\1/' || true)"
fi
HOST="${HOST:-192.168.1.88}"
START="${CYCLE_START:-20}"
END="${CYCLE_END:-80}"
INTERVAL="${CYCLE_INTERVAL:-60}"

# Keep a Windows-side copy of the helper for python.exe
if [[ -f "$ROOT/scripts/kasa_plug.py" ]]; then
  mkdir -p /mnt/c/Users/"${USER:-josep}"/AppData/Local/razochar6e 2>/dev/null || true
  cp -f "$ROOT/scripts/kasa_plug.py" /mnt/c/Users/"${USER:-josep}"/AppData/Local/razochar6e/kasa_plug.py 2>/dev/null || true
fi

if [[ -z "${KASA_USERNAME:-}" || -z "${KASA_PASSWORD:-}" ]]; then
  echo "error: set KASA_USERNAME and KASA_PASSWORD (or create $ENV_FILE)" >&2
  exit 1
fi

mkdir -p "$(dirname "$LOG")"
if [[ -f "$LOG" ]]; then
  sz=$(wc -c <"$LOG" || echo 0)
  if (( sz > 5242880 )); then
    tail -n 2000 "$LOG" >"${LOG}.tmp" && mv "${LOG}.tmp" "$LOG"
  fi
fi

# Stop a prior instance (match binary name only)
if command -v pkill >/dev/null 2>&1; then
  pkill -x razochar6e 2>/dev/null || true
  sleep 0.4
fi

echo "$(date -Iseconds 2>/dev/null || date) cycle start host=$HOST band=${START}-${END}% interval=${INTERVAL}s bin=$BIN" >>"$LOG"

if [[ "${1:-}" == "--foreground" ]]; then
  exec "$BIN" cycle --host "$HOST" --start "$START" --end "$END" --interval "$INTERVAL" >>"$LOG" 2>&1
fi

nohup "$BIN" cycle --host "$HOST" --start "$START" --end "$END" --interval "$INTERVAL" \
  >>"$LOG" 2>&1 &
echo "$(date -Iseconds 2>/dev/null || date) cycle pid=$! log=$LOG" >>"$LOG"
echo "cycle started pid=$! log=$LOG"
