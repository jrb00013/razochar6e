#!/usr/bin/env bash
# Start razochar6e sleepcut in the background (sleep → Kasa OFF, wake → ON).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${ROOT}/target/release/razochar6e"
LOG="${SLEEPCUT_LOG:-/tmp/razochar6e-sleepcut.log}"
ENV_FILE="${KASA_ENV_FILE:-/tmp/razo-kasa-env.sh}"

if [[ ! -x "$BIN" ]]; then
  echo "error: missing $BIN — run: cargo build --release" >&2
  exit 1
fi
if [[ -f "$ENV_FILE" ]]; then
  # shellcheck disable=SC1090
  . "$ENV_FILE"
fi
if [[ -z "${KASA_USERNAME:-}" || -z "${KASA_PASSWORD:-}" ]]; then
  echo "error: set KASA_USERNAME / KASA_PASSWORD (or create $ENV_FILE)" >&2
  exit 1
fi

# Stop prior sleepcut / cycle daemons (match argv0 only)
pkill -x razochar6e 2>/dev/null || true
sleep 0.3

# Cap log growth
if [[ -f "$LOG" ]]; then
  sz=$(wc -c <"$LOG" || echo 0)
  if (( sz > 5242880 )); then
    tail -n 2000 "$LOG" > "${LOG}.tmp" && mv "${LOG}.tmp" "$LOG"
  fi
fi

HOST_ARG=()
if [[ -n "${KASA_HOST:-}" ]]; then
  HOST_ARG=(--host "$KASA_HOST")
fi

nohup "$BIN" sleepcut "${HOST_ARG[@]}" >>"$LOG" 2>&1 &
echo "sleepcut started pid=$! log=$LOG"
sleep 1
tail -n 15 "$LOG" || true
