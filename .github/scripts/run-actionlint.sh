#!/usr/bin/env bash
set -euo pipefail

version="1.7.12"
archive_name="actionlint_${version}_linux_amd64.tar.gz"
expected_sha256="8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8"

refresh=false
case "${1:-}" in
	"") ;;
	--refresh) refresh=true ;;
	*)
		echo "Usage: $0 [--refresh]" >&2
		exit 2
		;;
esac

if [ -n "${ACTIONLINT_BIN:-}" ]; then
	if [ ! -x "$ACTIONLINT_BIN" ]; then
		echo "ACTIONLINT_BIN is not executable: $ACTIONLINT_BIN" >&2
		exit 1
	fi
	exec "$ACTIONLINT_BIN" -color
fi

repository_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
tool_root="$repository_root/.generated/.cache/actionlint"
binary="$tool_root/actionlint"
archive="$tool_root/$archive_name"

mkdir -p "$tool_root"
if [ "$refresh" = true ]; then
	rm -f -- "$binary" "$archive"
fi
if [ ! -x "$binary" ]; then
	curl --fail --location --silent --show-error \
		"https://github.com/rhysd/actionlint/releases/download/v$version/$archive_name" \
		--output "$archive"
	printf '%s  %s\n' "$expected_sha256" "$archive" | sha256sum --check --strict
	tar -xzf "$archive" -C "$tool_root" actionlint
	chmod 700 "$binary"
fi

exec "$binary" -color
