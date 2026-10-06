#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo 'usage: install-docker.sh INSTALL_DIRECTORY' >&2
  exit 2
fi

version=29.1.3
case "$(uname -m)" in
  x86_64|amd64)
    architecture=x86_64
    checksum=c019c608ba2bb009dd673f3230e4d743f36a78d36166c6c2444c05d0aa9ff0d9
    ;;
  aarch64|arm64)
    architecture=aarch64
    checksum=2219f3ac48727d8c5546b13d5d271a58856188ecb8ff1f71f9e2f9aefde3ea76
    ;;
  *) echo 'Docker Engine bootstrap requires Linux x64 or ARM64' >&2; exit 1 ;;
esac

destination="$1"
archive="$destination/docker-$version-$architecture.tgz"
mkdir -p -- "$destination"
if [[ ! -x "$destination/docker/dockerd" ]]; then
  if [[ ! -f "$archive" ]]; then
    curl --fail --location --retry 3 --silent --show-error \
      "https://download.docker.com/linux/static/stable/$architecture/docker-$version.tgz" \
      --output "$archive.partial"
    mv -- "$archive.partial" "$archive"
  fi
  printf '%s  %s\n' "$checksum" "$archive" | sha256sum --check --strict
  tar -xzf "$archive" -C "$destination"
fi
"$destination/docker/docker" --version
