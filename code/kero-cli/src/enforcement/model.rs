use serde::{Deserialize, Serialize};

/// Metadata that is safe to record for a mediated attempt. Secret material,
/// request bodies, and credentials are deliberately absent from this type.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Attempt {
    pub artifact_id: String,
    pub nonce: String,
    pub boundary: String,
    pub operation: String,
    pub target_digest: String,
    pub snapshot_digest: String,
    pub generation: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Lifecycle {
    Prepared,
    Completed,
    Failed,
    Blocked,
    Uncertain,
}
