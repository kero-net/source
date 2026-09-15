use kero_core::knowledge::{
    Availability, Mount, MountId, MountKind, ProjectError, SourceError, SourceId,
    SourceObservation, SourceRegion, discover, load, observe_mount, save,
};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/knowledge")
        .join(name)
}

#[test]
fn phase_2_project_fixture_loads_multiple_kinds_and_preserves_extensions() {
    let boundary = discover(&fixture("valid/docs")).unwrap();
    let project = load(&boundary).unwrap();
    assert_eq!(project.mounts.len(), 7);
    assert_eq!(project.extensions["owner"].as_str(), Some("fixture-team"));
    assert_eq!(
        project.mounts[0].extensions["label"].as_str(),
        Some("Project guide")
    );

    let temp = tempfile::tempdir().unwrap();
    copy_fixture(&fixture("valid"), temp.path());
    let copied = discover(&temp.path().join("docs")).unwrap();
    let mut mutated = project.clone();
    let disabled = MountId::new("disabled").unwrap();
    mutated.set_mount_enabled(&disabled, true).unwrap();
    mutated.reorder_mount(&disabled, 0).unwrap();
    mutated
        .remove_mount(&MountId::new("remote").unwrap())
        .unwrap();
    mutated
        .add_mount(Mount::new(
            MountId::new("replacement").unwrap(),
            SourceId::new("source.replacement").unwrap(),
            "https://example.invalid/replacement",
            MountKind::Opaque,
            99,
        ))
        .unwrap();
    save(&copied, &mutated).unwrap();
    let round_trip = load(&copied).unwrap();
    assert_eq!(round_trip, mutated);
    assert_eq!(round_trip.extensions, project.extensions);
}

#[test]
fn phase_2_observations_preserve_identity_and_distinct_states() {
    let boundary = discover(&fixture("valid")).unwrap();
    let project = load(&boundary).unwrap();
    let observations = project
        .mounts
        .iter()
        .map(|mount| {
            (
                mount.id.as_str(),
                observe_mount(&boundary.root, mount).unwrap(),
            )
        })
        .collect::<Vec<_>>();

    let one = current_source(&observations[2].1);
    let two = current_source(&observations[3].1);
    assert_ne!(one.id, two.id);
    assert_eq!(
        one.revision.as_ref().unwrap().content_digest,
        two.revision.as_ref().unwrap().content_digest
    );
    assert_ne!(
        one.revision.as_ref().unwrap().id,
        two.revision.as_ref().unwrap().id
    );
    assert!(matches!(
        observations[4].1,
        SourceObservation::Unavailable { .. }
    ));
    assert!(matches!(
        observations[5].1,
        SourceObservation::Disabled { .. }
    ));
    assert!(matches!(
        observations[6].1,
        SourceObservation::Unsupported { .. }
    ));
}

#[test]
fn phase_2_fixture_errors_have_stable_diagnostic_codes() {
    let invalid = discover(&fixture("invalid-id")).unwrap();
    assert_eq!(
        load(&invalid).unwrap_err().diagnostic().code,
        "project.declaration-invalid"
    );

    let conflict = discover(&fixture("conflicting-locators")).unwrap();
    assert_eq!(
        load(&conflict).unwrap_err().diagnostic().code,
        "project.mount-locator-conflict"
    );

    let mismatch = discover(&fixture("digest-mismatch")).unwrap();
    let project = load(&mismatch).unwrap();
    let error = observe_mount(&mismatch.root, &project.mounts[0]).unwrap_err();
    assert_eq!(
        error
            .diagnostic(Some(project.mounts[0].source_id.clone()))
            .code,
        "source.digest-mismatch"
    );
}

#[test]
fn phase_2_invalid_region_fixture_is_rejected() {
    #[derive(Deserialize)]
    struct Regions {
        regions: Vec<Region>,
    }
    #[derive(Deserialize)]
    struct Region {
        name: String,
        start: usize,
        end: usize,
    }

    let input = fs::read_to_string(fixture("invalid-regions/regions.toml")).unwrap();
    let fixture: Regions = toml::from_str(&input).unwrap();
    let revision = kero_core::knowledge::SourceRevision::from_bytes(
        kero_core::knowledge::SourceId::new("source.text").unwrap(),
        "text/plain",
        "aé\nz".as_bytes(),
        Default::default(),
    );
    for invalid in fixture.regions {
        let error = SourceRegion::utf8(
            &revision,
            "aé\nz",
            invalid.start,
            invalid.end,
            Some(invalid.name),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            SourceError::RegionBounds { .. } | SourceError::RegionUtf8Boundary { .. }
        ));
    }
}

fn current_source(observation: &SourceObservation) -> &kero_core::knowledge::Source {
    match observation {
        SourceObservation::Current { source } => {
            assert_eq!(source.availability, Availability::Available);
            source
        }
        other => panic!("expected current source, got {other:?}"),
    }
}

fn copy_fixture(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_fixture(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn phase_2_mount_reorder_does_not_change_source_identity() {
    let boundary = discover(&fixture("valid")).unwrap();
    let mut project = load(&boundary).unwrap();
    let guide = MountId::new("guide").unwrap();
    let before = project
        .mounts
        .iter()
        .find(|mount| mount.id == guide)
        .unwrap()
        .source_id
        .clone();
    project.reorder_mount(&guide, 3).unwrap();
    let after = project
        .mounts
        .iter()
        .find(|mount| mount.id == guide)
        .unwrap()
        .source_id
        .clone();
    assert_eq!(before, after);
    assert_eq!(project.mounts[3].display_order, 3);
}

#[test]
fn phase_2_file_move_preserves_identity_but_byte_change_revises_it() {
    let temp = tempfile::tempdir().unwrap();
    copy_fixture(&fixture("valid"), temp.path());
    let boundary = discover(temp.path()).unwrap();
    let mut project = load(&boundary).unwrap();
    let guide = project
        .mounts
        .iter_mut()
        .find(|mount| mount.id == MountId::new("guide").unwrap())
        .unwrap();

    let before = current_source(&observe_mount(&boundary.root, guide).unwrap()).clone();
    fs::rename(
        boundary.root.join("docs/guide.md"),
        boundary.root.join("docs/moved-guide.md"),
    )
    .unwrap();
    guide.source = "docs/moved-guide.md".into();
    let moved = current_source(&observe_mount(&boundary.root, guide).unwrap()).clone();
    assert_eq!(before.id, moved.id);
    assert_eq!(
        before.revision.as_ref().unwrap().id,
        moved.revision.as_ref().unwrap().id
    );

    fs::write(
        boundary.root.join("docs/moved-guide.md"),
        "changed knowledge",
    )
    .unwrap();
    let changed = current_source(&observe_mount(&boundary.root, guide).unwrap()).clone();
    assert_eq!(moved.id, changed.id);
    assert_ne!(moved.revision.unwrap().id, changed.revision.unwrap().id);
}

#[test]
fn phase_2_project_errors_are_typed() {
    let missing = fixture("does-not-exist");
    assert!(matches!(discover(&missing), Err(ProjectError::Io(_))));
}
