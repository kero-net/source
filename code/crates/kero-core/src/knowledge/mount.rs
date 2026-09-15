use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::{ContentDigest, MountId, SourceId};

fn enabled_by_default() -> bool {
    true
}

fn required_by_default() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MountKind {
    LocalFile,
    LocalDirectory,
    StructuredJson,
    GitRepository,
    RemoteSnapshot,
    KeroSet,
    Opaque,
}

impl MountKind {
    pub fn is_locally_acquirable(&self) -> bool {
        matches!(self, Self::LocalFile | Self::StructuredJson)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Mount {
    pub id: MountId,
    pub source_id: SourceId,
    pub source: String,
    pub kind: MountKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub importer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
    #[serde(default = "required_by_default")]
    pub required: bool,
    pub display_order: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_digest: Option<ContentDigest>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub acquisition: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: BTreeMap<String, toml::Value>,
}

impl Mount {
    pub fn new(
        id: MountId,
        source_id: SourceId,
        source: impl Into<String>,
        kind: MountKind,
        display_order: u32,
    ) -> Self {
        Self {
            id,
            source_id,
            source: source.into(),
            kind,
            importer: None,
            media_type: None,
            enabled: true,
            required: true,
            display_order,
            expected_digest: None,
            acquisition: BTreeMap::new(),
            extensions: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knowledge_mount_defaults_are_reviewable_product_state() {
        let mount = Mount::new(
            MountId::new("project-docs").unwrap(),
            SourceId::new("source.project-docs").unwrap(),
            "docs/guide.md",
            MountKind::LocalFile,
            0,
        );
        assert!(mount.enabled);
        assert!(mount.required);
        assert_eq!(mount.display_order, 0);
        assert_eq!(mount.source, "docs/guide.md");
        assert!(mount.extensions.is_empty());
    }

    #[test]
    fn knowledge_mount_source_kind_does_not_imply_semantic_precedence() {
        assert!(MountKind::LocalFile.is_locally_acquirable());
        assert!(MountKind::StructuredJson.is_locally_acquirable());
        assert!(!MountKind::RemoteSnapshot.is_locally_acquirable());
        assert!(!MountKind::KeroSet.is_locally_acquirable());
    }
}
