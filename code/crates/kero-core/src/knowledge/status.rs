use super::*;
use serde::{Deserialize, Serialize};

pub const STATUS_VERSION: &str = "kero/project-status/v1alpha1";
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MountStatusKind {
    Current,
    Changed,
    NotCompiled,
    Error,
    Disabled,
    Unavailable,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MountStatus {
    pub mount_id: MountId,
    pub state: MountStatusKind,
    pub diagnostic: Option<Diagnostic>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProjectStatus {
    pub version: String,
    pub current: bool,
    pub mounts: Vec<MountStatus>,
}

pub fn project_status(
    boundary: &ProjectBoundary,
    compiled: Option<&CompiledState>,
) -> Result<ProjectStatus, ProjectError> {
    let project = load(boundary)?;
    let project_changed = compiled
        .is_some_and(|s| declaration_digest(&project).ok().as_ref() != Some(&s.project_digest));
    let mut mounts = Vec::new();
    for mount in &project.mounts {
        let old = compiled.and_then(|s| s.mounts.iter().find(|v| v.mount_id == mount.id));
        let (state, diagnostic) = match observe_mount(&boundary.root, mount) {
            Ok(SourceObservation::Disabled { .. }) => (MountStatusKind::Disabled, None),
            Ok(SourceObservation::Unavailable { diagnostic, .. }) => {
                (MountStatusKind::Unavailable, Some(diagnostic))
            }
            Ok(SourceObservation::Unsupported { opaque }) => {
                (MountStatusKind::Error, opaque.diagnostics.first().cloned())
            }
            Ok(SourceObservation::Current { source }) => match old {
                None => (MountStatusKind::NotCompiled, None),
                Some(old)
                    if project_changed || old.revision_id != source.revision.map(|r| r.id) =>
                {
                    (MountStatusKind::Changed, None)
                }
                Some(_) => (MountStatusKind::Current, None),
            },
            Err(error) => (
                MountStatusKind::Error,
                Some(error.diagnostic(Some(mount.source_id.clone()))),
            ),
        };
        mounts.push(MountStatus {
            mount_id: mount.id.clone(),
            state,
            diagnostic,
        });
    }
    let current = compiled.is_some()
        && mounts.iter().all(|v| {
            matches!(
                v.state,
                MountStatusKind::Current | MountStatusKind::Disabled
            )
        });
    Ok(ProjectStatus {
        version: STATUS_VERSION.into(),
        current,
        mounts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn knowledge_status_distinguishes_not_compiled_and_disabled() {
        let temp = tempfile::tempdir().unwrap();
        let boundary =
            initialize(temp.path(), KnowledgeSetId::new("status.test").unwrap()).unwrap();
        let mut project = load(&boundary).unwrap();
        let mut mount = Mount::new(
            MountId::new("missing").unwrap(),
            SourceId::new("source.missing").unwrap(),
            "missing.md",
            MountKind::LocalFile,
            0,
        );
        mount.enabled = false;
        project.add_mount(mount).unwrap();
        save(&boundary, &project).unwrap();
        let status = project_status(&boundary, None).unwrap();
        assert_eq!(status.mounts[0].state, MountStatusKind::Disabled);
        assert!(!status.current);
    }
}
