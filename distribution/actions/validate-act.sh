#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 0 ]]; then
  echo 'validate-act.sh accepts no arguments' >&2
  exit 2
fi

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
heap="$root/.heap"
cache="$heap/cache"
logs="$cache/logs"
mkdir -p -- "$logs"

bash "$root/distribution/actions/install-act.sh" "$cache/toolchains/act" | tee "$logs/act-bootstrap.log"
act="$cache/toolchains/act/act"

docker_runtime=''
cleanup_docker() {
  if [[ -n "$docker_runtime" ]]; then
    if [[ -f "$docker_runtime/dockerd.pid" ]]; then
      sudo -n kill "$(cat "$docker_runtime/dockerd.pid")" 2>/dev/null || true
    fi
    rm -rf -- "$docker_runtime"
  fi
}
trap cleanup_docker EXIT

if ! command -v docker >/dev/null 2>&1 || ! docker info >/dev/null 2>&1; then
  docker_tool="$cache/toolchains/docker"
  bash "$root/distribution/actions/install-docker.sh" "$docker_tool" | tee "$logs/docker-bootstrap.log"
  export PATH="$docker_tool/docker:$PATH"
  if ! sudo -n true 2>/dev/null; then
    echo 'Disposable Docker Engine startup requires passwordless sudo in the WSL distribution.' >&2
    exit 1
  fi
  docker_runtime="$(mktemp -d /tmp/kero-docker.XXXXXXXX)"
  sudo -n "$docker_tool/docker/dockerd" \
    --host="unix://$docker_runtime/docker.sock" \
    --pidfile="$docker_runtime/dockerd.pid" \
    --data-root="$docker_runtime/data" \
    --exec-root="$docker_runtime/exec" \
    --iptables=false --bridge=none --storage-driver=vfs \
    > "$logs/docker-daemon.log" 2>&1 &
  export DOCKER_HOST="unix://$docker_runtime/docker.sock"
  ready=0
  for _ in {1..60}; do
    if docker info >/dev/null 2>&1; then ready=1; break; fi
    sleep 1
  done
  if [[ "$ready" != 1 ]]; then
    tail -n 40 "$logs/docker-daemon.log" >&2
    echo 'Disposable Docker Engine did not start.' >&2
    exit 1
  fi
fi

case "$(uname -m)" in
  aarch64|arm64)
    image=catthehacker/ubuntu@sha256:c58e2b364da03b0c804c7d660f2ecbedf2f221a382b9baa0b344b0144780ff43
    platform=linux/arm64
    expected_image=sha256:10ca2cfc3a29b70e13fe0a2a9244fe7e5d24fbd7350ac4205028335c9541f926
    ;;
  x86_64|amd64)
    image=catthehacker/ubuntu@sha256:9c7b3a3613c6d8459f6fca00b2f42a6f9c02f6a16b3cb31a6fbb4727726f6395
    platform=linux/amd64
    expected_image=''
    ;;
  *)
    echo 'Local GitHub workflow replay requires Linux x64 or ARM64.' >&2
    exit 1
    ;;
esac

docker pull --platform "$platform" "$image" | tee "$logs/act-image.log"
actual_image="$(docker image inspect --platform "$platform" "$image" --format '{{.Id}}')"
if [[ -n "$expected_image" && "$actual_image" != "$expected_image" ]]; then
  echo "Runner image mismatch: expected $expected_image, received $actual_image" >&2
  exit 1
fi

snapshot="$cache/act-source"
if [[ "$snapshot" != "$root/.heap/cache/act-source" ]]; then
  echo 'Refusing to replace an unexpected act source path' >&2
  exit 1
fi
rm -rf -- "$snapshot"
mkdir -p -- "$snapshot"
(cd "$root" && git ls-files --cached --others --exclude-standard -z | tar --null --ignore-failed-read -T - -cf -) \
  | tar -xf - -C "$snapshot"
git -C "$snapshot" init --initial-branch=source -q
git -C "$snapshot" add -A
git -C "$snapshot" -c user.name='KERO local validation' \
  -c user.email='validation@localhost' commit -qm 'Validate current source snapshot'
source_sha="$(git -C "$snapshot" rev-parse HEAD)"
tree_sha="$(git -C "$snapshot" rev-parse HEAD^{tree})"
source_fingerprint="$(cd "$root" && git ls-files --cached --others --exclude-standard -z | xargs -0 -r sha256sum | sha256sum | cut -d ' ' -f 1)"

{
  printf 'source-head=%s\n' "$(git -C "$root" rev-parse HEAD)"
  printf 'snapshot-head=%s\n' "$source_sha"
  printf 'snapshot-tree=%s\n' "$tree_sha"
  printf 'source-fingerprint=%s\n' "$source_fingerprint"
  printf 'act=%s\n' "$("$act" --version)"
  printf 'docker=%s\n' "$(docker version --format '{{.Server.Version}}')"
  printf 'runner-platform=%s\n' "$platform"
  printf 'runner-image=%s\n' "$actual_image"
} > "$logs/act-inputs.txt"

common=(
  -P "ubuntu-latest=$image"
  --container-architecture "${platform#linux/}"
  --concurrent-jobs 1
  --pull=false
  --artifact-server-path "$cache/act-artifacts"
  --action-cache-path "$cache/act-actions"
  --cache-server-path "$cache/act-cache"
  --secret-file /dev/null
)

cd -- "$snapshot"
"$act" workflow_dispatch -W .github/workflows/ci.yml -j ci-gate \
  --matrix os:ubuntu-latest "${common[@]}" 2>&1 | tee "$logs/act-ci.log"
if ! grep -Fq '[CI/CI Gate' "$logs/act-ci.log" || ! grep -Fq 'Job succeeded' "$logs/act-ci.log"; then
  echo 'act did not complete the required CI gate job.' >&2
  exit 1
fi

cat > "$logs/workflow-replay.ok" <<EOF
snapshot-head=$source_sha
snapshot-tree=$tree_sha
source-fingerprint=$source_fingerprint
runner-platform=$platform
runner-image=$actual_image
scope=ci-orchestration
EOF

echo 'GitHub CI orchestration replay passed. Distribution packages are built once by the native local target providers.'
