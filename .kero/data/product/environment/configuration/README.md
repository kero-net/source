# Environment configuration

`.kero/config` is human-owned configuration for one KERO environment. It may
contain product policy and future declarative capabilities, but it does not
encode machine paths, CPU/OS identity, credentials, installer state, or mount
transport locators.

## Configuration templates

New repository environments receive a KST template with the supported
per-mount `refresh` override (`manual` or `event`) and commented alternatives.
The last `mount <name>` section for a named mount wins. Writable access is not
repository policy: it requires a signed, source-owned `syncGrant`, then an
explicit `mount sync` operation.

The selected global home's `config` is the current preferences catalog. It
documents every active preference, its allowed values, and commented
alternatives. A user selects one value per preference by uncommenting it and
commenting the previous value. Repository configuration never overrides those
global defaults.

Human-owned configuration must preserve comments and formatting when edited by
a tool. Canonical serialization for generated data is a separate concern and
must not be used as an implicit rewrite of `config`.

There is no legacy declaration or project-TOML compatibility layer. A future
compatibility mechanism requires its own explicit product decision rather than
becoming an implicit parser fallback.

KERO Structured Text is the current syntax used for this configuration. Its
syntax contract is defined in [`structured-text.md`](structured-text.md).

## Operational setting ownership

KST is the required format for new KERO-owned, human-editable operational
settings, including policy, defaults, toolchain, and release settings. Each
mutable setting has one owning KST document and one authoritative editable
definition. Hosts, scripts, generated records, and command arguments may derive
values from that definition, but must not introduce a second editable default.

True invariants belong in code. An adjustable value belongs in the KST document
that owns its policy. Repository-owned TOML is migrated to KST incrementally
when its owning subsystem changes; TOML remains valid where an external tool
requires it, such as Cargo manifests.

Contributor review verifies setting ownership and focused contract tests verify
the affected KST schema and its consumers. KERO does not use a generic repeated
literal scan, because equal text alone does not establish duplicated policy.

## Why

Configuration is part of the portable environment contract. Keeping host facts
out of it prevents a repository from silently becoming tied to one developer's
machine or one distribution target.
