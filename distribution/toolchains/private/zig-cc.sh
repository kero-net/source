#!/usr/bin/env bash
set -euo pipefail
: "${KERO_ZIG:?KERO_ZIG is required}"
exec "$KERO_ZIG" cc -target x86_64-linux-gnu.2.28 "$@"
