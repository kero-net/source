use serde::{Deserialize, Serialize};

use super::{SourceId, SourceRegion, SourceRegionId, SourceRevision, SourceRevisionId};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub source_id: SourceId,
    pub revision_id: SourceRevisionId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_id: Option<SourceRegionId>,
}

impl Provenance {
    pub fn source(revision: &SourceRevision) -> Self {
        Self {
            source_id: revision.source_id.clone(),
            revision_id: revision.id.clone(),
            region_id: None,
        }
    }

    pub fn region(region: &SourceRegion) -> Self {
        Self {
            source_id: region.source_id.clone(),
            revision_id: region.revision_id.clone(),
            region_id: Some(region.id.clone()),
        }
    }

    pub fn validate(&self, revision: &SourceRevision, region: Option<&SourceRegion>) -> bool {
        if self.source_id != revision.source_id || self.revision_id != revision.id {
            return false;
        }
        match (&self.region_id, region) {
            (None, None) => true,
            (Some(expected), Some(actual)) => {
                expected == &actual.id
                    && actual.source_id == self.source_id
                    && actual.revision_id == self.revision_id
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::SourceId;
    use std::collections::BTreeMap;

    #[test]
    fn knowledge_provenance_binds_exact_revision_and_region() {
        let revision = SourceRevision::from_bytes(
            SourceId::new("source.docs").unwrap(),
            "text/plain",
            b"knowledge",
            BTreeMap::new(),
        );
        let region = SourceRegion::utf8(&revision, "knowledge", 0, 9, None).unwrap();
        assert!(Provenance::source(&revision).validate(&revision, None));
        assert!(Provenance::region(&region).validate(&revision, Some(&region)));

        let other = SourceRevision::from_bytes(
            SourceId::new("source.docs").unwrap(),
            "text/plain",
            b"changed",
            BTreeMap::new(),
        );
        assert!(!Provenance::region(&region).validate(&other, Some(&region)));
    }
}
