#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from datetime import datetime
from pathlib import Path
from typing import Any


SHA = re.compile(r"^[0-9a-f]{40}$")
VERSION = re.compile(r"^\d{4}\.(?:0[1-9]|1[0-2])\.[1-9]\d*-(?:regular|hotfix|security)$")
CHANNELS = ("canary", "beta", "stable")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Collect generated branch provenance for Pages.")
    for channel in CHANNELS:
        parser.add_argument(f"--{channel}-manifest")
    parser.add_argument("--output", required=True)
    parser.add_argument("--github-output")
    return parser.parse_args()


def read_manifest(path_value: str | None, expected_channel: str) -> dict[str, str] | None:
    if not path_value:
        return None
    path = Path(path_value)
    if not path.is_file():
        return None
    data: Any = json.loads(path.read_text(encoding="utf-8"))
    expected = {"channel", "version", "sourceCommit", "generatedAt"}
    if not isinstance(data, dict) or set(data) != expected:
        raise SystemExit(f"Invalid publication manifest shape: {path}")
    if data["channel"] != expected_channel:
        raise SystemExit(f"Publication channel mismatch: {path}")
    if not isinstance(data["sourceCommit"], str) or not SHA.fullmatch(data["sourceCommit"]):
        raise SystemExit(f"Publication sourceCommit must be a full lowercase SHA: {path}")
    if not isinstance(data["version"], str) or not VERSION.fullmatch(data["version"]):
        raise SystemExit(f"Publication version is invalid: {path}")
    if not isinstance(data["generatedAt"], str):
        raise SystemExit(f"Publication generatedAt must be a string: {path}")
    try:
        datetime.fromisoformat(data["generatedAt"].replace("Z", "+00:00"))
    except ValueError as error:
        raise SystemExit(f"Publication generatedAt is invalid: {path}") from error
    return {key: data[key] for key in sorted(expected)}


def append_outputs(path_value: str | None, manifests: dict[str, dict[str, str] | None]) -> None:
    if not path_value:
        return
    values = {}
    for channel, manifest in manifests.items():
        values[f"{channel}_source_commit"] = (manifest or {}).get("sourceCommit", "")
        values[f"{channel}_version"] = (manifest or {}).get("version", "")
    with Path(path_value).open("a", encoding="utf-8", newline="\n") as output:
        for key, value in values.items():
            output.write(f"{key}={value}\n")


def main() -> None:
    args = parse_args()
    manifests = {
        channel: read_manifest(getattr(args, f"{channel}_manifest"), channel)
        for channel in CHANNELS
    }
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(
        json.dumps(manifests, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    append_outputs(args.github_output, manifests)
    print(f"Publication manifests collected: {output_path}")


if __name__ == "__main__":
    main()
