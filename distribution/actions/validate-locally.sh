#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 0 ]]; then
  echo 'validate-locally.sh accepts no arguments' >&2
  exit 2
fi

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
bash "$root/distribution/actions/validate-act.sh"
export KERO_ACT_VALIDATED=1
bash "$root/distribution/actions/kero-build.sh"
