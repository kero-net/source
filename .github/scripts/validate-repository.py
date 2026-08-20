#!/usr/bin/env python3
from __future__ import annotations

import ast
import re
import sys
import tomllib
from pathlib import Path

import yaml


ROOT = Path(__file__).resolve().parents[2]
GITHUB = ROOT / ".github"
FORBIDDEN = re.compile(r"Observer License", re.IGNORECASE)
FULL_SHA = re.compile(r"^[0-9a-f]{40}$")
RELEASE_RECORD = re.compile(
    r"^(\d{4}\.(?:0[1-9]|1[0-2])\.[1-9]\d*-(?:regular|hotfix|security))\.release\.md$"
)
RELEASE_SECTIONS = (
    "Summary",
    "Notable Changes",
    "Issues Addressed",
    "Compatibility",
    "Verification",
)
PUBLICATION_COMMON = {
    ".gitattributes",
    ".gitignore",
    ".vscode",
    "CODE_OF_CONDUCT.md",
    "CITATION.cff",
    "CONTRIBUTING.md",
    "LICENSE",
    "SECURITY.md",
    "content/assets",
    "src",
}
STABLE_CONTROL_PLANE = {
    ".github/DISCUSSION_TEMPLATE",
    ".github/ISSUE_TEMPLATE",
    ".github/PULL_REQUEST_TEMPLATE",
    ".github/FUNDING.yml",
    ".github/SUPPORT.md",
    ".github/dependabot.yml",
    ".github/labeler.yml",
    ".github/release.yml",
    ".github/workflows/codeql.yml",
    ".github/workflows/greetings.yml",
    ".github/workflows/labeler.yml",
    ".github/workflows/pages.yml",
    ".github/workflows/publish.yml",
    ".github/workflows/stale.yml",
    ".github/workflows/validate.yml",
    ".github/workflows/visit-counter.yml",
}


def validate_python() -> list[str]:
    errors: list[str] = []
    for path in sorted((GITHUB / "scripts").glob("*.py")):
        try:
            ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
        except (SyntaxError, UnicodeError) as error:
            errors.append(f"{path.relative_to(ROOT)}: {error}")
    return errors


def validate_yaml() -> list[str]:
    errors: list[str] = []
    for path in sorted(GITHUB.rglob("*.yml")):
        try:
            yaml.safe_load(path.read_text(encoding="utf-8"))
        except yaml.YAMLError as error:
            errors.append(f"{path.relative_to(ROOT)}: {error}")
    return errors


def yaml_data(path: Path) -> object:
    # YAML 1.1 treats the key `on` as a boolean. GitHub uses YAML 1.2 semantics,
    # but the distinction does not affect the structural checks below.
    return yaml.safe_load(path.read_text(encoding="utf-8"))


def validate_issue_forms() -> list[str]:
    errors: list[str] = []
    for path in sorted((GITHUB / "ISSUE_TEMPLATE").glob("*.yml")):
        data = yaml_data(path)
        if path.name == "config.yml":
            if not isinstance(data, dict) or "blank_issues_enabled" not in data:
                errors.append(f"{path.relative_to(ROOT)}: missing blank_issues_enabled")
            continue
        if not isinstance(data, dict):
            errors.append(f"{path.relative_to(ROOT)}: form must be an object")
            continue
        for key in ("name", "description", "body"):
            if not data.get(key):
                errors.append(f"{path.relative_to(ROOT)}: missing {key}")
        body = data.get("body", [])
        if not isinstance(body, list):
            errors.append(f"{path.relative_to(ROOT)}: body must be a list")
            continue
        ids = [item.get("id") for item in body if isinstance(item, dict) and item.get("id")]
        if len(ids) != len(set(ids)):
            errors.append(f"{path.relative_to(ROOT)}: form ids must be unique")
    return errors


def validate_workflow_security() -> list[str]:
    errors: list[str] = []
    for path in sorted((GITHUB / "workflows").glob("*.yml")):
        data = yaml_data(path)
        if not isinstance(data, dict):
            errors.append(f"{path.relative_to(ROOT)}: workflow must be an object")
            continue
        if "permissions" not in data:
            errors.append(f"{path.relative_to(ROOT)}: missing top-level permissions")
        text = path.read_text(encoding="utf-8")
        for action, ref in re.findall(r"^\s*uses:\s*([^\s@]+)@([^\s#]+)", text, re.MULTILINE):
            if action.startswith("./"):
                continue
            if not FULL_SHA.fullmatch(ref):
                errors.append(f"{path.relative_to(ROOT)}: action is not pinned to a full SHA: {action}@{ref}")
        if "pull_request_target:" in text:
            if "actions/checkout@" in text or re.search(r"^\s*(?:run|shell):", text, re.MULTILINE):
                errors.append(f"{path.relative_to(ROOT)}: pull_request_target workflow executes repository code")
        if re.search(r"persist-credentials:\s*true", text):
            errors.append(f"{path.relative_to(ROOT)}: checkout credentials must not persist")
    return errors


def validate_repository_boundaries() -> list[str]:
    errors: list[str] = []
    required = [
        GITHUB / "README.md",
        ROOT / "src" / ".gitkeep",
        GITHUB / "TRANSLATORS.md",
        ROOT / "CODE_OF_CONDUCT.md",
        ROOT / "CITATION.cff",
        ROOT / "CONTRIBUTING.md",
        ROOT / "LICENSE",
        ROOT / "SECURITY.md",
        ROOT / "content" / "locales.toml",
        ROOT / "content" / "releases" / "README.md",
        ROOT / "content" / "releases" / "2026.08.1-regular.release.md",
        ROOT / "content" / "pages" / "README.md",
        ROOT / "content" / "pages" / "documentation.toml",
        ROOT / "content" / "assets" / "branding" / ".gitkeep",
        ROOT / "content" / "assets" / "icons" / ".gitkeep",
        ROOT / "content" / "assets" / "images" / ".gitkeep",
        ROOT / "content" / "assets" / "diagrams" / ".gitkeep",
        ROOT / "content" / "assets" / "screenshots" / ".gitkeep",
        ROOT / "content" / "assets" / "media" / ".gitkeep",
        ROOT / "content" / "repo" / "shared" / "README.template.md",
        ROOT / "content" / "repo" / "shared" / "readme.repository.toml",
        ROOT / "content" / "repo" / "canary" / ".gitkeep",
        ROOT / "content" / "repo" / "beta" / ".gitkeep",
        ROOT / "content" / "repo" / "stable" / ".gitkeep",
        ROOT / "docs" / "README.md",
        ROOT / ".scope" / "scope.toml",
        ROOT / ".scope" / "knowledge.toml",
        ROOT / ".scope" / ".gitignore",
        ROOT / ".scope" / "knowledge" / "shared" / "architecture" / "README.md",
        ROOT / ".scope" / "knowledge" / "shared" / "architecture" / "repository.md",
        ROOT / "src" / "tests" / "fixtures" / "policy" / "environment.toml",
        ROOT / ".vscode" / "launch.json",
        ROOT / ".vscode" / "tasks.json",
        ROOT / ".vscode" / "extensions.json",
        ROOT / ".vscode" / "project.code-snippets",
        ROOT / ".vscode" / "mcp.json",
        GITHUB / "mkdocs.yml",
        GITHUB / "scripts" / "render-localization.py",
        GITHUB / "scripts" / "test-localization.py",
    ]
    for path in required:
        if not path.is_file():
            errors.append(f"missing public repository boundary: {path.relative_to(ROOT)}")
    ignore_lines = {
        line.strip()
        for line in (ROOT / ".gitignore").read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    }
    for pattern in (".generated/", "AGENTS.md", ".agents/", ".vscode/settings.json"):
        if pattern not in ignore_lines:
            errors.append(f".gitignore must ignore private/generated surface: {pattern}")
    if ".vscode/" in ignore_lines:
        errors.append("shared .vscode/ launch and task templates must not be ignored")

    extensions = yaml_data(ROOT / ".vscode" / "extensions.json")
    if not isinstance(extensions, dict) or not isinstance(extensions.get("recommendations"), list):
        errors.append(".vscode/extensions.json must define extension recommendations")
    mcp = yaml_data(ROOT / ".vscode" / "mcp.json")
    if not isinstance(mcp, dict) or not isinstance(mcp.get("servers"), dict):
        errors.append(".vscode/mcp.json must define a servers object")
    snippets = yaml_data(ROOT / ".vscode" / "project.code-snippets")
    if not isinstance(snippets, dict):
        errors.append(".vscode/project.code-snippets must be an object")
    if "docs/" in ignore_lines:
        errors.append("public docs/ must not be ignored")
    if (ROOT / ".docs").exists():
        errors.append("legacy .docs/ is forbidden; SCOPE-owned knowledge belongs under .scope/knowledge/")
    scope_manifest = tomllib.loads((ROOT / ".scope" / "scope.toml").read_text(encoding="utf-8"))
    if scope_manifest.get("kind") != "project" or scope_manifest.get("id") != "scope.project":
        errors.append(".scope/scope.toml must declare the project discovery identity")
    scope_paths = scope_manifest.get("paths", {})
    if scope_paths.get("knowledge") != "knowledge.toml":
        errors.append(".scope/scope.toml must locate knowledge.toml inside .scope/")
    knowledge = tomllib.loads((ROOT / ".scope" / "knowledge.toml").read_text(encoding="utf-8"))
    project_knowledge = knowledge.get("project", {})
    if project_knowledge.get("shared_root") != "knowledge/shared":
        errors.append("repository-shared knowledge must live under .scope/knowledge/shared/")
    if project_knowledge.get("local_root") != "knowledge/local":
        errors.append("repository-local knowledge must live under .scope/knowledge/local/")
    for legacy_test_root in (ROOT / "tests", ROOT / "examples"):
        if legacy_test_root.exists():
            errors.append(
                f"{legacy_test_root.relative_to(ROOT)}/: product tests and fixtures must stay under src/tests/"
            )
    mkdocs = yaml_data(GITHUB / "mkdocs.yml")
    if not isinstance(mkdocs, dict):
        errors.append(".github/mkdocs.yml must be an object")
    else:
        if mkdocs.get("docs_dir") != "../.generated/docs-source":
            errors.append("MkDocs must read the generated public staging tree")
        if mkdocs.get("site_dir") != "../.generated/site":
            errors.append("MkDocs site output must be .generated/site")
        if mkdocs.get("strict") is not True:
            errors.append("MkDocs strict mode must remain enabled")
    return errors


def validate_publication_layout() -> list[str]:
    errors: list[str] = []
    path = GITHUB / "publication" / "config.yml"
    data = yaml_data(path)
    if not isinstance(data, dict):
        return [f"{path.relative_to(ROOT)}: publication config must be an object"]
    include = data.get("include")
    channel_include = data.get("channel_include")
    if not isinstance(include, list) or set(include) != PUBLICATION_COMMON:
        errors.append(
            ".github/publication/config.yml: include must match the common publication payload"
        )
    if not isinstance(channel_include, dict):
        errors.append(".github/publication/config.yml: channel_include must be an object")
        return errors
    if channel_include.get("canary") != [] or channel_include.get("beta") != []:
        errors.append(".github/publication/config.yml: preview channels must not publish authored .github files")
    stable = channel_include.get("stable")
    if not isinstance(stable, list) or set(stable) != STABLE_CONTROL_PLANE:
        errors.append(
            ".github/publication/config.yml: stable must match the default-branch control plane"
        )
    for relative in STABLE_CONTROL_PLANE:
        if not (ROOT / relative).exists():
            errors.append(f"missing stable control-plane source: {relative}")
    return errors


def validate_citation() -> list[str]:
    path = ROOT / "CITATION.cff"
    data = yaml_data(path)
    if not isinstance(data, dict):
        return ["CITATION.cff: citation metadata must be an object"]
    required = {"cff-version", "message", "title", "type", "authors", "repository-code", "url"}
    if set(data) != required:
        return ["CITATION.cff: citation metadata fields do not match the repository contract"]
    errors: list[str] = []
    if data.get("cff-version") != "1.2.0":
        errors.append("CITATION.cff: cff-version must be 1.2.0")
    if data.get("type") != "software":
        errors.append("CITATION.cff: type must be software")
    authors = data.get("authors")
    if not isinstance(authors, list) or not authors or not all(isinstance(author, dict) for author in authors):
        errors.append("CITATION.cff: authors must be a non-empty list of objects")
    for field in ("message", "title", "repository-code", "url"):
        if not isinstance(data.get(field), str) or not data[field].strip():
            errors.append(f"CITATION.cff: {field} must be a non-empty string")
    return errors


def validate_release_records() -> list[str]:
    errors: list[str] = []
    releases = ROOT / "content" / "releases"
    if not releases.is_dir():
        return errors
    for path in sorted(releases.iterdir()):
        if path.name == "README.md":
            continue
        if not path.is_file():
            errors.append(f"{path.relative_to(ROOT)}: release entries must be files")
            continue
        match = RELEASE_RECORD.fullmatch(path.name)
        if not match:
            errors.append(
                f"{path.relative_to(ROOT)}: release records must use YYYY.MM.N-KIND.release.md"
            )
            continue
        release_id = match.group(1)
        text = path.read_text(encoding="utf-8")
        lines = text.splitlines()
        if not lines or lines[0] != f"# {release_id}":
            errors.append(f"{path.relative_to(ROOT)}: first heading must be # {release_id}")
        headings = list(re.finditer(r"^## ([^\n]+)\n", text, re.MULTILINE))
        names = [heading.group(1) for heading in headings]
        if names != list(RELEASE_SECTIONS):
            errors.append(
                f"{path.relative_to(ROOT)}: required sections are {', '.join(RELEASE_SECTIONS)} in order"
            )
            continue
        for index, heading in enumerate(headings):
            end = headings[index + 1].start() if index + 1 < len(headings) else len(text)
            if not text[heading.end():end].strip():
                errors.append(f"{path.relative_to(ROOT)}: {heading.group(1)} must not be empty")

    publish = yaml_data(GITHUB / "workflows" / "publish.yml")
    if isinstance(publish, dict):
        dispatch = publish.get(True, {}).get("workflow_dispatch", {})
        version = dispatch.get("inputs", {}).get("version", {}) if isinstance(dispatch, dict) else {}
        options = version.get("options") if isinstance(version, dict) else None
        if not isinstance(options, list) or not options or not all(isinstance(item, str) for item in options):
            errors.append(".github/workflows/publish.yml: version must define non-empty choice options")
        else:
            expected = {
                path.name.removesuffix(".release.md")
                for path in releases.glob("*.release.md")
            }
            if set(options) != expected or len(options) != len(expected):
                errors.append(
                    ".github/workflows/publish.yml: version choices must exactly match content/releases/*.release.md"
                )
    return errors


def validate_local_links() -> list[str]:
    errors: list[str] = []
    link_pattern = re.compile(r"\[[^]]*\]\((?!https?://|mailto:|#)([^)#]+)(?:#[^)]+)?\)")
    paths = [GITHUB / "README.md", GITHUB / "TRANSLATORS.md", ROOT / "CONTRIBUTING.md"]
    paths.extend(sorted((ROOT / "docs").rglob("*.md")))
    paths.extend(sorted((ROOT / ".scope" / "knowledge" / "shared").rglob("*.md")))
    paths.extend(sorted(GITHUB.rglob("*.md")))
    for path in paths:
        if not path.is_file():
            errors.append(f"missing public Markdown source: {path.relative_to(ROOT)}")
            continue
        text = path.read_text(encoding="utf-8")
        for target in link_pattern.findall(text):
            if not (path.parent / target).resolve().exists():
                errors.append(f"{path.relative_to(ROOT)}: missing link target {target}")
    return errors


def validate_residue() -> list[str]:
    errors: list[str] = []
    paths = [GITHUB / "README.md", ROOT / "CONTRIBUTING.md"]
    paths.extend(sorted((ROOT / "docs").rglob("*")))
    paths.extend(sorted((ROOT / ".scope" / "knowledge" / "shared").rglob("*")))
    paths.extend(sorted(GITHUB.rglob("*")))
    for path in paths:
        if not path.is_file() or path.name == "validate-repository.py":
            continue
        if FORBIDDEN.search(path.read_text(encoding="utf-8", errors="replace")):
            errors.append(f"{path.relative_to(ROOT)}: contains prior-project residue")
    return errors


def main() -> None:
    errors = (
        validate_python()
        + validate_yaml()
        + validate_issue_forms()
        + validate_workflow_security()
        + validate_repository_boundaries()
        + validate_publication_layout()
        + validate_citation()
        + validate_release_records()
        + validate_local_links()
        + validate_residue()
    )
    if errors:
        print("Repository validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        raise SystemExit(1)
    print("GitHub repository contracts OK")


if __name__ == "__main__":
    main()
