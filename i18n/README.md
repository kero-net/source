# Internationalization

`i18n/` is the canonical translation store shared by repository generation and
GitHub Pages.

`locales.toml` contains only the ordered locale registry. The first entry is
the default and fallback locale:

```toml
locales = [
  { key = "en-US", language = "English" },
  { key = "ja-JP", language = "日本語" },
]
```

Translation catalogs are named after the first segment of their template key.
For example:

```text
{{ l10n:documentation.overview.title }}
```

resolves `overview.title` from `documentation.toml`:

```toml
[overview.title.values]
en-US = "KERO Documentation"
ja-JP = "KERO ドキュメント"
```

Keep Markdown, HTML, links, lists, and code blocks in templates. Catalog values
contain translated text only. Every translation key must define the first
locale from `locales.toml`. Other locales fall back according to registry order.

The current catalogs are:

- `documentation.toml` for documentation landing content.
- `getting-started.toml` for the getting-started documentation.
- `repository.toml` for public repository README content.
- `releases.toml` for localized release terminology.

Run `lua5.4 i18n/validate.lua` to validate registry and catalog structure.
