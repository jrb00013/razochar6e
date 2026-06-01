#!/usr/bin/env bash
# Build razochar6e with visible progress (avoids "frozen" appearance on first compile).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RELEASE=1

while [[ $# -gt 0 ]]; do
  case "$1" in
    --release) RELEASE=1; shift ;;
    --debug) RELEASE=0; shift ;;
    *) echo "Usage: $0 [--release|--debug]" >&2; exit 1 ;;
  esac
done

cd "$REPO_ROOT"
export CARGO_TERM_PROGRESS=always
export CARGO_INCREMENTAL=1

if [[ "$RELEASE" -eq 1 ]]; then
  MODE=(--release)
  OUT="$REPO_ROOT/target/release/razochar6e"
else
  MODE=()
  OUT="$REPO_ROOT/target/debug/razochar6e"
fi

echo "==> Compiling razochar6e (${MODE[*]:-debug})..."
echo "    First build often takes 5–20 minutes on Windows/WSL — not stuck."
echo "    Proc-macros (serde/clap) at ~49/61 are the slowest step."
echo "    Tip next time: ./setup.sh  (downloads prebuilt binary, skips this)"
echo ""

heartbeat() {
  local n=0
  while [[ -f "$REPO_ROOT/.razochar6e-build.lock" ]]; do
    n=$((n + 1))
    echo "    … still compiling (${n}×30s elapsed) — leave this running"
    sleep 30
  done
}

touch "$REPO_ROOT/.razochar6e-build.lock"
heartbeat &
HB=$!

set +e
cargo build "${MODE[@]}" --message-format=human 2>&1 | tee /tmp/razochar6e-build.log
STATUS=${PIPESTATUS[0]}
set -e

rm -f "$REPO_ROOT/.razochar6e-build.lock"
wait "$HB" 2>/dev/null || true

if [[ "$STATUS" -ne 0 ]]; then
  echo "Build failed. Log: /tmp/razochar6e-build.log" >&2
  exit "$STATUS"
fi

echo "==> Built: $OUT"
