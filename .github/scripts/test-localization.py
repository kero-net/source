#!/usr/bin/env python3
from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("render-localization.py")


class LocalizationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.localization = self.root / "content"
        self.templates = self.localization / "pages"
        self.output = self.root / "output"
        self.templates.mkdir(parents=True)
        (self.localization / "locales.toml").write_text(
            '[locales."en-US"]\nname = "English"\n\n'
            '[locales."ja-JP"]\nname = "日本語"\nfallback = ["en-US"]\n',
            encoding="utf-8",
        )
        (self.templates / "ui.toml").write_text(
            '[buttons.save]\ndescription = "Save button"\n\n'
            '[buttons.save.values]\nen-US = "Save"\nja-JP = "保存"\n\n'
            '[buttons.cancel.values]\nen-US = "Cancel"\n',
            encoding="utf-8",
        )
        (self.templates / "page.md").write_text(
            "# {{ l10n:ui.buttons.save }}\n\n{{ l10n:ui.buttons.cancel }}\n",
            encoding="utf-8",
        )

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def run_renderer(self) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--localization-root",
                str(self.localization),
                "--templates",
                str(self.templates),
                "--output",
                str(self.output),
            ],
            text=True,
            capture_output=True,
            check=False,
        )

    def test_renders_all_locales_and_falls_back_to_english(self) -> None:
        result = self.run_renderer()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.output / "en-US" / "page.md").read_text(), "# Save\n\nCancel\n")
        self.assertEqual((self.output / "ja-JP" / "page.md").read_text(), "# 保存\n\nCancel\n")
        self.assertFalse((self.output / "en-US" / "ui.toml").exists())

    def test_unknown_template_key_fails(self) -> None:
        (self.templates / "page.md").write_text("{{ l10n:ui.missing }}\n", encoding="utf-8")
        result = self.run_renderer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unknown localization key", result.stderr)

    def test_template_markdown_drops_template_segment(self) -> None:
        (self.templates / "README.template.md").write_text("# Template\n", encoding="utf-8")
        result = self.run_renderer()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.output / "en-US" / "README.md").read_text(), "# Template\n")
        self.assertFalse((self.output / "en-US" / "README.template.md").exists())

    def test_template_output_collision_fails(self) -> None:
        (self.templates / "README.md").write_text("# Directory\n", encoding="utf-8")
        (self.templates / "README.template.md").write_text("# Template\n", encoding="utf-8")
        result = self.run_renderer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("template output collides", result.stderr)

    def test_catalog_requires_default_english_value(self) -> None:
        (self.templates / "ui.toml").write_text(
            '[buttons.save.values]\nja-JP = "保存"\n', encoding="utf-8"
        )
        result = self.run_renderer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("requires a non-empty en-US value", result.stderr)

    def test_unknown_fallback_fails(self) -> None:
        path = self.localization / "locales.toml"
        path.write_text(path.read_text().replace('fallback = ["en-US"]', 'fallback = ["fr-FR"]'), encoding="utf-8")
        result = self.run_renderer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unknown fallback", result.stderr)


if __name__ == "__main__":
    unittest.main()
