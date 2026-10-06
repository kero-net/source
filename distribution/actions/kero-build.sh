#!/usr/bin/env sh
# The sole POSIX build entry point. It always builds every enabled distribution.
set -eu

if [ "$#" -ne 0 ]; then
  echo "kero-build accepts no arguments; it always builds every enabled distribution." >&2
  exit 2
fi

workspace=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd -- "$workspace"
export PATH="$HOME/.cargo/bin:$PATH"
export WSLENV="${WSLENV:+$WSLENV:}CARGO_TARGET_DIR/p"

git_directory=$(git -C "$workspace" rev-parse --absolute-git-dir)
lock_directory="$git_directory/kero-local-build.lock"
if ! mkdir -- "$lock_directory" 2>/dev/null; then
  echo 'KERO local validation is already running for this worktree.' >&2
  exit 1
fi
cleanup_lock() {
  status=$?
  if [ -n "${source_fingerprint:-}" ] && [ -d "$heap" ]; then
    if [ "$status" -eq 0 ]; then
      rm -f -- "$heap/.resume-source"
      printf '%s\n' "$source_fingerprint" >"$heap/.complete-source"
    else
      printf '%s\n' "$source_fingerprint" >"$heap/.resume-source"
      rm -f -- "$heap/.complete-source"
    fi
  fi
  rmdir -- "$lock_directory" 2>/dev/null || true
  if [ -t 1 ]; then
    printf '\033[0m\033[?25h\033[r'
  fi
}
trap cleanup_lock EXIT HUP INT TERM

heap="$workspace/.heap"
logs="$heap/cache/logs"
if [ "$(uname -s)" = Linux ]; then
  workspace_key=$(printf '%s' "$workspace" | sha256sum | cut -c 1-20)
  export KERO_WSL_WORK_ROOT="${TMPDIR:-/tmp}/kero/workspaces/$workspace_key"
  mkdir -p -- "$KERO_WSL_WORK_ROOT"
  if [ -n "${WSL_INTEROP:-}" ]; then
    work_fs=$(stat -f -c %T "$KERO_WSL_WORK_ROOT")
    case "$work_fs" in
      ext2/ext3|ext4) ;;
      *) echo "KERO_WSL_WORK_ROOT must be on WSL ext4, found $work_fs" >&2; exit 1 ;;
    esac
  fi
fi
source_fingerprint=$(git ls-files --cached --others --exclude-standard -z | xargs -0 -r sha256sum 2>/dev/null | sha256sum | cut -d ' ' -f 1)
resume=0
if [ "${KERO_CLEAN:-0}" = 1 ]; then
  rm -rf -- "$heap"
  if [ -n "${KERO_WSL_WORK_ROOT:-}" ]; then
    rm -rf -- "$KERO_WSL_WORK_ROOT"
    mkdir -p -- "$KERO_WSL_WORK_ROOT"
  fi
elif [ -f "$heap/.complete-source" ] && [ "$(cat "$heap/.complete-source")" = "$source_fingerprint" ] \
  && grep -q 'Status: \*\*complete\*\*' "$heap/release/RELEASE-MESSAGE.md" 2>/dev/null \
  && [ -f "$heap/release/artifacts/SHA256SUMS" ] \
  && [ "$(wc -l < "$heap/release/artifacts/SHA256SUMS")" -eq 4 ] \
  && (cd "$heap/release/artifacts" && sha256sum -c SHA256SUMS); then
  echo 'All four completed artifacts match the unchanged source and recorded checksums.'
  exit 0
elif [ -f "$heap/.resume-source" ] && [ "$(cat "$heap/.resume-source")" = "$source_fingerprint" ]; then
  resume=1
  echo 'Resuming failed validation from verified completed artifacts.'
elif [ "${KERO_REUSE_VERIFIED_ARTIFACTS:-0}" = 1 ] && [ -f "$heap/.resume-source" ]; then
  resume=1
  echo 'Reusing checksum-verified artifacts after an operator-confirmed packaging-only correction.'
elif [ -f "$heap/.resume-source" ] || grep -q 'Status: \*\*incomplete\*\*' "$heap/release/RELEASE-MESSAGE.md" 2>/dev/null; then
  echo 'Source changed since the failed run; retaining toolchains and compiler cache while rebuilding targets.'
  rm -rf -- "$heap/cache/build/releases" "$heap/release/artifacts/distributions"
  rm -f -- "$heap/release/artifacts/SHA256SUMS" "$heap/release/RELEASE-MESSAGE.md"
elif [ -f "$heap/.complete-source" ]; then
  echo 'Source changed; retaining toolchains and compiler cache while rebuilding targets.'
  rm -rf -- "$heap/cache/build/releases" "$heap/release/artifacts/distributions"
  rm -f -- "$heap/release/artifacts/SHA256SUMS" "$heap/release/RELEASE-MESSAGE.md"
else
  rm -rf -- "$heap/cache/build" "$heap/release" "$heap/pages" "$heap/repo" "$heap/packages"
  if [ -n "${KERO_WSL_WORK_ROOT:-}" ]; then rm -rf -- "$KERO_WSL_WORK_ROOT"; fi
fi
mkdir -p -- "$logs" "$heap/pages" "$heap/repo/stable" "$heap/repo/beta" "$heap/repo/canary" "$heap/packages" "$heap/release/artifacts"
export KERO_RESUME="$resume"

# Use host Pandoc through WSL when available; keep the shim disposable.
if ! command -v pandoc >/dev/null 2>&1 && command -v pandoc.exe >/dev/null 2>&1; then
  mkdir -p -- "$heap/cache/build/bin"
  cat >"$heap/cache/build/bin/pandoc" <<'EOF'
#!/usr/bin/env sh
exec pandoc.exe "$@"
EOF
  chmod +x "$heap/cache/build/bin/pandoc"
  export PATH="$heap/cache/build/bin:$PATH"
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo 'KERO cannot start because Cargo is not installed in this WSL distribution.' >&2
  echo 'Install Rust/Cargo in WSL, then run Kero: Validate Locally again.' >&2
  exit 1
fi

lune_directory="$heap/cache/toolchains/lune"
bash "$workspace/distribution/actions/install-lune.sh" "$lune_directory" >"$logs/bootstrap.log" 2>&1 || {
  echo 'Could not install the pinned Lune release. See .heap/cache/logs/bootstrap.log.' >&2
  exit 1
}
export PATH="$lune_directory:$PATH"
kero_luau="$lune_directory/lune"

export KERO_LUAU="$kero_luau"
export KERO_HEAP_PREPARED=1
if [ -t 1 ]; then
  export KERO_INTERACTIVE=1
else
  export KERO_INTERACTIVE=0
fi
"$kero_luau" run "$workspace/distribution/scripts/build/build-every-distribution.luau"
