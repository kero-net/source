# Initial Release Implementation Plan

> **Approval status:** proposed. Do not start an unchecked implementation item
> until this plan has maintainer approval. After approval, this is the working
> index for version 1; `release-v1.md` remains the release gate.

This plan turns the frozen version 1 scope into ordered, independently
verifiable deliverables. It does not change a policy, serialization, or
boundary contract. Those remain in registered knowledge documents, especially
`knowledge/shared/releases/version-1.md`.

## How to use this hub

- [ ] Approve the scope, file map, and release order below as the v1 direction.
- [ ] Before each phase, promote its unchecked items to the active work list in
  `release-v1.md`; do not begin a later phase early.
- [ ] For each implementation item, add or update the named tests and record
  the command that passed in the pull request.
- [ ] If a file, schema, boundary identifier, or support claim must change,
  update this plan and the relevant registered contract in the same review.
- [ ] Mark an item complete only after code review and its direct evidence pass.

## Delivery constraints

- [ ] Preserve the three-layer boundary: knowledge resolution, pure policy,
  then native enforcement.
- [ ] Keep Layer 2 deterministic, side-effect free, and replayable from its
  immutable snapshot.
- [ ] Make every Layer 3 failure fail closed without rewriting authorization.
- [ ] Treat Linux as the only native enforcement target for v1.
- [ ] Keep unsupported elevation, arbitrary execution, broad filesystem access,
  and network sandboxing outside the release.
- [ ] Do not add a dependency, external service, or platform-specific feature
  unless its threat model and release evidence are approved first.

## Planned end-state tree

`+` means a file planned to be created; `~` means an existing file planned to
be materially updated. Directories containing only existing files are shown to
make ownership and placement unambiguous.

```text
.scope/
├── checklists/
│   ├── README.md                                      ~
│   ├── initial-release-plan.md                         +  (this hub)
│   └── release-v1.md                                  ~
├── knowledge/
│   └── shared/
│       ├── architecture/
│       │   ├── parity.md                              ~
│       │   └── repository.md                          ~
│       └── releases/
│           └── version-1.md                           ~
└── tests/
    ├── policy/                                        +  immutable TOML/JSON fixtures
    ├── boundaries/                                    +  Linux-only integration fixtures
    └── recovery/                                      +  fault and state fixtures

src/
├── Cargo.toml                                         ~
├── rust/
│   ├── lib.rs                                         ~
│   ├── main.rs                                        ~
│   ├── result.rs                                      ~
│   ├── artifact.rs                                    ~
│   ├── workspace_write.rs                             ~
│   ├── enforcement/                                   +
│   │   ├── mod.rs                                     +
│   │   ├── model.rs                                   +
│   │   ├── verify.rs                                  +
│   │   ├── nonce.rs                                   +
│   │   ├── audit.rs                                   +
│   │   ├── capability.rs                              +
│   │   ├── state.rs                                   +
│   │   └── recovery.rs                                +
│   └── boundary/                                      +
│       ├── mod.rs                                     +
│       ├── workspace_write.rs                         +
│       ├── bubblewrap.rs                              +
│       ├── git_push.rs                                +
│       └── https_service.rs                            +
└── tests/
    ├── policy_validation.rs                           ~
    ├── policy_parity.rs                               ~
    ├── restrictive_policy.rs                          ~
    ├── delegation.rs                                  ~
    ├── enforcement_common.rs                          +
    ├── workspace_write_boundary.rs                    +
    ├── bubblewrap_boundary.rs                         +
    ├── git_push_boundary.rs                            +
    ├── https_service_boundary.rs                       +
    ├── recovery.rs                                    +
    ├── cli_release.rs                                 +
    └── fixtures/
        ├── policy/                                    ~
        ├── enforcement/                               +
        ├── boundaries/                                +
        └── recovery/                                  +

content/
├── pages/
│   ├── getting-started/getting-started.toml           ~
│   ├── policy-authoring.toml                           +
│   ├── enforcement-boundaries.toml                    +
│   ├── recovery-and-limits.toml                       +
│   └── releases/version-1.toml                         +
├── repo/shared/
│   ├── README.template.md                             ~
│   └── readme.repository.toml                         ~
└── releases/
    └── YYYY.MM.N-KIND.release.md                      +  created only when version is approved
```

The tree is intentionally a plan, not permission to add empty placeholders.
Create a listed file only with its phase. Existing `src/rust/workspace_write.rs`
will be moved into `boundary/workspace_write.rs` only in the common-enforcement
refactor, as one reviewed atomic change; it must not be duplicated long term.

## Phase 0 — Approval and baseline

- [x] Confirm the release target is the first public **v1**, while the exact
  semver crate version and `YYYY.MM.N-KIND` release ID remain release-decision
  inputs.
- [x] Approve this tree and the four boundary sequence: workspace write,
  Bubblewrap command, Git push, HTTPS service.
- [x] Capture a clean baseline: `cargo fmt --all --check`, strict Clippy,
  `cargo test --all-targets`, localization/publication tests, repository
  validation, actionlint, and shell syntax checks.
- [x] Reconcile pre-existing uncommitted work with this plan before claiming
  any checkbox as evidence.
- [x] Update `release-v1.md` only to link back to this hub and retain it as the
  single high-level release gate.

### Phase 0 evidence — 2026-08-19

All repository validation commands passed. The documentation build initially
could not find MkDocs in the system Python; it subsequently passed using the
repository's exact `.github/automation-requirements.txt` in the ignored local
`.generated/phase0-venv` environment. The Material-for-MkDocs upstream
deprecation notice was informational and did not make the strict build fail.

| Check | Result |
|---|---|
| `cargo fmt --all --check` | pass |
| strict `cargo clippy --all-targets --all-features -- -D warnings` | pass |
| `cargo test --all-targets --locked` | pass (39 tests) |
| `cargo build --release --locked` | pass |
| repository, localization, publication, and manifest tests | pass |
| shell syntax and actionlint | pass |
| strict localized MkDocs build | pass |

Existing uncommitted work is preserved and maps to this plan as follows:

| Existing area | Planned phase |
|---|---|
| `src/rust/policy/{authority,load,resolve,scope,snapshot}.rs` and policy tests | Phase 1 |
| `.scope/knowledge*` and architecture/release knowledge | Scope baseline and Phase 1 evidence |
| localization, repository README, publication scripts, and VS Code tasks | Phase 8/repository infrastructure; preserved until release documentation review |
| this checklist directory | Phase 0 |

## Phase 1 — Finish pure policy parity

### Implementation

- [x] `~ src/rust/policy/load.rs`: validate environment and records before
  resolution; surface stable validation reason codes.
- [x] `~ src/rust/policy/model.rs`: represent every accepted v1 validation
  field without implicit defaults that broaden authority.
- [x] `~ src/rust/policy/scope.rs`: implement each applicability gate and its
  inapplicable/indeterminate precedence.
- [x] `~ src/rust/policy/authority.rs`: reject cycles and unsupported
  delegation shapes with stable, deterministic evidence.
- [x] `~ src/rust/policy/resolve.rs`: preserve deny-only and delegation-depth
  behavior while keeping explanation provenance complete.
- [x] `~ src/rust/policy/snapshot.rs`: bind validated source identity and
  resolver version to the immutable replayable snapshot.

### Evidence and fixtures

- [x] `~ src/tests/policy_validation.rs`: table-test all malformed,
  duplicate, missing-reference, cycle, source, and version failures.
- [x] `~ src/tests/policy_parity.rs`: cover every applicability gate and
  precedence branch against immutable fixtures.
- [x] `~ src/tests/restrictive_policy.rs` and `~ src/tests/delegation.rs`:
  cover restrictive narrowing, complete chains, too-deep chains, and cycles.
- [x] `+ src/tests/fixtures/policy/validation/*.toml`: one minimal fixture per
  validation failure and one accepted edge case.
- [x] `+ src/tests/fixtures/policy/decisions/*.toml`: one named decision-table
  case per gate/precedence branch, with expected result JSON beside it.
- [x] Update `~ .scope/knowledge/shared/architecture/parity.md` only after the
  direct tests pass.

### Exit gate

- [x] No invalid policy input panics or silently widens authority.
- [x] Snapshot replay is byte-equivalent and never rereads mutable TOML.
- [x] Every Layer 2 parity row has an executable local test.

## Phase 2 — Common enforcement kernel

### Implementation

- [ ] `+ src/rust/enforcement/model.rs`: define the reusable typed attempt,
  verified binding, result reason, lifecycle phase, and safe-disable state.
- [ ] `+ src/rust/enforcement/verify.rs`: verify artifact, signature, snapshot,
  audience, principal, session, operation, targets, context, execution binding,
  and freshness without invoking policy resolution.
- [ ] `+ src/rust/enforcement/nonce.rs`: durable single-use nonce admission
  with atomic replay rejection and no automatic retry after uncertainty.
- [ ] `+ src/rust/enforcement/audit.rs`: append-only, fsynced hash-chain events
  for prepared/completed/failed/blocked/uncertain outcomes.
- [ ] `+ src/rust/enforcement/capability.rs`: bounded observations, expiry, and
  stable unavailable/unknown outcomes.
- [ ] `+ src/rust/enforcement/state.rs`: broker-owned state layout, permissions,
  generation invalidation, and protected-resource checks.
- [ ] `+ src/rust/enforcement/recovery.rs`: signed checkpoint verification and
  proof-gated recovery that never rewinds nonce or generation state.
- [ ] `+ src/rust/enforcement/mod.rs` and `~ src/rust/lib.rs`: expose only the
  required public interfaces.
- [ ] `~ src/rust/result.rs`: keep the independent result dimensions stable;
  add only documented reason-code serialization.

### Evidence

- [ ] `+ src/tests/enforcement_common.rs`: verification mismatch matrix,
  nonce replay, audit-chain corruption, permissions, safe-disable, and result
  dimension tests.
- [ ] `+ src/tests/recovery.rs`: checkpoint mismatch, truncation, state loss,
  interrupted lifecycle, and non-rewind tests.
- [ ] `+ src/tests/fixtures/enforcement/*.json`: canonical artifact, binding,
  audit, capability, and checkpoint fixtures—never real keys or secrets.
- [ ] `+ src/tests/fixtures/recovery/*`: corrupt and interrupted state samples.

### Exit gate

- [ ] A boundary can use the kernel without recomputing Layer 2.
- [ ] Failed audit/nonce/capability/checkpoint evidence blocks side effects.
- [ ] `ENFORCED` cannot be returned without authenticated deployment evidence.

## Phase 3 — Exact Linux workspace write

- [ ] `+ src/rust/boundary/mod.rs`: register supported native boundary adapters.
- [ ] `+ src/rust/boundary/workspace_write.rs`: replace the current adapter with
  descriptor-relative parent walking, `O_DIRECTORY | O_NOFOLLOW`, atomic
  create/fsync/rename/directory-fsync, and kernel lifecycle admission.
- [ ] `~ src/rust/workspace_write.rs`: remove after the move; retain no second
  adapter implementation.
- [ ] `+ src/tests/workspace_write_boundary.rs`: test invalid paths, symlinks,
  replacement races, mismatches, nonce/audit failures, unavailable capability,
  crash points, success, and uncertain failures.
- [ ] `+ src/tests/fixtures/boundaries/workspace-write/*`: checked-in safe
  workspace layouts and expected non-secret outputs.
- [ ] `~ src/rust/main.rs`: add the stable write command and structured result
  only after the boundary exit gate passes.

### Exit gate

- [ ] Only one artifact-bound existing workspace target can be written.
- [ ] Escape, replay, audit failure, and post-admission uncertainty preserve a
  consumed nonce and block automatic retry.

## Phase 4 — Bubblewrap command boundary

- [ ] `+ src/rust/boundary/bubblewrap.rs`: bind an exact argv, working
  directory, environment, timeout, output limit, root, and network mode.
- [ ] `+ src/tests/bubblewrap_boundary.rs`: test absent Bubblewrap, binding
  mismatch, mounts, network/filesystem isolation, TTY/credential rejection,
  timeout, output overflow, replay, audit failure, and safe-disable.
- [ ] `+ src/tests/fixtures/boundaries/bubblewrap/*`: deterministic harmless
  commands and expected result fixtures.
- [ ] `~ src/rust/main.rs`: add the stable no-network command interface.

### Exit gate

- [ ] The sandbox starts with cleared ambient environment, no network, private
  temporary storage, and no writable host paths.
- [ ] Capability absence is reported without changing authorization.

## Phase 5 — Git push and credential broker

- [ ] `+ src/rust/boundary/git_push.rs`: bind repository identity, worktree,
  remote URL, exact refspec, timeout, and optional credential handle.
- [ ] `+ src/tests/git_push_boundary.rs`: use a broker-owned local bare remote
  for success, replay, timeout, mismatch, credential failure, cleanup, Git
  failure, and secret-non-disclosure tests.
- [ ] `+ src/tests/fixtures/boundaries/git-push/*`: non-secret repositories,
  configs, and expected results.
- [ ] `~ src/rust/main.rs`: add the exact Git push command and prohibit caller
  control of hooks, helpers, prompting, force, deletion, and wildcard refspecs.

### Exit gate

- [ ] Credentials appear in neither process arguments, result/audit data,
  fixture files, nor generated artifacts.
- [ ] Network push output remains `ADVISORY` until an attested HTTPS boundary
  composes it.

## Phase 6 — Exact HTTPS service boundary

- [ ] `+ src/rust/boundary/https_service.rs`: bind HTTPS URL, method, headers,
  body digest, timeout, response limit, IP address, CA input, and optional
  credential header; invoke Curl with exact `--resolve` binding.
- [ ] `+ src/tests/https_service_boundary.rs`: test local TLS success, HTTP
  error, TLS failure, timeout, overflow, replay, binding mismatch, absent Curl,
  safe-disable, and redaction.
- [ ] `+ src/tests/fixtures/boundaries/https-service/*`: local test CA,
  server configuration, safe request bodies, and expected outputs. Do not add
  production credentials.
- [ ] `~ src/rust/main.rs`: add the stable exact-service command interface.

### Exit gate

- [ ] Proxies, redirects, netrc, userinfo, fragments, alternate protocols, and
  DNS-selected destinations are rejected or disabled.
- [ ] Caller-accessible state cannot reveal request bodies, CAs, or credentials.

## Phase 7 — CLI, quality, and recovery rehearsal

- [ ] `~ src/rust/main.rs`: consolidate all supported commands, JSON schemas,
  reason codes, exit codes, and unsupported-operation behavior.
- [ ] `+ src/tests/cli_release.rs`: exercise every supported command, invalid
  input, stable machine-readable error, and exit-code path.
- [ ] `~ src/Cargo.toml`: add only approved dependencies; lock and document
  their license/security review in the release evidence.
- [ ] `~ CONTRIBUTING.md`: state the complete verification matrix if it changes.
- [x] Run clean-machine smoke tests on the supported Linux distributions and
  release builds on minimum-supported and current stable Rust.
- [ ] Run fault injection through every boundary lifecycle phase.
- [ ] Audit dependencies, licenses, secrets, generated residue, unsafe code,
  and residual risks.

### Exit gate

- [ ] The optimized locked build passes all automated checks.
- [ ] A clean clone contains no machine-local paths, state, keys, credentials,
  or undocumented support claims.

## Phase 8 — Release identity, artifacts, and public material

- [ ] `~ src/Cargo.toml`: set the approved v1 version only after all prior
  exit gates pass.
- [ ] `+ content/releases/YYYY.MM.N-KIND.release.md`: create the approved
  release record with exact versioned name, compatibility, verification, and
  residual-risk summary.
- [ ] `~ content/repo/shared/README.template.md` and
  `~ content/repo/shared/readme.repository.toml`: describe only proven v1
  commands and limitations, preserving localization structure.
- [ ] `~ content/pages/getting-started/getting-started.toml`: update final
  install and initialization instructions.
- [ ] `+ content/pages/policy-authoring.toml`: document accepted policy files,
  authoring, snapshots, and verification.
- [ ] `+ content/pages/enforcement-boundaries.toml`: document each supported
  boundary, threat model, `ADVISORY` limits, and unsupported operations.
- [ ] `+ content/pages/recovery-and-limits.toml`: document safe-disable,
  recovery, retention, and operational limits.
- [ ] `+ content/pages/releases/version-1.toml`: publish the approved v1
  release summary and artifact provenance.
- [ ] `~ .scope/knowledge/shared/releases/version-1.md`: update only if final
  evidence narrows a support claim; do not expand scope at this stage.
- [ ] `~ .scope/knowledge/shared/architecture/parity.md`: mark each capability
  implemented only with named direct evidence.
- [ ] `~ .scope/knowledge/shared/architecture/repository.md`: update the tree
  only if actual repository ownership changes.
- [ ] Produce reproducible Linux binaries, checksums, source provenance,
  generated-channel manifests, and a non-production publication rehearsal.
- [ ] Validate English/Japanese structural parity, Pages routes, local links,
  publication scripts, protected values, signing, tag, and GitHub Release flow.

### Final decision

- [ ] Re-read every v1 public claim against its test evidence and parity row.
- [ ] Confirm every uncompleted checklist item is explicitly deferred outside
  the release claim.
- [ ] Obtain explicit maintainer approval for the immutable release commit.
- [ ] Publish version 1.

## Deferred from this plan

- [ ] Do not plan files for elevation, emergency bypass, root roles, macOS,
  Windows, mobile, Flatpak, arbitrary process mediation, or a global host
  enforcement agent. Each requires a separate approved scope package.
