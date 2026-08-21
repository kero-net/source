# Policy Validation Fixtures

The Rust integration tests keep complete malformed-record scenarios inline when
they need one focused mutation of the canonical valid policy. This directory
holds immutable file-level validation cases whose failure depends on parsing or
request shape rather than a single record mutation.

Fixtures are intentionally non-secret and must have a matching stable reason
code asserted by `code/crates/kero-cli/tests/policy_validation.rs`.
