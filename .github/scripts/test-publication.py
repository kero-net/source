#!/usr/bin/env python3
from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("build-publication.py")
SHA = "a" * 40


class PublicationBuilderTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.source = self.root / "source"
        self.output = self.root / "output"
        self.source.mkdir()
        (self.source / "public").mkdir()
        (self.source / "public" / "artifact.txt").write_text("artifact\n", encoding="utf-8")
        (self.source / "docs").mkdir()
        (self.source / "docs" / "public.md").write_text("public\n", encoding="utf-8")
        (self.source / ".agents" / "docs").mkdir(parents=True)
        (self.source / ".agents" / "docs" / "private.md").write_text("private\n", encoding="utf-8")
        (self.source / ".vscode").mkdir()
        (self.source / ".vscode" / "launch.json").write_text("{}\n", encoding="utf-8")
        (self.source / ".vscode" / "settings.json").write_text("{}\n", encoding="utf-8")
        (self.source / "LICENSE").write_text("Test license\n", encoding="utf-8")
        self.config = self.root / "config.yml"

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def write_config(self, text: str) -> None:
        self.config.write_text(text, encoding="utf-8")

    def run_builder(self, channel: str = "stable") -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--source",
                str(self.source),
                "--destination",
                str(self.output),
                "--config",
                str(self.config),
                "--channel",
                channel,
                "--version",
                "2026.01.1-regular",
                "--source-commit",
                SHA,
                "--generated-at",
                "2026-01-01T00:00:00Z",
            ],
            text=True,
            capture_output=True,
            check=False,
        )

    def test_builds_whitelisted_payload_and_manifest(self) -> None:
        self.write_config(
            "enabled: true\ninclude: [public]\nchannel_include: {canary: [], beta: [], stable: []}\n"
            "required: [public/artifact.txt]\nexclude: []\n"
        )
        result = self.run_builder()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.output / "public" / "artifact.txt").read_text(), "artifact\n")
        self.assertFalse((self.output / "docs").exists())
        manifest = json.loads((self.output / ".github" / "publication.json").read_text())
        self.assertEqual(manifest["sourceCommit"], SHA)
        self.assertEqual(manifest["channel"], "stable")

    def test_disabled_publication_fails_closed(self) -> None:
        self.write_config("enabled: false\ninclude: []\nchannel_include: {canary: [], beta: [], stable: []}\nrequired: []\nexclude: []\n")
        result = self.run_builder()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("disabled", result.stderr)

    def test_private_and_wrong_generated_roots_cannot_be_included(self) -> None:
        for path in (".agents", ".workspace", "site"):
            with self.subTest(path=path):
                self.write_config(f"enabled: true\ninclude: [{path}]\nchannel_include: {{canary: [], beta: [], stable: []}}\nrequired: [{path}]\nexclude: []\n")
                result = self.run_builder()
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("cannot be published", result.stderr)

    def test_public_docs_can_be_included(self) -> None:
        self.write_config("enabled: true\ninclude: [docs]\nchannel_include: {canary: [], beta: [], stable: []}\nrequired: [docs/public.md]\nexclude: []\n")
        result = self.run_builder()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.output / "docs" / "public.md").read_text(), "public\n")

    def test_publication_materializes_english_and_japanese_readmes(self) -> None:
        (self.source / "README.md").write_text("source build readme\n", encoding="utf-8")
        (self.source / "content").mkdir()
        (self.source / "content" / "locales.toml").write_text(
            '[locales."en-US"]\nname = "English"\n\n'
            '[locales."ja-JP"]\nname = "日本語"\nfallback = ["en-US"]\n',
            encoding="utf-8",
        )
        (self.source / "content" / "repo" / "shared").mkdir(parents=True)
        (self.source / "content" / "repo" / "stable").mkdir(parents=True)
        (self.source / "content" / "repo" / "shared" / "repository.toml").write_text(
            '[title.values]\nen-US = "Published README"\nja-JP = "公開 README"\n',
            encoding="utf-8",
        )
        (self.source / "content" / "repo" / "shared" / "README.template.md").write_text(
            "# {{ l10n:repository.title }}\n",
            encoding="utf-8",
        )
        self.write_config("enabled: true\ninclude: [public]\nchannel_include: {canary: [], beta: [], stable: []}\nrequired: [docs/README.md]\nexclude: []\n")
        result = self.run_builder()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.output / "README.md").exists())
        self.assertEqual((self.output / "docs" / "README.md").read_text(), "# Published README\n")
        self.assertEqual((self.output / "docs" / "README.ja-JP.md").read_text(), "# 公開 README\n")

    def test_shared_vscode_files_publish_without_private_settings(self) -> None:
        self.write_config("enabled: true\ninclude: [.vscode]\nchannel_include: {canary: [], beta: [], stable: []}\nrequired: [.vscode/launch.json]\nexclude: []\n")
        result = self.run_builder()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.output / ".vscode" / "launch.json").is_file())
        self.assertFalse((self.output / ".vscode" / "settings.json").exists())

    def test_channel_include_adds_default_branch_control_plane_only_when_selected(self) -> None:
        (self.source / ".github").mkdir()
        (self.source / ".github" / "FUNDING.yml").write_text("github: [example]\n", encoding="utf-8")
        self.write_config(
            "enabled: true\ninclude: [public]\n"
            "channel_include: {canary: [], beta: [], stable: [.github/FUNDING.yml]}\n"
            "required: [public/artifact.txt]\nexclude: []\n"
        )
        result = self.run_builder()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.output / ".github" / "FUNDING.yml").is_file())
        result = self.run_builder("canary")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.output / ".github" / "FUNDING.yml").exists())

    def test_private_vscode_settings_cannot_be_required(self) -> None:
        self.write_config(
            "enabled: true\n"
            "include: [.vscode]\n"
            "channel_include: {canary: [], beta: [], stable: []}\n"
            "required: [.vscode/settings.json]\n"
            "exclude: []\n"
        )
        result = self.run_builder()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("cannot be published", result.stderr)

    def test_parent_traversal_is_rejected(self) -> None:
        self.write_config("enabled: true\ninclude: [../outside]\nchannel_include: {canary: [], beta: [], stable: []}\nrequired: [public/artifact.txt]\nexclude: []\n")
        result = self.run_builder()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Unsafe include path", result.stderr)

    def test_missing_required_path_fails(self) -> None:
        self.write_config("enabled: true\ninclude: [public]\nchannel_include: {canary: [], beta: [], stable: []}\nrequired: [missing.txt]\nexclude: []\n")
        result = self.run_builder()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("missing required path", result.stderr)


if __name__ == "__main__":
    unittest.main()
