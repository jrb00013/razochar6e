#!/usr/bin/env bash
# Download official release tarball and install razochar6e + scripts.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
ASSET_SUFFIX="${1:-}"

GITHUB_REPO="jrb00013/razochar6e"

version_from_cargo() {
  sed -n 's/^version = "\(.*\)"/\1/p' "$REPO_ROOT/Cargo.toml" | head -1
}

resolve_release_tag() {
  local ver tag
  ver="$(version_from_cargo)"
  tag="v${ver}"
  if curl -fsSL "https://api.github.com/repos/${GITHUB_REPO}/releases/tags/${tag}" >/dev/null 2>&1; then
    echo "$tag"
    return
  fi
  curl -fsSL "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" \
    | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -1
}

default_asset_suffix() {
  local uname_s uname_m
  uname_s="$(uname -s)"
  uname_m="$(uname -m)"
  case "$uname_s" in
    Linux) echo "linux-x86_64" ;;
    Darwin)
      if [[ "$uname_m" == "arm64" ]]; then echo "macos-aarch64"; else echo "macos-x86_64"; fi
      ;;
    MINGW*|MSYS*|CYGWIN*) echo "windows-x86_64" ;;
    *) echo "linux-x86_64" ;;
  esac
}

find_binary_in_dir() {
  local dir="$1"
  if [[ -f "$dir/razochar6e.exe" ]]; then
    echo "$dir/razochar6e.exe"
    return 0
  fi
  if [[ -f "$dir/razochar6e" ]]; then
    echo "$dir/razochar6e"
    return 0
  fi
  return 1
}

ASSET_SUFFIX="${ASSET_SUFFIX:-$(default_asset_suffix)}"
TAG="$(resolve_release_tag)"
VER="${TAG#v}"
ARCHIVE="razochar6e-${VER}-${ASSET_SUFFIX}.tar.gz"
URL="https://github.com/${GITHUB_REPO}/releases/download/${TAG}/${ARCHIVE}"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "==> Downloading ${URL}"
curl -fL --progress-bar "$URL" -o "$TMP/archive.tar.gz"

tar -xzf "$TMP/archive.tar.gz" -C "$TMP"

BIN_SRC=""
if ! BIN_SRC="$(find_binary_in_dir "$TMP")"; then
  echo "Binary not found in archive. Contents:" >&2
  find "$TMP" -maxdepth 2 -type f >&2
  exit 1
fi

mkdir -p "$PREFIX/bin"
install -m 755 "$BIN_SRC" "$PREFIX/bin/razochar6e"

if [[ -d "$TMP/scripts" ]]; then
  mkdir -p "$PREFIX/share/razochar6e"
  cp -a "$TMP/scripts" "$PREFIX/share/razochar6e/"
fi

echo "==> Installed prebuilt razochar6e ${VER} (${ASSET_SUFFIX}) → $PREFIX/bin/razochar6e"
