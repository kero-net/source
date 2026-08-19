#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from datetime import datetime, timezone
from pathlib import Path, PurePosixPath
from typing import Any

import yaml


SHA = re.compile(r"^[0-9a-f]{40}$")
CHANNELS = {"beta", "canary", "stable"}
RELEASE_ID = re.compile(r"^\d{4}\.(?:0[1-9]|1[0-2])\.[1-9]\d*-(?:regular|hotfix|security)$")
DEFAULT_LOCALE = "en-US"
HARD_DENY = {".git", ".generated", ".agents", ".workspace", ".venv", "AGENTS.md", "site"}
HARD_DENY_PATHS = {PurePosixPath(".vscode/settings.json")}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build a generated publication payload.")
    parser.add_argument("--source", required=True)
    parser.add_argument("--destination", required=True)
    parser.add_argument("--config", required=True)
    parser.add_argument("--channel", required=True, choices=sorted(CHANNELS))
    parser.add_argument("--version", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--generated-at")
    return parser.parse_args()


def relative_path(value: Any, field: str, *, block_private: bool = True) -> PurePosixPath:
    if not isinstance(value, str) or not value.strip():
        raise SystemExit(f"{field} entries must be non-empty strings")
    path = PurePosixPath(value)
    if path.is_absolute() or ".." in path.parts or "." in path.parts:
        raise SystemExit(f"Unsafe {field} path: {value}")
    if block_private and (
        path.parts[0] in HARD_DENY
        or any(path == denied or denied in path.parents for denied in HARD_DENY_PATHS)
    ):
        raise SystemExit(f"Private or generated path cannot be published: {value}")
    return path


def load_config(
    path: Path,
) -> tuple[list[PurePosixPath], dict[str, list[PurePosixPath]], list[PurePosixPath], list[PurePosixPath]]:
    data = yaml.safe_load(path.read_text(encoding="utf-8"))
    allowed = {"enabled", "include", "channel_include", "required", "exclude"}
    if not isinstance(data, dict) or set(data) != allowed:
        raise SystemExit(
            "Publication config must contain only enabled, include, channel_include, required, and exclude"
        )
    if data["enabled"] is not True:
        raise SystemExit("Publication is disabled in configuration")
    for key in ("include", "required", "exclude"):
        if not isinstance(data[key], list):
            raise SystemExit(f"Publication config {key} must be a list")
    include = [relative_path(item, "include") for item in data["include"]]
    channel_data = data["channel_include"]
    if not isinstance(channel_data, dict) or set(channel_data) != CHANNELS:
        raise SystemExit("Publication config channel_include must define canary, beta, and stable")
    channel_include: dict[str, list[PurePosixPath]] = {}
    for channel, values in channel_data.items():
        if not isinstance(values, list):
            raise SystemExit(f"Publication config channel_include.{channel} must be a list")
        channel_include[channel] = [relative_path(item, f"channel_include.{channel}") for item in values]
    required = [relative_path(item, "required") for item in data["required"]]
    exclude = [relative_path(item, "exclude", block_private=False) for item in data["exclude"]]
    if not include or not required:
        raise SystemExit("Enabled publication requires non-empty include and required lists")
    if len(set(include)) != len(include) or len(set(required)) != len(required):
        raise SystemExit("Publication include and required paths must be unique")
    return include, channel_include, required, exclude


def is_excluded(path: PurePosixPath, exclusions: list[PurePosixPath]) -> bool:
    return any(path == item or item in path.parents for item in exclusions)


def safe_source(root: Path, relative: PurePosixPath) -> Path:
    source = root.joinpath(*relative.parts)
    resolved = source.resolve(strict=True)
    try:
        resolved.relative_to(root.resolve(strict=True))
    except ValueError as error:
        raise SystemExit(f"Publication path escapes source through a symlink: {relative}") from error
    return source


def copy_entry(source_root: Path, destination: Path, relative: PurePosixPath, exclusions: list[PurePosixPath]) -> None:
    if is_excluded(relative, exclusions):
        return
    source = safe_source(source_root, relative)
    target = destination.joinpath(*relative.parts)
    if source.is_symlink():
        raise SystemExit(f"Publication entries cannot be symlinks: {relative}")
    if source.is_file():
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
        return
    for child in sorted(source.rglob("*")):
        child_relative = PurePosixPath(*child.relative_to(source_root).parts)
        if is_excluded(child_relative, exclusions):
            continue
        if child.is_symlink():
            raise SystemExit(f"Publication trees cannot contain symlinks: {child_relative}")
        if child.is_file():
            child_target = destination.joinpath(*child_relative.parts)
            child_target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(child, child_target)


def materialize_localization(source_root: Path, destination: Path, channel: str) -> None:
    manifest = source_root / "content" / "locales.toml"
    repository_content = source_root / "content" / "repo"
    shared = repository_content / "shared"
    channel_content = repository_content / channel
    renderer = Path(__file__).with_name("render-localization.py")
    localization_inputs = (manifest, shared, channel_content)
    if not any(path.exists() for path in localization_inputs):
        return
    if not manifest.is_file() or not shared.is_dir() or not channel_content.is_dir() or not renderer.is_file():
        raise SystemExit("Published repository content requires localization, shared/channel templates, and renderer")

    with manifest.open("rb") as handle:
        locale_data = tomllib.load(handle)
    locales = locale_data.get("locales")
    if not isinstance(locales, dict) or DEFAULT_LOCALE not in locales:
        raise SystemExit(f"{manifest}: invalid locale manifest")

    with tempfile.TemporaryDirectory() as temporary:
        temporary_root = Path(temporary)
        templates = temporary_root / "templates"
        output = temporary_root / "output"
        shutil.copytree(shared, templates)
        shutil.copytree(channel_content, templates, dirs_exist_ok=True)
        result = subprocess.run(
            [
                sys.executable,
                str(renderer),
                "--localization-root",
                str(manifest.parent),
                "--templates",
                str(templates),
                "--output",
                str(output),
            ],
            cwd=source_root,
            text=True,
            capture_output=True,
            check=False,
        )
        if result.returncode:
            raise SystemExit(result.stderr.strip() or "Localization rendering failed")

        default_root = output / DEFAULT_LOCALE
        rendered_readme = default_root / "README.md"
        if not rendered_readme.is_file():
            raise SystemExit(f"Localization output is missing {DEFAULT_LOCALE}/README.md")
        localized_docs = destination / "docs"
        localized_docs.mkdir(parents=True, exist_ok=True)
        shutil.copy2(rendered_readme, localized_docs / "README.md")

        for code, metadata in locales.items():
            if code == DEFAULT_LOCALE or not isinstance(metadata, dict) or metadata.get("published", True) is not True:
                continue
            rendered_locale_readme = output / code / "README.md"
            if rendered_locale_readme.is_file():
                shutil.copy2(rendered_locale_readme, localized_docs / f"README.{code}.md")


def main() -> None:
    args = parse_args()
    source_root = Path(args.source).resolve(strict=True)
    destination = Path(args.destination).resolve()
    config_path = Path(args.config).resolve(strict=True)
    if not SHA.fullmatch(args.source_commit):
        raise SystemExit("source-commit must be a full lowercase 40-character SHA")
    if not RELEASE_ID.fullmatch(args.version):
        raise SystemExit("version must match YYYY.MM.N-KIND using regular, hotfix, or security")
    try:
        destination.relative_to(source_root)
    except ValueError:
        pass
    else:
        if destination == source_root:
            raise SystemExit("destination cannot be the source root")

    include, channel_include, required, configured_exclude = load_config(config_path)
    exclusions = (
        [PurePosixPath(item) for item in sorted(HARD_DENY)]
        + sorted(HARD_DENY_PATHS, key=str)
        + configured_exclude
    )
    shutil.rmtree(destination, ignore_errors=True)
    destination.mkdir(parents=True)
    for relative in [*include, *channel_include[args.channel]]:
        copy_entry(source_root, destination, relative, exclusions)

    materialize_localization(source_root, destination, args.channel)

    for relative in required:
        if not destination.joinpath(*relative.parts).exists():
            raise SystemExit(f"Generated payload is missing required path: {relative}")

    generated_at = args.generated_at or datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    manifest_path = destination / ".github" / "publication.json"
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(
        json.dumps(
            {
                "channel": args.channel,
                "version": args.version,
                "sourceCommit": args.source_commit,
                "generatedAt": generated_at,
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(f"Publication payload built: {destination}")


if __name__ == "__main__":
    main()
