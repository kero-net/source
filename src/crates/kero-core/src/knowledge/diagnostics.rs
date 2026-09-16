use serde::{Deserialize, Serialize};

use super::{SourceId, SourceRegionId};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<SourceId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_id: Option<SourceRegionId>,
}

impl Diagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Error,
            message: message.into(),
            source_id: None,
            region_id: None,
        }
    }

    pub fn with_source(mut self, source_id: SourceId) -> Self {
        self.source_id = Some(source_id);
        self
    }

    pub fn with_region(mut self, region_id: SourceRegionId) -> Self {
        self.region_id = Some(region_id);
        self
    }
}
