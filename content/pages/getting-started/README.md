# {{ l10n:getting-started.title }}

{{ l10n:getting-started.introduction }}

{{ l10n:getting-started.global }}

```bash
scope global init
# or: scope global init --root /chosen/path/.scope
scope knowledge path global --plain
```

```bash
scope init
```

{{ l10n:getting-started.layout }}

```text
.scope/
├── scope.toml
├── knowledge.toml
├── knowledge/
│   ├── shared/
│   └── local/
├── policy/
│   ├── environment.toml
│   └── records.toml
├── checklists/
├── tests/
└── state/
```

{{ l10n:getting-started.state }}
