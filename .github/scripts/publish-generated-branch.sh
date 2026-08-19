#!/usr/bin/env bash
set -euo pipefail

publication_path="${1:?publication payload path is required}"
remote_url="${2:?remote URL is required}"
channel="${3:?channel is required}"
version="${4:?version is required}"
source_commit="${5:?source commit SHA is required}"
github_output="${6:-}"

case "$channel" in
	canary|beta|stable) ;;
	*)
		echo "Unsupported publication channel: $channel" >&2
		exit 1
		;;
esac

if [[ ! "$source_commit" =~ ^[0-9a-f]{40}$ ]]; then
	echo "source commit must be a full lowercase 40-character SHA" >&2
	exit 1
fi
if [ ! -d "$publication_path" ]; then
	echo "Missing publication payload: $publication_path" >&2
	exit 1
fi
if [ -z "$(git config user.name || true)" ] || [ -z "$(git config user.email || true)" ]; then
	echo "Git user.name and user.email must be configured before publication" >&2
	exit 1
fi

publication_path="$(cd -- "$publication_path" && pwd)"
worktree_path="$(mktemp -d)"
cleanup() { rm -rf -- "$worktree_path"; }
trap cleanup EXIT

git clone --quiet --no-checkout "$remote_url" "$worktree_path"
cd "$worktree_path"

if git ls-remote --exit-code origin "refs/heads/$channel" >/dev/null 2>&1; then
	git fetch --quiet --no-tags origin "refs/heads/$channel:refs/remotes/origin/$channel"
	git checkout --quiet --orphan "generated-$channel" "origin/$channel"
	push_mode=(--force-with-lease="refs/heads/$channel:$(git rev-parse "origin/$channel")")
else
	git checkout --quiet --orphan "generated-$channel"
	push_mode=(--force)
fi

git rm -rf . >/dev/null 2>&1 || true
find . -mindepth 1 -maxdepth 1 ! -name .git -exec rm -rf -- {} +
cp -R "$publication_path"/. .
git add -A
git commit -S -m "Generate $channel $version" -m "Source commit: $source_commit"
git verify-commit HEAD
git push --quiet origin "HEAD:refs/heads/$channel" "${push_mode[@]}"
git fetch --quiet --no-tags origin "refs/heads/$channel:refs/remotes/origin/$channel"
generated_commit="$(git rev-parse "origin/$channel")"

if [ -n "$github_output" ]; then
	echo "generated_commit=$generated_commit" >> "$github_output"
fi
echo "Published $channel $version at $generated_commit from source $source_commit"
