#!/usr/bin/env sh
# The sole POSIX build entry point. It always builds every enabled distribution.
set -eu

if [ "$#" -ne 0 ]; then
  echo "kero-build accepts no arguments; it always builds every enabled distribution." >&2
  exit 2
fi

workspace=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
export PATH="$HOME/.cargo/bin:$PATH"
export WSLENV="${WSLENV:+$WSLENV:}CARGO_TARGET_DIR/p"

git_directory=$(git -C "$workspace" rev-parse --absolute-git-dir)
lock_directory="$git_directory/kero-local-build.lock"
if ! mkdir -- "$lock_directory" 2>/dev/null; then
  echo 'KERO local validation is already running for this worktree.' >&2
  exit 1
fi
cleanup_lock() {
  rmdir -- "$lock_directory" 2>/dev/null || true
}
trap cleanup_lock EXIT HUP INT TERM

heap="$workspace/.heap"
logs="$heap/logs"
rm -rf -- "$heap"
mkdir -p -- "$logs"

if ! command -v cargo >/dev/null 2>&1; then
  echo 'KERO cannot start because Cargo is not installed in this WSL distribution.' >&2
  echo 'Install Rust/Cargo in WSL, then run Kero: Validate Locally again.' >&2
  exit 1
fi

lune_version=''
if command -v lune >/dev/null 2>&1; then
  lune_version=$(lune --version 2>/dev/null || true)
fi

if printf '%s' "$lune_version" | grep -q '0\.10\.5'; then
  kero_luau=$(command -v lune)
else
  printf '%s\n\n' 'KERO requires Lune 0.10.5 (the pinned Luau VM) inside WSL.'
  printf '%s' 'Install Lune 0.10.5 now with Cargo? [Y/n] '
  read -r answer

  case "$answer" in
    ''|y|Y|yes|YES|Yes)
      if ! command -v cargo >/dev/null 2>&1; then
        echo 'Cannot install Lune because Cargo is not installed in this WSL distribution.' >&2
        echo 'Install Rust/Cargo in WSL, then run Kero: Validate Locally again.' >&2
        exit 1
      fi
      printf '%s\r' 'Installing Lune 0.10.5 with Cargo...'
      if ! cargo install lune --version 0.10.5 --locked --force >"$logs/bootstrap.log" 2>&1; then
        printf '\n%s\n' 'Lune installation failed. See .heap/logs/bootstrap.log.' >&2
        exit 1
      fi
      hash -r
      kero_luau=$(command -v lune)
      if ! "$kero_luau" --version | grep -q '0\.10\.5'; then
        printf '\n%s\n' 'Cargo finished, but Lune 0.10.5 could not be verified.' >&2
        exit 1
      fi
      printf '%s\n' 'Installed Lune 0.10.5.                     '
      ;;
    *)
      echo 'Lune was not installed; local validation was not started.' >&2
      exit 1
      ;;
  esac
fi

export KERO_LUAU="$kero_luau"
export KERO_HEAP_PREPARED=1
if [ -t 1 ]; then
  export KERO_INTERACTIVE=1
else
  export KERO_INTERACTIVE=0
fi
"$kero_luau" run "$workspace/distribution/scripts/build/build-every-distribution.luau"
