# Translating Content

This guide explains how contributors translate repository and Pages content.
The current locale registry lives only in `content/locales.toml`; this guide
does not duplicate its entries.

## Model

Localizable templates contain keys at every translated fill-in area. Language
text belongs in the adjacent component catalog, including the default English
value.

```md
## {{ l10n:repository.sections.features.title }}
```

`en-US` is the fixed default. It must be registered in `content/locales.toml`
and every translation leaf must define it. The locale registry does not contain
a schema version or configurable source-locale field.

Translations live in component TOML catalogs beside the content they affect.
The catalog filename supplies the first key segment:

```text
content/repo/shared/README.template.md
content/repo/shared/repository.toml
```

```toml
[sections.features.title.values]
en-US = "Features"
ja-JP = "機能"
de-DE = "Funktionen"
```

The effective key is `repository.sections.features.title`. Lists and larger
Markdown or HTML regions can also be a single keyed value when translating the
region together preserves its structure and meaning.

## Adding A Translation

1. Confirm the locale ID and fallback order in `content/locales.toml`.
2. Find the template key for the content being translated.
3. Open or create the component TOML beside that template.
4. Add the translation at the existing hierarchical key without replacing the
   key in the template.
5. Preserve Markdown, HTML, links, interpolation parameters, and intended
   meaning.
6. Run the localization tests and the build that consumes the translated
   content.

Missing translations follow the locale's configured fallback order and
ultimately use `en-US`. Unknown keys and missing English values fail rendering.

Core control files—including `.github/README.md`, `CONTRIBUTING.md`, and
`LICENSE`—remain English-only and do not use localization markers.

Generated publication branches retain only `content/assets/` from the authored
`content/` tree. The English repository output becomes `docs/README.md`;
additional published translations use `docs/README.<locale>.md`. This generated
`docs/` directory contains repository README translations, not the authored
source documentation tree. Pages is structured exclusively by `content/pages/`.
The build compiles each published locale as a runtime content source while
serving one stable URL. Material for MkDocs displays the language selector;
JavaScript swaps the current page in place and stores the selection in a cookie.
