#!/usr/bin/env python3
from __future__ import annotations

import shutil
import subprocess
import sys
import tomllib
from pathlib import Path

import yaml


ROOT = Path(__file__).resolve().parents[2]
STAGING = ROOT / ".generated" / "docs-source"
DEFAULT_SITE = ROOT / ".generated" / "site"
LOCALE_SITES = ROOT / ".generated" / "locale-sites"
LOCALIZED = ROOT / ".generated" / "localization"
CONFIGURED = ROOT / ".generated" / "mkdocs-config.yml"
DEFAULT_LOCALE = "en-US"
SITE_BASE = "https://kooraseru.github.io/SCOPE/"


def stage_public_docs(locale: str) -> None:
    localized_pages = LOCALIZED / locale
    if not localized_pages.is_dir():
        raise SystemExit(f"Missing rendered Pages content for locale {locale}: {localized_pages}")
    shutil.rmtree(STAGING, ignore_errors=True)
    shutil.copytree(localized_pages, STAGING)
    for readme in sorted(STAGING.rglob("README.md")):
        readme.rename(readme.with_name("index.md"))


def configure_mkdocs(locale: str, locales: dict[str, object]) -> None:
    config_path = ROOT / ".github" / "mkdocs.yml"
    config = yaml.safe_load(config_path.read_text(encoding="utf-8"))
    if not isinstance(config, dict):
        raise SystemExit(f"{config_path}: expected a YAML object")
    config["docs_dir"] = "docs-source"
    config["site_dir"] = "site"
    config["site_url"] = SITE_BASE
    theme = config.setdefault("theme", {})
    theme["language"] = locale.split("-", 1)[0].lower()
    extra = config.setdefault("extra", {})
    extra["alternate"] = [
        {
            "name": metadata.get("name", code) if isinstance(metadata, dict) else code,
            "link": f"?language={code}",
            "lang": code,
        }
        for code, metadata in locales.items()
        if isinstance(metadata, dict) and metadata.get("published", True) is True
    ]
    CONFIGURED.parent.mkdir(parents=True, exist_ok=True)
    CONFIGURED.write_text(
        yaml.safe_dump(config, sort_keys=False, allow_unicode=True),
        encoding="utf-8",
        newline="\n",
    )


def main() -> None:
    renderer = [
        sys.executable,
        str(ROOT / ".github" / "scripts" / "render-localization.py"),
        "--templates",
        str(ROOT / "content" / "pages"),
    ]
    try:
        rendered = subprocess.run([*renderer, "--output", str(LOCALIZED)], cwd=ROOT, check=False)
        if rendered.returncode:
            raise SystemExit(rendered.returncode)

        with (ROOT / "content" / "locales.toml").open("rb") as handle:
            locale_data = tomllib.load(handle)
        locales = locale_data.get("locales")
        if not isinstance(locales, dict):
            raise SystemExit("content/locales.toml: locales must be a table")
        published = [
            code
            for code, metadata in locales.items()
            if isinstance(metadata, dict) and metadata.get("published", True) is True
        ]
        if DEFAULT_LOCALE not in published:
            raise SystemExit(f"content/locales.toml: required default locale {DEFAULT_LOCALE} is not published")
        published.remove(DEFAULT_LOCALE)
        published.insert(0, DEFAULT_LOCALE)

        shutil.rmtree(DEFAULT_SITE, ignore_errors=True)
        shutil.rmtree(LOCALE_SITES, ignore_errors=True)

        for locale in published:
            stage_public_docs(locale)
            configure_mkdocs(locale, locales)
            site_dir = LOCALE_SITES / locale
            command = [
                sys.executable,
                "-m",
                "mkdocs",
                "build",
                "--strict",
                "--config-file",
                str(CONFIGURED),
                "--site-dir",
                str(site_dir),
            ]
            result = subprocess.run(command, cwd=ROOT, check=False)
            if result.returncode:
                raise SystemExit(result.returncode)

        shutil.copytree(LOCALE_SITES / DEFAULT_LOCALE, DEFAULT_SITE)
        runtime_locales = DEFAULT_SITE / "_locales"
        for locale in published:
            shutil.copytree(LOCALE_SITES / locale, runtime_locales / locale)
    finally:
        shutil.rmtree(STAGING, ignore_errors=True)
        shutil.rmtree(LOCALIZED, ignore_errors=True)
        shutil.rmtree(LOCALE_SITES, ignore_errors=True)
        CONFIGURED.unlink(missing_ok=True)


if __name__ == "__main__":
    main()
