# SCOPE Version 1 Public Release Checklist

This checklist tracks work toward the first publicly supported SCOPE version 1
release. It records progress only; canonical behavior remains defined by the
registered knowledge contracts and approved boundary packages.

The proposed implementation order, planned file tree, and per-deliverable
evidence are maintained in
[`initial-release-plan.md`](initial-release-plan.md). Approve that plan before
using this checklist to begin new release work.

## Release invariant

- [ ] Every capability claimed by version 1 has direct Linux-local evidence.
- [ ] Unsupported capabilities are omitted or clearly reported as unsupported.
- [ ] Layer 2 remains pure and replayable; Layer 3 verifies and enforces without re-deciding policy.
- [ ] No `ENFORCED` result is possible without the required deployment attestation and protected paths.
- [ ] All committed code and public material remain ISC licensed.

## Stage 1 — Freeze the version 1 scope

- [x] Confirm the exact commands and compatibility surfaces included in version 1.
- [x] Decide which approved Layer 3 packages are required for v1 or explicitly deferred.
- [x] Record unsupported platforms and operations; Windows remains deferred.
- [x] Freeze serialized schema, resolver, boundary, adapter, and threat-model identifiers.
- [x] Define compatibility promises for policy files, snapshots, artifacts, and execution results.
- [x] Define the support status of `ADVISORY` versus `ENFORCED` operation.

## Stage 2 — Complete Layer 1 and Layer 2 foundations

- [x] Discover global and project `.scope/` environments independently.
- [x] Keep global and project knowledge inside their respective `.scope/` environments.
- [x] Initialize only one consumer-repository integration directory.
- [x] Serialize independent authorization, verification, capability, enforcement, and execution dimensions.
- [x] Produce RFC 8785 canonical JSON and SHA-256 digests.
- [x] Issue and verify HMAC-authenticated authorization artifacts.
- [x] Reject authentic artifacts containing a Layer 2 `DENY` decision.
- [x] Replay immutable snapshots without mutable TOML and retain byte-equivalent results.
- [x] Complete policy environment and record validation parity.
- [x] Cover every applicability gate independently, including inapplicable-versus-indeterminate precedence.
- [x] Complete restrictive-source behavior and supported delegation-depth parity.
- [x] Prove restrictive deny-only roles and assignments can narrow authority but cannot grant it.
- [x] Prove valid multi-hop delegation retains its complete chain and enforces redelegation depth.
- [x] Prove forbidden redelegation and complementary incomplete chains fail closed.
- [x] Reject malformed, duplicate, role-cycle, missing-reference, unauthorized-source, and unsupported-version records fail closed.
- [x] Reject delegation cycles with stable validation evidence.
- [x] Add immutable shared fixtures for every Layer 2 validation and decision-table case.
- [x] Verify snapshot retrieval, canonical-byte rejection, digest mismatch, and immutable collision behavior.

## Stage 3 — Complete common Layer 3 infrastructure

- [ ] Define one reusable attempt/result API without changing Layer 2 result semantics.
- [ ] Verify exact operation, target, principal, session, audience, snapshot, context, execution binding, and freshness.
- [ ] Implement durable single-use nonce admission with serialized replay rejection.
- [ ] Implement and verify the append-only audit hash chain before every admitted operation.
- [ ] Record prepared, completed, failed, blocked, and uncertain outcomes without secrets.
- [ ] Implement capability observation with bounded provenance and lifetime.
- [ ] Implement safe-disable and boundary generation invalidation.
- [ ] Verify broker-owned key, nonce, audit, and checkpoint permissions.
- [ ] Keep unauthenticated deployment claims at `ADVISORY`.
- [ ] Add crash and fault-injection harnesses shared by boundary tests.

## Stage 4 — Exact Linux workspace write

- [ ] Walk existing parents descriptor-relatively with `O_DIRECTORY | O_NOFOLLOW`.
- [ ] Reject empty, absolute, dot, parent, NUL, backslash, multi-target, and mismatched paths.
- [ ] Prevent symlink traversal and path replacement races.
- [ ] Create, fsync, rename, and directory-fsync through the bound parent descriptor.
- [ ] Consume the nonce after durable audit admission and before the filesystem side effect.
- [ ] Block on artifact, snapshot, mapping, capability, audit, nonce, and write failures.
- [ ] Preserve consumed nonces after uncertain failures and never retry automatically.
- [ ] Test escape attempts, target mismatch, replay, audit corruption, unavailable capability, and crash points.
- [ ] Add an end-to-end CLI test proving exact authorized writes and denied execution.

## Stage 5 — Bubblewrap command boundary

- [ ] Verify the exact signed argument vector, working directory, environment, timeout, output limit, root, and network mode.
- [ ] Require Bubblewrap and report unavailable capability without changing authorization.
- [ ] Run with unshared namespaces, read-only host root and workspace, hidden user/data paths, private temporary storage, and no network.
- [ ] Clear ambient environment and reject credentials, interactive TTYs, writable host paths, and alternate mount layouts.
- [ ] Bound execution time and captured output.
- [ ] Test filesystem isolation, network isolation, timeout, overflow, binding mismatch, replay, audit failure, and safe-disable.

## Stage 6 — Git push and credential broker

- [ ] Bind one repository identity, working tree, remote URL, exact refspec, timeout, and optional credential handle.
- [ ] Reject force, deletion, wildcard, rewritten remote, arbitrary arguments, hooks, helpers, and interactive prompting.
- [ ] Broker credentials through temporary `GIT_ASKPASS` material inaccessible to the caller.
- [ ] Ensure credentials never enter results, audit events, process arguments, fixtures, or generated files.
- [ ] Test exact pushes against a broker-owned local bare remote.
- [ ] Test replay, timeout, binding mismatch, credential failure, nonzero Git outcome, cleanup, and secret non-disclosure.
- [ ] Keep network pushes `ADVISORY` until composed with an attested HTTPS boundary.

## Stage 7 — Exact HTTPS service boundary

- [ ] Bind HTTPS URL, method, headers, body digest, timeout, response limit, IP address, CA input, and optional credential header.
- [ ] Disable proxies, redirects, ambient netrc, userinfo, fragments, DNS-selected destinations, and alternate protocols.
- [ ] Invoke Curl with exact `--resolve` destination binding while authenticating the signed hostname.
- [ ] Keep body, CA, credential, and broker state inaccessible to the caller.
- [ ] Test local TLS success, HTTP errors, TLS failure, timeout, overflow, replay, binding mismatch, unavailable Curl, and safe-disable.
- [ ] Verify credentials and request bodies never leak into diagnostics or audit evidence.

## Stage 8 — Keep elevation disabled for version 1

- [x] Record elevation, emergency bypass, interactive sudo, and root-role shortcuts as unsupported in the v1 scope.
- [ ] Ensure no CLI command, configuration, or artifact path enables elevation.
- [ ] Test that approval-shaped inputs cannot mutate or reinterpret an existing `DENY`.
- [ ] Keep the separately approved elevation package outside public v1 claims and binaries.

## Stage 9 — Recovery and rollback

- [ ] Sign checkpoints and bind them to generation, audit head, policy snapshot, boundary state, and protected resources.
- [ ] Fail closed when checkpoint, audit, nonce, policy, or boundary evidence is missing, corrupt, stale, or unverifiable.
- [ ] Invalidate earlier artifacts whenever the boundary generation changes.
- [ ] Preserve audit continuity through safe-disable and recovery attempts.
- [ ] Require proof-gated restore; never rewind nonce or generation counters.
- [ ] Test interrupted writes, audit truncation, checkpoint corruption, state loss, rollback attempts, and recovery retries.

## Stage 10 — Product hardening and release validation

- [ ] Expose every supported v1 operation through stable, documented CLI commands.
- [ ] Return structured machine-readable errors and stable exit behavior.
- [ ] Remove panics from untrusted input and operational failure paths.
- [ ] Run formatting, strict Clippy, all Rust targets, localization tests, publication tests, actionlint, shell syntax, and repository validation.
- [x] Run tests with locked dependencies on the minimum supported Rust version and current stable Rust.
- [ ] Build the optimized Linux release with `cargo build --release --locked`.
- [x] Run clean-machine installation and smoke tests on Ubuntu, Debian, and Arch.
- [ ] Audit dependencies, licenses, committed secrets, generated residue, and unsafe code.
- [ ] Complete security review of each claimed boundary and residual-risk statement.
- [ ] Confirm no known release-blocking defects remain.

## Stage 11 — Release identity and artifacts

- [ ] Set the crate and CLI version to the approved v1 release version.
- [ ] Confirm the release ID follows the existing `YYYY.MM.N-KIND` publication format.
- [ ] Author the matching release record with summary, changes, issues, compatibility, and verification.
- [ ] Produce reproducible Linux binaries and record checksums and source provenance.
- [ ] Verify generated canary, beta, and stable payload manifests against the immutable source commit.
- [ ] Exercise the local publication workflow without root-owned generated files.
- [ ] Verify signing, protected environment values, branch replacement, tag creation, and GitHub Release behavior in a non-production rehearsal.

## Stage 12 — Public documentation and repository setup

- [ ] Finish public README content only after supported v1 behavior is frozen.
- [ ] Keep Markdown structure in templates and translation catalogs text-only with semantic keys.
- [ ] Complete and review English and Japanese README and Pages output for structural parity.
- [ ] Document installation, global environment setup, project initialization, policy authoring, artifact verification, and each supported enforcement command.
- [ ] Document threat models, enforcement boundaries, `ADVISORY` limitations, residual risks, recovery, and unsupported platforms.
- [ ] Update examples so every command runs against the final v1 CLI.
- [ ] Finalize repository topics, description, funding, support, governance, security policy, issue forms, discussions, CODEOWNERS, citation, and contribution guidance.
- [ ] Confirm source, canary, beta, stable, Pages, release, and Visit-Counter publication formats remain intact.
- [ ] Validate all local links, localized navigation, generated README locations, and Pages routing.
- [ ] Perform a final public-repository review from a clean clone with no private or machine-local paths.
- [ ] Obtain explicit maintainer sign-off for public v1 publication.

## Release decision

- [ ] Every required stage above is complete or explicitly deferred outside the claimed v1 scope.
- [ ] The parity matrix marks every claimed capability implemented with direct evidence.
- [ ] The final source commit is clean, immutable, validated, and approved.
- [ ] Publish version 1.
