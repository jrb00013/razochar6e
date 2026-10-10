#!/usr/bin/env bash
# Back-compat entrypoint — installs Linux/macOS login autostart for the 20–80 cycle.
exec "$(cd "$(dirname "$0")" && pwd)/install-cycle-autostart.sh" "$@"
