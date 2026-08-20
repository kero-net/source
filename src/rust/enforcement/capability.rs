use crate::result::Capability;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapabilityObservation {
    pub schema: String,
    pub capability: Capability,
    pub mechanism: String,
    pub observed_at: String,
    pub expires_at: String,
}

/// Observes only an executable path; it does not run a command or inherit its
/// environment. Boundaries must re-observe before an expired observation.
pub fn observe_command(
    path: &Path,
    mechanism: &str,
    now: DateTime<Utc>,
    lifetime: chrono::Duration,
) -> CapabilityObservation {
    let capability = match path.metadata() {
        Ok(metadata) if metadata.is_file() => Capability::Can,
        Ok(_) => Capability::Cannot,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Capability::Cannot,
        Err(_) => Capability::Unknown,
    };
    CapabilityObservation {
        schema: "scope/capability-observation/v1".into(),
        capability,
        mechanism: mechanism.into(),
        observed_at: now.to_rfc3339_opts(SecondsFormat::Secs, true),
        expires_at: (now + lifetime).to_rfc3339_opts(SecondsFormat::Secs, true),
    }
}
