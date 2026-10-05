<div align="center">
  <img src="assets/images/kero-icon.png">
  <h1>KERO</h1>
  <h3>{{ l10n:repository.hero.tagline }}</h3>
  <p><a href="https://github.com/kero-net/kero"><img alt="Stars + Issues + License" src="https://shieldcn.dev/group/github/stars/kero-net/kero+github/kero-net/kero/issues+github/license/kero-net/kero.svg?variant=outline"></a></p>
  <table><tr><td><a href="#direction">{{ l10n:repository.navigation.direction }}</a></td><td><a href="#architecture">{{ l10n:repository.navigation.architecture }}</a></td><td><a href="#setup">{{ l10n:repository.navigation.setup }}</a></td><td><a href="#status">{{ l10n:repository.navigation.status }}</a></td><td><a href="#development">{{ l10n:repository.navigation.development }}</a></td><td><a href="#project">{{ l10n:repository.navigation.project }}</a></td></tr></table>
  {{ locales:repository }}
</div>

<h2 id="direction">{{ l10n:repository.direction.heading }}</h2>

{{ l10n:repository.direction.introduction }}

- {{ l10n:repository.direction.mounts }}
- {{ l10n:repository.direction.provenance }}
- {{ l10n:repository.direction.machine_first }}
- {{ l10n:repository.direction.project_state }}

<h2 id="architecture">{{ l10n:repository.architecture.heading }}</h2>

{{ l10n:repository.architecture.pipeline_intro }}

```text
sources
  → mounted project environment
  → source-preserving model
  → semantic IR
  → canonicalization
  → canonical encoding
  → queries and bounded context projections
```

{{ l10n:repository.architecture.boundary }}

<h2 id="setup">{{ l10n:repository.setup.heading }}</h2>

{{ l10n:repository.setup.introduction }}

### {{ l10n:repository.setup.windows_heading }}

{{ l10n:repository.setup.windows_intro }}

```text
winget install Rustlang.Rustup
winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
rustup toolchain install stable
```

{{ l10n:repository.setup.windows_after_install }}

```text
git clone https://github.com/kero-net/kero.git
cd kero/src
cargo test --all-targets --all-features --locked
cargo run -p kero-cli -- --help
```

### {{ l10n:repository.setup.macos_heading }}

{{ l10n:repository.setup.macos_intro }}

```bash
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"
```

### {{ l10n:repository.setup.linux_heading }}

{{ l10n:repository.setup.linux_intro }}

#### {{ l10n:repository.setup.debian_heading }}

```bash
sudo apt update
sudo apt install --yes build-essential curl
```

#### {{ l10n:repository.setup.fedora_heading }}

```bash
sudo dnf install --assumeyes gcc gcc-c++ make curl
```

#### {{ l10n:repository.setup.arch_heading }}

```bash
sudo pacman -S --needed base-devel curl
```

{{ l10n:repository.setup.unix_build_intro }}

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"
git clone https://github.com/kero-net/kero.git
cd kero/src
cargo test --all-targets --all-features --locked
cargo run -p kero-cli -- --help
```

{{ l10n:repository.setup.note }}

<h2 id="status">{{ l10n:repository.status.heading }}</h2>

{{ l10n:repository.status.pre_alpha }}

{{ l10n:repository.status.cli }}

```bash
cd src
cargo build --release --locked
cargo test --all-targets --all-features --locked
cargo run -p kero-cli -- --help
```

<h2 id="development">{{ l10n:repository.development.heading }}</h2>

{{ l10n:repository.development.validation }}

```bash
cargo fmt --manifest-path src/Cargo.toml --all -- --check
cargo clippy --manifest-path src/Cargo.toml --all-targets --all-features --locked -- -D warnings
cargo test --manifest-path src/Cargo.toml --all-targets --all-features --locked
```

<h2 id="project">{{ l10n:repository.project.heading }}</h2>

{{ l10n:repository.project.repository_topology }}

### {{ l10n:repository.license.heading }}

{{ l10n:repository.license.notice }}
