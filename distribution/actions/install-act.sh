#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo 'usage: install-act.sh INSTALL_DIRECTORY' >&2
  exit 2
fi

version=0.2.89
case "$(uname -m)" in
  x86_64|amd64)
    archive=act_Linux_x86_64.tar.gz
    checksum=0191d6f1f3b716b5c55820032605d05fc3c1cdbf581ebeff655019e5dd1524c0
    ;;
  aarch64|arm64)
    archive=act_Linux_arm64.tar.gz
    checksum=daa8679ba9615a74d2d0cec321dc593f21948a2a11bb65862b063d8b930f4bcb
    ;;
  *) echo 'act requires Linux x64 or ARM64' >&2; exit 1 ;;
esac

destination="$1"
mkdir -p -- "$destination"
if [[ -x "$destination/act" ]] && [[ "$("$destination/act" --version)" == "act version $version" ]]; then
  exit 0
fi

curl --fail --location --retry 3 --silent --show-error \
  "https://github.com/nektos/act/releases/download/v${version}/${archive}" \
  --output "$destination/$archive.partial"
printf '%s  %s\n' "$checksum" "$destination/$archive.partial" | sha256sum --check --strict
tar -xzf "$destination/$archive.partial" -C "$destination" act
chmod 700 "$destination/act"
rm -f -- "$destination/$archive.partial"
"$destination/act" --version
