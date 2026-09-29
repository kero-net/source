#!/usr/bin/env bash
set -euo pipefail
: "${KERO_ZIG:?KERO_ZIG is required}"
args=()
for argument in "$@"; do
  case "$argument" in
    -Wl,-rpath-link,*) ;;
    *) args+=("$argument") ;;
  esac
done
exec "$KERO_ZIG" c++ -target x86_64-linux-gnu.2.28 "${args[@]}"
