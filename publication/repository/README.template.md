<div align="center">
  <h1>KERO</h1>
  <h3>{{ l10n:repository.hero.tagline }}</h3>
  <p><a href="https://github.com/kero-net/kero"><img alt="Stars + Issues + License" src="https://shieldcn.dev/group/github/stars/kero-net/kero+github/kero-net/kero/issues+github/license/kero-net/kero.svg?variant=outline"></a></p>
  <table><tr><td><a href="#features">{{ l10n:repository.navigation.features }}</a></td><td><a href="#quick-start">{{ l10n:repository.navigation.quick_start }}</a></td><td><a href="#policy-model">{{ l10n:repository.navigation.policy_model }}</a></td><td><a href="#status">{{ l10n:repository.navigation.status }}</a></td><td><a href="#development">{{ l10n:repository.navigation.development }}</a></td><td><a href="#project">{{ l10n:repository.navigation.project }}</a></td></tr></table>
  <table><tr><td><a href="README.md">English</a></td><td><a href="README.ja-JP.md">日本語</a></td></tr></table>
</div>

<h2 id="features">{{ l10n:repository.features.heading }}</h2>

- {{ l10n:repository.features.independent_results }}
- {{ l10n:repository.features.immutable_artifacts }}
- {{ l10n:repository.features.linux_boundaries }}
- {{ l10n:repository.features.rust_backend }}

<h2 id="quick-start">{{ l10n:repository.quick_start.heading }}</h2>

### {{ l10n:repository.quick_start.build_cli.heading }}

{{ l10n:repository.quick_start.build_cli.platform_support }}

{{ l10n:repository.quick_start.build_cli.prerequisites_intro }}

#### Ubuntu/Debian

```bash
sudo apt update
sudo apt install build-essential curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### Arch

```bash
sudo pacman -S --needed base-devel rust
```

{{ l10n:repository.quick_start.build_cli.build_intro }}

```bash
cd code
cargo build --release --locked
/usr/bin/install -Dm755 target/release/kero "$HOME/.local/bin/kero"
kero --version
```

{{ l10n:repository.quick_start.build_cli.path_note }}

### {{ l10n:repository.quick_start.global_environment.heading }}

{{ l10n:repository.quick_start.global_environment.intro }}

{{ l10n:repository.quick_start.global_environment.default_intro }}

```bash
kero init
kero global path
kero knowledge path global --plain
```

{{ l10n:repository.quick_start.global_environment.custom_intro }}

```bash
kero init --root /path/to/global/.kero
kero global path
kero knowledge path global --plain
```

{{ l10n:repository.quick_start.global_environment.storage_note }}

### {{ l10n:repository.quick_start.initialize_repository.heading }}

```bash
cd /path/to/repository
kero project init
```

{{ l10n:repository.quick_start.initialize_repository.layout_intro }}

```text
.kero/
├── .gitignore
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

### {{ l10n:repository.quick_start.minimal_policy.heading }}

{{ l10n:repository.quick_start.minimal_policy.records_intro }}

```toml
schema = "terminal-policy/records/v1"
source = "source.project"
[[principals]]
id = "principal.local-user"
[[roles]]
id = "role.contributor"
[[statements]]
id = "statement.allow-source-write"
role = "role.contributor"
effect = "allow"
operations = ["filesystem.write"]
keros = ["path:src/**"]
[[assignments]]
id = "assignment.local-user.contributor"
principal = "principal.local-user"
roles = ["role.contributor"]
operations = ["filesystem.write"]
keros = ["path:src/**"]
```

{{ l10n:repository.quick_start.minimal_policy.request_intro }}

```toml
schema = "terminal-policy/request/v1"
request_id = "request.write-source"
principal = "principal.local-user"
operation = "filesystem.write"
targets = ["path:src/main.rs"]
[session]
id = "session.local"
client = "kero-cli"
```

{{ l10n:repository.quick_start.minimal_policy.authorize_intro }}

```bash
kero policy authorize --request .kero/tests/write-source.toml
```

{{ l10n:repository.quick_start.minimal_policy.replay_intro }}

```bash
kero policy replay --snapshot .kero/state/policy-snapshots/SNAPSHOT.json --digest sha256:DIGEST --request .kero/tests/write-source.toml
```

<h2 id="policy-model">{{ l10n:repository.policy_model.heading }}</h2>

{{ l10n:repository.policy_model.authority_intro }}

{{ l10n:repository.policy_model.deny_precedence }}

{{ l10n:repository.policy_model.record_locations }}

```bash
global_kero="$(kero global path --plain)"
kero policy authorize --environment .kero/policy/environment.toml --records "$global_kero/policy/records.toml" --records .kero/policy/records.toml --request .kero/tests/write-source.toml
```

{{ l10n:repository.policy_model.trust_modes }}

<h2 id="status">{{ l10n:repository.status.heading }}</h2>

{{ l10n:repository.status.channel_notice }}

<h2 id="development">{{ l10n:repository.development.heading }}</h2>

{{ l10n:repository.development.consumer_setup }}

{{ l10n:repository.development.validation }}

<h2 id="project">{{ l10n:repository.project.heading }}</h2>

{{ l10n:repository.project.branch_ownership }}

{{ l10n:repository.project.repository_topology }}

### {{ l10n:repository.license.heading }}

{{ l10n:repository.license.notice }}
