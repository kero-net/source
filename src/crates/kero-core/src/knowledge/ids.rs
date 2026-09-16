use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

const LOGICAL_ID_MAX: usize = 128;

#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("knowledge.id.invalid: invalid {kind} identifier `{value}`")]
pub struct IdError {
    pub kind: &'static str,
    pub value: String,
}

fn logical_id(value: &str) -> bool {
    if value.is_empty() || value.len() > LOGICAL_ID_MAX || !value.is_ascii() {
        return false;
    }
    let mut previous_separator = true;
    for (index, byte) in value.bytes().enumerate() {
        let valid = byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || (index > 0 && (byte == b'.' || byte == b'-'));
        if !valid {
            return false;
        }
        let separator = byte == b'.' || byte == b'-';
        if separator && previous_separator {
            return false;
        }
        previous_separator = separator;
    }
    !previous_separator && value.as_bytes()[0].is_ascii_lowercase()
}

fn sha256_id(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

fn region_id(value: &str) -> bool {
    value.strip_prefix("region:").is_some_and(sha256_id)
}

fn semantic_id(value: &str) -> bool {
    value.strip_prefix("sem:").is_some_and(logical_id)
}

fn record_id(value: &str) -> bool {
    value
        .strip_prefix("record:")
        .is_some_and(|rest| logical_id(rest) || sha256_id(rest))
}

fn derivation_id(value: &str) -> bool {
    value.strip_prefix("derivation:").is_some_and(sha256_id)
}

macro_rules! id_type {
    ($name:ident, $kind:literal, $validator:ident) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, IdError> {
                let value = value.into();
                if !$validator(&value) {
                    return Err(IdError { kind: $kind, value });
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = IdError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::new(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

id_type!(KnowledgeSetId, "knowledge-set", logical_id);
id_type!(MountId, "mount", logical_id);
id_type!(SourceId, "source", logical_id);
id_type!(ContentDigest, "content-digest", sha256_id);
id_type!(SourceRevisionId, "source-revision", sha256_id);
id_type!(SourceRegionId, "source-region", region_id);
id_type!(EntityId, "semantic-entity", semantic_id);
id_type!(SemanticRecordId, "semantic-record", record_id);
id_type!(DerivationId, "derivation", derivation_id);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_identifiers_are_lowercase_segmented_and_bounded() {
        assert!(KnowledgeSetId::new("project.docs-v1").is_ok());
        for invalid in [
            "",
            "Project",
            "1project",
            "project..docs",
            "project_1",
            "project-",
        ] {
            assert!(KnowledgeSetId::new(invalid).is_err(), "accepted {invalid}");
        }
        assert!(KnowledgeSetId::new("a".repeat(129)).is_err());
    }

    #[test]
    fn digest_and_region_identifiers_have_distinct_grammars() {
        let digest = format!("sha256:{}", "a".repeat(64));
        assert!(ContentDigest::new(&digest).is_ok());
        assert!(SourceRevisionId::new(&digest).is_ok());
        assert!(SourceRegionId::new(format!("region:{digest}")).is_ok());
        assert!(SourceRegionId::new(digest).is_err());
    }
}
