#!/usr/bin/env bash
# Install razochar6e to ~/.local/bin (or PREFIX)
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
RELEASE=0

usage() {
  echo "Usage: $0 [--release] [--prefix DIR]"
  exit 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --release) RELEASE=1; shift ;;
    --prefix) PREFIX="$2"; shift 2 ;;
    -h|--help) usage ;;
    *) echo "Unknown: $1"; usage ;;
  esac
done

cd "$REPO_ROOT"
if [[ "$RELEASE" -eq 1 ]]; then
  cargo build --release
  BIN="$REPO_ROOT/target/release/razochar6e"
else
  cargo build
  BIN="$REPO_ROOT/target/debug/razochar6e"
fi

mkdir -p "$PREFIX/bin"
install -m 755 "$BIN" "$PREFIX/bin/razochar6e"

echo "Installed razochar6e → $PREFIX/bin/razochar6e"
echo "Ensure $PREFIX/bin is on your PATH."
echo ""
echo "Next:"
echo "  razochar6e probe"
echo "  sudo razochar6e set --start 20 --end 80 --save"
echo "  sudo razochar6e install-persist"
