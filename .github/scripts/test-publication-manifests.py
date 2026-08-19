#!/usr/bin/env python3
from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("collect-publication-manifests.py")


class PublicationManifestTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def manifest(self, channel: str, **changes: object) -> Path:
        data: dict[str, object] = {
            "channel": channel,
            "version": "2026.01.1-regular",
            "sourceCommit": "a" * 40,
            "generatedAt": "2026-01-01T00:00:00Z",
        }
        data.update(changes)
        path = self.root / f"{channel}.json"
        path.write_text(json.dumps(data), encoding="utf-8")
        return path

    def run_collector(self, **manifests: Path) -> subprocess.CompletedProcess[str]:
        command = [sys.executable, str(SCRIPT), "--output", str(self.root / "output.json")]
        for channel, manifest in manifests.items():
            command.extend([f"--{channel}-manifest", str(manifest)])
        return subprocess.run(command, text=True, capture_output=True, check=False)

    def test_collects_all_channels(self) -> None:
        result = self.run_collector(
            canary=self.manifest("canary"),
            beta=self.manifest("beta"),
            stable=self.manifest("stable"),
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        output = json.loads((self.root / "output.json").read_text())
        self.assertEqual(output["stable"]["sourceCommit"], "a" * 40)
        self.assertEqual(output["canary"]["channel"], "canary")

    def test_missing_channels_are_optional(self) -> None:
        result = self.run_collector()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            json.loads((self.root / "output.json").read_text()),
            {"canary": None, "beta": None, "stable": None},
        )

    def test_channel_mismatch_fails(self) -> None:
        result = self.run_collector(stable=self.manifest("beta"))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("channel mismatch", result.stderr)

    def test_invalid_source_commit_fails(self) -> None:
        result = self.run_collector(stable=self.manifest("stable", sourceCommit="short"))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("full lowercase SHA", result.stderr)


if __name__ == "__main__":
    unittest.main()
