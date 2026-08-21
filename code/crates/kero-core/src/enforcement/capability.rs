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

impl CapabilityObservation {
    /// Malformed or expired evidence is never usable by a boundary.
    pub fn is_current(&self, now: DateTime<Utc>) -> bool {
        DateTime::parse_from_rfc3339(&self.observed_at)
            .ok()
            .zip(DateTime::parse_from_rfc3339(&self.expires_at).ok())
            .is_some_and(|(observed, expires)| {
                observed <= expires && now <= expires.with_timezone(&Utc)
            })
    }
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
        schema: "kero/capability-observation/v1".into(),
        capability,
        mechanism: mechanism.into(),
        observed_at: now.to_rfc3339_opts(SecondsFormat::Secs, true),
        expires_at: (now + lifetime).to_rfc3339_opts(SecondsFormat::Secs, true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn stale_or_malformed_observations_cannot_be_used() {
        let now = Utc::now();
        let observation = observe_command(
            Path::new("/missing-kero-capability"),
            "fixture/v1",
            now,
            Duration::seconds(1),
        );
        assert!(observation.is_current(now));
        assert!(!observation.is_current(now + Duration::seconds(2)));
        let malformed = CapabilityObservation {
            expires_at: "not-a-time".into(),
            ..observation
        };
        assert!(!malformed.is_current(now));
    }
}
