#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo 'usage: install-lune.sh INSTALL_DIRECTORY' >&2
  exit 2
fi

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
manifest="$root/distribution/tools/lune.toml"
version="$(sed -n 's/^version = "\([^"]*\)"$/\1/p' "$manifest")"
if [[ -z "$version" ]]; then
  echo 'Lune version is missing from distribution/tools/lune.toml' >&2
  exit 1
fi

case "${KERO_LUNE_OS:-$(uname -s)}" in
  Linux) platform=linux ;;
  macOS|Darwin) platform=macos ;;
  Windows|MINGW*|MSYS*) platform=windows ;;
  *) echo 'Unsupported Lune host OS' >&2; exit 1 ;;
esac
case "${KERO_LUNE_ARCH:-$(uname -m)}" in
  X64|x86_64|amd64) architecture=x86_64 ;;
  ARM64|aarch64|arm64) architecture=aarch64 ;;
  *) echo 'Unsupported Lune host architecture' >&2; exit 1 ;;
esac

key="${platform}_${architecture}"
checksum="$(sed -n "s/^${key} = \"\([a-f0-9]*\)\"$/\1/p" "$manifest")"
if [[ ! "$checksum" =~ ^[a-f0-9]{64}$ ]]; then
  echo "Missing SHA-256 checksum for $key" >&2
  exit 1
fi

destination="$1"
executable=lune
if [[ "$platform" == windows ]]; then executable=lune.exe; fi
mkdir -p -- "$destination"
if [[ -x "$destination/$executable" ]] && "$destination/$executable" --version | grep -Fq "$version"; then
  echo "Using Lune $version from $destination"
  exit 0
fi

scratch="$(mktemp -d)"
trap 'rm -rf -- "$scratch"' EXIT
archive="lune-${version}-${platform}-${architecture}.zip"
curl --fail --location --retry 3 --silent --show-error \
  "https://github.com/lune-org/lune/releases/download/v${version}/${archive}" \
  --output "$scratch/$archive"
printf '%s  %s\n' "$checksum" "$scratch/$archive" | sha256sum --check --strict
unzip -q "$scratch/$archive" "$executable" -d "$scratch"
cp -- "$scratch/$executable" "$destination/$executable"
chmod +x "$destination/$executable"
"$destination/$executable" --version | grep -F "$version"
