#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
import shutil
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path


TOKEN = re.compile(r"\{\{\s*l10n:([a-z][a-z0-9_-]*(?:\.[a-z][a-z0-9_]*)+)\s*\}\}")
DEFAULT_LOCALE = "en-US"
COMPONENT = re.compile(r"^[a-z][a-z0-9_-]*$")
SEGMENT = re.compile(r"^[a-z][a-z0-9_]*$")
LOCALE = re.compile(r"^[A-Za-z]{2,8}(?:-[A-Za-z0-9]{1,8})*$")
LEAF_FIELDS = {"values", "description", "meaning"}
TEXT_SUFFIXES = {".md", ".txt", ".html", ".yml", ".yaml", ".json", ".toml"}


@dataclass(frozen=True)
class Locale:
    code: str
    name: str
    published: bool
    fallback: tuple[str, ...]


@dataclass(frozen=True)
class Catalog:
    locales: dict[str, Locale]
    messages: dict[str, dict[str, str]]


def load_toml(path: Path) -> dict[str, object]:
    try:
        with path.open("rb") as handle:
            data = tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise SystemExit(f"{path}: {error}") from error
    if not isinstance(data, dict):
        raise SystemExit(f"{path}: expected a TOML table")
    return data


def load_locales(path: Path) -> dict[str, Locale]:
    data = load_toml(path)
    if set(data) != {"locales"}:
        raise SystemExit(f"{path}: expected only locales")
    locale_data = data["locales"]
    if not isinstance(locale_data, dict):
        raise SystemExit(f"{path}: locales must be a table")

    locales: dict[str, Locale] = {}
    for code, value in locale_data.items():
        if not isinstance(code, str) or not LOCALE.fullmatch(code):
            raise SystemExit(f"{path}: invalid locale identifier {code!r}")
        if not isinstance(value, dict) or set(value) - {"name", "published", "fallback"}:
            raise SystemExit(f"{path}: locale {code} contains unsupported fields")
        name = value.get("name", code)
        published = value.get("published", True)
        fallback = value.get("fallback", [])
        if not isinstance(name, str) or not name.strip():
            raise SystemExit(f"{path}: locale {code} requires a non-empty name")
        if not isinstance(published, bool):
            raise SystemExit(f"{path}: locale {code} published must be a boolean")
        if not isinstance(fallback, list) or not all(isinstance(item, str) for item in fallback):
            raise SystemExit(f"{path}: locale {code} fallback must be a string array")
        locales[code] = Locale(code, name, published, tuple(fallback))

    if DEFAULT_LOCALE not in locales:
        raise SystemExit(f"{path}: required default locale {DEFAULT_LOCALE!r} is not registered")
    for locale in locales.values():
        unknown = [code for code in locale.fallback if code not in locales]
        if unknown:
            raise SystemExit(f"{path}: locale {locale.code} references unknown fallback {unknown[0]!r}")
        if len(locale.fallback) != len(set(locale.fallback)):
            raise SystemExit(f"{path}: locale {locale.code} repeats a fallback")
    validate_fallback_cycles(path, locales)
    return locales


def validate_fallback_cycles(path: Path, locales: dict[str, Locale]) -> None:
    def visit(code: str, active: tuple[str, ...]) -> None:
        if code in active:
            cycle = " -> ".join((*active, code))
            raise SystemExit(f"{path}: locale fallback cycle: {cycle}")
        for fallback in locales[code].fallback:
            visit(fallback, (*active, code))

    for code in locales:
        visit(code, ())


def collect_messages(
    path: Path,
    component: str,
    table: dict[str, object],
    locales: dict[str, Locale],
    default_locale: str,
    prefix: tuple[str, ...] = (),
) -> dict[str, dict[str, str]]:
    messages: dict[str, dict[str, str]] = {}
    for segment, value in table.items():
        if not isinstance(segment, str) or not SEGMENT.fullmatch(segment):
            raise SystemExit(f"{path}: invalid key segment {segment!r}")
        if not isinstance(value, dict):
            raise SystemExit(f"{path}: {'.'.join((*prefix, segment))} must be a table")
        key_path = (*prefix, segment)
        if "values" in value:
            unsupported = set(value) - LEAF_FIELDS
            if unsupported:
                raise SystemExit(f"{path}: {'.'.join(key_path)} has unsupported leaf fields {sorted(unsupported)}")
            values = value["values"]
            if not isinstance(values, dict) or not values:
                raise SystemExit(f"{path}: {'.'.join(key_path)}.values must be a non-empty table")
            unknown = set(values) - set(locales)
            if unknown:
                raise SystemExit(f"{path}: {'.'.join(key_path)} uses unknown locale {sorted(unknown)[0]!r}")
            if default_locale not in values or not isinstance(values[default_locale], str) or not values[default_locale]:
                raise SystemExit(f"{path}: {'.'.join(key_path)} requires a non-empty {default_locale} value")
            if not all(isinstance(message, str) for message in values.values()):
                raise SystemExit(f"{path}: {'.'.join(key_path)} locale values must be strings")
            for metadata in ("description", "meaning"):
                if metadata in value and not isinstance(value[metadata], str):
                    raise SystemExit(f"{path}: {'.'.join(key_path)}.{metadata} must be a string")
            messages[".".join((component, *key_path))] = dict(values)
            continue
        messages.update(collect_messages(path, component, value, locales, default_locale, key_path))
    return messages


def load_catalog(localization_root: Path) -> Catalog:
    locales = load_locales(localization_root / "locales.toml")
    messages: dict[str, dict[str, str]] = {}
    components = [
        path
        for path in sorted(localization_root.rglob("*.toml"))
        if path != localization_root / "locales.toml"
    ]
    for path in components:
        component = path.stem
        if not COMPONENT.fullmatch(component):
            raise SystemExit(f"{path}: invalid component filename")
        component_messages = collect_messages(path, component, load_toml(path), locales, DEFAULT_LOCALE)
        duplicates = set(messages) & set(component_messages)
        if duplicates:
            raise SystemExit(f"{path}: duplicate effective key {sorted(duplicates)[0]}")
        messages.update(component_messages)
    return Catalog(locales, messages)


def fallback_chain(catalog: Catalog, locale: str) -> list[str]:
    chain: list[str] = []

    def append(code: str) -> None:
        if code in chain:
            return
        chain.append(code)
        for fallback in catalog.locales[code].fallback:
            append(fallback)

    append(locale)
    append(DEFAULT_LOCALE)
    return chain


def resolve(catalog: Catalog, key: str, locale: str) -> str:
    values = catalog.messages.get(key)
    if values is None:
        raise KeyError(key)
    for candidate in fallback_chain(catalog, locale):
        if candidate in values:
            return values[candidate]
    raise RuntimeError(f"{key} has no {DEFAULT_LOCALE} value")


def render_text(text: str, source: Path, catalog: Catalog, locale: str, used: set[str]) -> str:
    missing: set[str] = set()

    def replace(match: re.Match[str]) -> str:
        key = match.group(1)
        used.add(key)
        try:
            return resolve(catalog, key, locale)
        except KeyError:
            missing.add(key)
            return match.group(0)

    rendered = TOKEN.sub(replace, text)
    if missing:
        raise SystemExit(f"{source}: unknown localization key(s): {', '.join(sorted(missing))}")
    return rendered


def render_locale(templates: Path, output: Path, catalog: Catalog, locale: str) -> set[str]:
    destination_root = output / locale
    shutil.rmtree(destination_root, ignore_errors=True)
    used: set[str] = set()
    for source in sorted(templates.rglob("*")):
        if source.is_dir():
            continue
        # TOML files beside templates are authored translation catalogs, not
        # publication content.
        if source.suffix.lower() == ".toml":
            continue
        relative = source.relative_to(templates)
        if relative.name.endswith(".template.md"):
            relative = relative.with_name(relative.name.removesuffix(".template.md") + ".md")
        destination = destination_root / relative
        if destination.exists():
            raise SystemExit(f"{source}: template output collides with {destination}")
        destination.parent.mkdir(parents=True, exist_ok=True)
        if source.suffix.lower() in TEXT_SUFFIXES:
            rendered = render_text(source.read_text(encoding="utf-8"), source, catalog, locale, used)
            destination.write_text(rendered, encoding="utf-8", newline="\n")
        else:
            shutil.copy2(source, destination)
    return used


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Render component-oriented localized repository content.")
    parser.add_argument("--localization-root", default="content")
    parser.add_argument("--templates", default="content/pages")
    parser.add_argument("--output")
    parser.add_argument("--locale", action="append", default=[])
    parser.add_argument("--list-locales", action="store_true")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    catalog = load_catalog(Path(args.localization_root))
    if args.list_locales:
        ordered = [catalog.locales[DEFAULT_LOCALE], *(
            locale for code, locale in catalog.locales.items() if code != DEFAULT_LOCALE
        )]
        for locale in ordered:
            if locale.published:
                print(locale.code)
        return
    if not args.output:
        raise SystemExit("--output is required when rendering localized content")
    selected = args.locale or [DEFAULT_LOCALE, *(
        code for code, locale in catalog.locales.items()
        if code != DEFAULT_LOCALE and locale.published
    )]
    unknown = [code for code in selected if code not in catalog.locales]
    if unknown:
        raise SystemExit(f"Unknown locale {unknown[0]!r}")
    templates = Path(args.templates)
    if not templates.is_dir():
        raise SystemExit(f"Missing localization templates: {templates}")
    output = Path(args.output)
    if not args.locale:
        shutil.rmtree(output, ignore_errors=True)
    output.mkdir(parents=True, exist_ok=True)
    used: set[str] = set()
    for locale in selected:
        used.update(render_locale(templates, output, catalog, locale))
    referenced_components = {key.split(".", 1)[0] for key in used}
    unused = {
        key for key in set(catalog.messages) - used
        if key.split(".", 1)[0] in referenced_components
    }
    if unused:
        print(f"warning: unused localization key(s): {', '.join(sorted(unused))}", file=sys.stderr)
    print(f"Rendered localization output: {output}")


if __name__ == "__main__":
    main()
