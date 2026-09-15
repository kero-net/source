# {{ l10n:getting-started.title }}

{{ l10n:getting-started.introduction }}

{{ l10n:getting-started.current_state }}

```bash
git clone https://github.com/kero-net/kero.git
cd kero/code
cargo test --all-targets --all-features --locked
cargo run -p kero-cli -- --help
```

{{ l10n:getting-started.project_model }}

```text
repository/
├── .git/
├── .kero/        project declaration and KERO-managed state
├── docs/         possible mounted knowledge source
└── src/
```

{{ l10n:getting-started.future_flow }}

```text
kero init
  → review discovered source suggestions
  → attach knowledge mounts
  → KERO maintains compiled knowledge
  → inspect status, query, trace, and project context
```
