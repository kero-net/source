# {{ l10n:getting-started.title }}

{{ l10n:getting-started.introduction }}

{{ l10n:getting-started.global }}

```bash
kero global init
# or: kero global init --root /chosen/path/.kero
kero knowledge path global --plain
```

```bash
kero project init
```

{{ l10n:getting-started.layout }}

```text
.kero/
├── kero.toml
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
